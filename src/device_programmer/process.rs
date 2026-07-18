use crate::device_programmer::{CREATE_NO_WINDOW, CompletionStatus, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use crate::utils::process_job::{CREATE_SUSPENDED, ProcessJob};
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type LineCallback = Option<Box<dyn Fn(&str) + Send + Sync + 'static>>;
type CompletionCallback = Option<Box<dyn FnOnce(bool) + Send + 'static>>;
pub(crate) type ProcessTerminator = Arc<dyn Fn() -> Result<(), String> + Send + Sync + 'static>;
const READER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const PROCESS_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);
const PROCESS_TERMINATION_POLL_INTERVAL: Duration = Duration::from_millis(25);

enum ProcessWaitOutcome {
    Exited(ExitStatus),
    Failed(String),
}

#[derive(Default)]
struct OperationState {
    generation: u64,
    active: bool,
    restart_blocked: Option<String>,
}

struct ReaderThreads {
    handles: Vec<JoinHandle<()>>,
    completion_rx: Receiver<Result<(), String>>,
    expected_completions: usize,
    callbacks_enabled: Arc<AtomicBool>,
}

#[derive(Debug, PartialEq, Eq)]
enum ReaderCleanupError {
    PossiblePipeHolder(String),
    Bookkeeping(String),
}

impl ReaderCleanupError {
    fn blocks_restart(&self) -> bool {
        matches!(self, Self::PossiblePipeHolder(_))
    }

    fn into_message(self) -> String {
        match self {
            Self::PossiblePipeHolder(message) | Self::Bookkeeping(message) => message,
        }
    }
}

impl ReaderThreads {
    fn finish(self) -> Result<(), ReaderCleanupError> {
        self.finish_with_timeout(READER_DRAIN_TIMEOUT)
    }

    fn finish_with_timeout(self, drain_timeout: Duration) -> Result<(), ReaderCleanupError> {
        let deadline = Instant::now() + drain_timeout;
        let mut reader_error = None;

        for _ in 0..self.expected_completions {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.completion_rx.recv_timeout(remaining) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    reader_error.get_or_insert(error);
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.callbacks_enabled.store(false, Ordering::SeqCst);
                    return Err(ReaderCleanupError::PossiblePipeHolder(format!(
                        "Process output streams did not close within {}ms",
                        drain_timeout.as_millis()
                    )));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.callbacks_enabled.store(false, Ordering::SeqCst);
                    return Err(ReaderCleanupError::Bookkeeping(
                        "Process output reader stopped unexpectedly".to_string(),
                    ));
                }
            }
        }

        self.callbacks_enabled.store(false, Ordering::SeqCst);
        for handle in self.handles {
            if handle.join().is_err() {
                return Err(ReaderCleanupError::Bookkeeping(
                    "Process output reader thread panicked".to_string(),
                ));
            }
        }

        reader_error.map_or(Ok(()), |error| Err(ReaderCleanupError::Bookkeeping(error)))
    }
}

pub struct ProcessExecutor {
    logger: Logger,
    completion_status: Arc<Mutex<CompletionStatus>>,
    start_time: Arc<Mutex<Option<Instant>>>,
    process_job: Arc<Mutex<Result<Arc<ProcessJob>, String>>>,
    operation_state: Arc<Mutex<OperationState>>,
}

pub struct CommandOptions {
    pub log_duration: bool,
    pub cleanup_temp_files: bool,
    pub duration_target: Option<Arc<Mutex<Option<Duration>>>>,
    pub on_complete: CompletionCallback,
}

impl ProcessExecutor {
    pub fn new(logger: Logger) -> Self {
        let process_job = ProcessJob::new_kill_on_close().map(Arc::new);
        if let Err(error) = &process_job {
            logger.error(error);
        }

        Self {
            logger,
            completion_status: Arc::new(Mutex::new(CompletionStatus::NotCompleted)),
            start_time: Arc::new(Mutex::new(None)),
            process_job: Arc::new(Mutex::new(process_job)),
            operation_state: Arc::new(Mutex::new(OperationState::default())),
        }
    }

    pub fn reset(&self) -> Result<(), String> {
        // Serializing reset with spawn/assignment and final publication prevents
        // an old worker from deleting input or overwriting a replacement
        // operation's status. Holding this guard while retiring the old Job also
        // prevents a new child from being assigned between the emptiness check
        // and Job rotation.
        let mut operation_state = self.operation_state.lock().unwrap();
        self.retire_for_restart_locked(&mut operation_state)
            .map_err(|error| format!("Cannot reset active process ownership: {error}"))?;

        let process_job = ProcessJob::new_kill_on_close().map(Arc::new);
        if let Err(error) = &process_job {
            self.logger.error(error);
        }
        *self.process_job.lock().unwrap() = process_job;
        operation_state.active = false;
        operation_state.restart_blocked = None;
        *self.completion_status.lock().unwrap() = CompletionStatus::NotCompleted;
        *self.start_time.lock().unwrap() = Some(Instant::now());

        match &*self.process_job.lock().unwrap() {
            Ok(_) => Ok(()),
            Err(error) => Err(error.clone()),
        }
    }

    #[cfg(test)]
    pub fn get_completion_status(&self) -> CompletionStatus {
        self.completion_status.lock().unwrap().clone()
    }

    pub(crate) fn completion_snapshot(&self) -> (CompletionStatus, bool) {
        // Keep the same lock order used by reset, launch, and final publication.
        // A terminal status can therefore never be observed with stale restart
        // safety from before its process-tree cleanup completed.
        let operation_state = self.operation_state.lock().unwrap();
        let status = self.completion_status.lock().unwrap().clone();
        (status, operation_state.restart_blocked.is_none())
    }

    pub(crate) fn retire_for_restart(&self) -> Result<(), String> {
        let mut operation_state = self.operation_state.lock().unwrap();
        self.retire_for_restart_locked(&mut operation_state)
    }

    fn retire_for_restart_locked(
        &self,
        operation_state: &mut OperationState,
    ) -> Result<(), String> {
        if let Some(reason) = &operation_state.restart_blocked {
            return Err(format!(
                "Restart is blocked because prior process cleanup was not confirmed: {reason}"
            ));
        }

        let process_job = self.process_job.lock().unwrap().as_ref().ok().cloned();
        if let Some(process_job) = process_job
            && let Err(error) = process_job.terminate_remaining_and_wait(
                PROCESS_TERMINATION_TIMEOUT,
                PROCESS_TERMINATION_POLL_INTERVAL,
            )
        {
            let reason = format!("Failed to confirm owned process-tree retirement: {error}");
            operation_state.restart_blocked = Some(reason.clone());
            return Err(reason);
        }

        // Invalidate any worker that was still draining output after the Job was
        // confirmed empty. It must not publish into a replacement operation.
        operation_state.generation = operation_state.generation.wrapping_add(1);
        operation_state.active = false;
        Ok(())
    }

    pub fn prepare_command(exe_path: &str, args: &[&str]) -> Command {
        let mut command = Command::new(exe_path);
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        command
    }

    pub fn execute_command(
        &self,
        command: Command,
        on_line_callback: LineCallback,
        options: CommandOptions,
    ) -> Result<(), String> {
        self.execute_command_inner(command, on_line_callback, options)
    }

    fn execute_command_inner(
        &self,
        mut command: Command,
        on_line_callback: LineCallback,
        options: CommandOptions,
    ) -> Result<(), String> {
        // Enforce suspended creation even for commands that did not originate
        // from `prepare_command`; otherwise they could execute before Job Object
        // assignment completes.
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);

        let mut operation_state = self.operation_state.lock().unwrap();
        if operation_state.active {
            return Err("Cannot start a second process while an operation is active".to_string());
        }
        if let Some(reason) = &operation_state.restart_blocked {
            return Err(format!(
                "Cannot start a process because prior cleanup was not confirmed: {reason}"
            ));
        }
        let operation_generation = operation_state.generation;

        let process_job = match &*self.process_job.lock().unwrap() {
            Ok(process_job) => Arc::clone(process_job),
            Err(error) => {
                let error_msg = format!("Cannot start an unowned process: {error}");
                Self::cleanup_temp_firmware_if_requested(options.cleanup_temp_files, &self.logger);
                self.logger.error(&error_msg);
                *self.completion_status.lock().unwrap() =
                    CompletionStatus::Failed(error_msg.clone());
                return Err(error_msg);
            }
        };
        operation_state.active = true;

        match command.spawn() {
            Ok(mut child) => {
                if let Err(ownership_error) = process_job.assign_and_resume(&child) {
                    let direct_cleanup_result = Self::terminate_child_bounded(
                        &mut child,
                        PROCESS_TERMINATION_TIMEOUT,
                        PROCESS_TERMINATION_POLL_INTERVAL,
                    );
                    let job_cleanup_result = process_job.terminate_remaining_and_wait(
                        PROCESS_TERMINATION_TIMEOUT,
                        PROCESS_TERMINATION_POLL_INTERVAL,
                    );
                    let error_msg = format!(
                        "Failed to take ownership of spawned process: {ownership_error}; \
                         direct cleanup result: {direct_cleanup_result:?}; \
                         job cleanup result: {job_cleanup_result:?}"
                    );
                    if direct_cleanup_result.is_err() || job_cleanup_result.is_err() {
                        operation_state.restart_blocked = Some(error_msg.clone());
                    }
                    Self::cleanup_temp_firmware_if_requested(
                        options.cleanup_temp_files,
                        &self.logger,
                    );
                    self.logger.error(&error_msg);
                    *self.completion_status.lock().unwrap() =
                        CompletionStatus::Failed(error_msg.clone());
                    operation_state.active = false;
                    return Err(error_msg);
                }

                let reader_threads = self.attach_readers(&mut child, on_line_callback);

                // Wait in a separate thread for the process to complete
                let logger = self.logger.clone();
                let completion_status = Arc::clone(&self.completion_status);
                let start_time = Arc::clone(&self.start_time);
                let process_job = Arc::downgrade(&process_job);
                let worker_operation_state = Arc::clone(&self.operation_state);

                // From this point the child is owned and the worker carries the
                // generation. A reset may now retire it, but cannot allow its
                // final side effects into the replacement operation.
                drop(operation_state);

                thread::spawn(move || {
                    let mut options = options;
                    let mut wait_result = Self::wait_for_process(&mut child);
                    let mut restart_blocked = None;
                    let elapsed = start_time.lock().unwrap().map(|start| start.elapsed());

                    // Waiting for the direct child is not sufficient: a helper
                    // process can outlive it and continue touching hardware or
                    // hold inherited output pipes open. Retire every remaining
                    // member before exposing a terminal status.
                    if let Some(process_job) = process_job.upgrade()
                        && let Err(job_error) = process_job.terminate_remaining_and_wait(
                            PROCESS_TERMINATION_TIMEOUT,
                            PROCESS_TERMINATION_POLL_INTERVAL,
                        )
                    {
                        let error = format!("Failed to retire owned process tree: {job_error}");
                        logger.error(&error);
                        restart_blocked = Some(error.clone());
                        wait_result = ProcessWaitOutcome::Failed(error);
                    }

                    // A process can exit before its pipe readers have consumed the final
                    // buffered lines. Keep the public status non-terminal until both
                    // streams are fully drained so snapshots cannot assess partial output.
                    // If a descendant inherited a pipe, stop accepting callbacks after a
                    // bounded grace period instead of blocking this operation forever.
                    if let Err(reader_error) = reader_threads.finish() {
                        let blocks_restart = reader_error.blocks_restart();
                        let reader_error = reader_error.into_message();
                        logger.error(&reader_error);
                        if blocks_restart && restart_blocked.is_none() {
                            restart_blocked = Some(reader_error.clone());
                        }
                        wait_result = match wait_result {
                            ProcessWaitOutcome::Exited(_) => {
                                ProcessWaitOutcome::Failed(reader_error)
                            }
                            ProcessWaitOutcome::Failed(error) => {
                                ProcessWaitOutcome::Failed(format!("{error}; {reader_error}"))
                            }
                        };
                    }

                    let mut operation_state = worker_operation_state.lock().unwrap();
                    if operation_state.generation != operation_generation {
                        logger.debug("Discarding stale process finalization after operation reset");
                        return;
                    }

                    // Publish restart safety before any terminal status. Result
                    // actions must never race ahead of an unconfirmed cleanup.
                    if let Some(reason) = restart_blocked {
                        operation_state.restart_blocked = Some(reason);
                    }

                    // Completion is the handoff that permits another firmware operation.
                    // Remove the shared input before publishing any terminal status so an
                    // older worker can never delete a replacement operation's input file.
                    Self::cleanup_temp_firmware_if_requested(options.cleanup_temp_files, &logger);

                    match wait_result {
                        ProcessWaitOutcome::Exited(exit_status) => {
                            if let Some(elapsed) = elapsed {
                                if let Some(duration_target) = &options.duration_target {
                                    *duration_target.lock().unwrap() = Some(elapsed);
                                }

                                if options.log_duration {
                                    // Format duration in a readable way based on the actual time
                                    if elapsed.as_secs() > 0 {
                                        // If operation took more than a second, show seconds.milliseconds
                                        let seconds = elapsed.as_secs();
                                        let millis = elapsed.subsec_millis();
                                        logger
                                            .info(format!("Operation took {seconds}.{millis:03}s"));
                                    } else {
                                        // For very quick operations, show milliseconds
                                        logger.info(format!(
                                            "Operation took {}ms",
                                            elapsed.as_millis()
                                        ));
                                    }
                                }
                            }

                            let command_succeeded = exit_status.success();

                            if command_succeeded {
                                logger.success("Command completed successfully");

                                if let Some(on_complete) = options.on_complete.take() {
                                    on_complete(true);
                                }

                                // Guard: don't overwrite a more specific terminal status
                                // (e.g. DnaReadCompleted set by a completion callback)
                                let mut status = completion_status.lock().unwrap();
                                if !matches!(
                                    *status,
                                    CompletionStatus::DnaReadCompleted(_)
                                        | CompletionStatus::Failed(_)
                                ) {
                                    *status = CompletionStatus::Completed;
                                }
                            } else {
                                let error_msg = format!(
                                    "Command failed with exit code: {:?}",
                                    exit_status.code()
                                );
                                logger.error(&error_msg);

                                if let Some(on_complete) = options.on_complete.take() {
                                    on_complete(false);
                                }

                                Self::set_failure_unless_callback_did(
                                    &completion_status,
                                    error_msg,
                                );
                            }
                        }
                        ProcessWaitOutcome::Failed(error_msg) => {
                            if let Some(on_complete) = options.on_complete.take() {
                                on_complete(false);
                            }

                            logger.error(&error_msg);
                            Self::set_failure_unless_callback_did(&completion_status, error_msg);
                        }
                    }

                    // This is deliberately the final operation-side effect while
                    // holding the same lock used by reset and launch.
                    operation_state.active = false;
                });

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to start process: {e}");
                Self::cleanup_temp_firmware_if_requested(options.cleanup_temp_files, &self.logger);
                self.logger.error(&error_msg);
                *self.completion_status.lock().unwrap() =
                    CompletionStatus::Failed(error_msg.clone());
                operation_state.active = false;

                Err(error_msg)
            }
        }
    }

    fn wait_for_process(child: &mut Child) -> ProcessWaitOutcome {
        match child.wait() {
            Ok(status) => ProcessWaitOutcome::Exited(status),
            Err(error) => {
                ProcessWaitOutcome::Failed(format!("Failed to wait for process: {error}"))
            }
        }
    }

    fn terminate_child_bounded(
        child: &mut Child,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<(), String> {
        let initial_inspection_error = match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => None,
            // Still attempt termination. An inspection failure must not turn
            // into an early return that leaves a suspended process behind.
            Err(error) => Some(error),
        };

        let kill_error = child.kill().err();
        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => {}
                Err(error) => {
                    let initial_context = initial_inspection_error
                        .as_ref()
                        .map_or_else(String::new, |initial| {
                            format!("; initial inspection also failed: {initial}")
                        });
                    let kill_context = kill_error.as_ref().map_or_else(String::new, |kill| {
                        format!("; termination request failed: {kill}")
                    });
                    return Err(format!(
                        "Failed to confirm spawned process exit: {error}{initial_context}{kill_context}"
                    ));
                }
            }

            if Instant::now() >= deadline {
                let kill_context = kill_error.map_or_else(
                    || "termination was requested".to_string(),
                    |error| format!("termination request failed: {error}"),
                );
                let initial_context = initial_inspection_error.map_or_else(String::new, |error| {
                    format!("; initial inspection failed: {error}")
                });
                return Err(format!(
                    "Spawned process did not exit within {} seconds ({kill_context}{initial_context})",
                    timeout.as_secs_f32(),
                ));
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            thread::sleep(poll_interval.max(Duration::from_millis(1)).min(remaining));
        }
    }

    fn set_failure_unless_callback_did(
        completion_status: &Arc<Mutex<CompletionStatus>>,
        error_message: String,
    ) {
        let mut status = completion_status.lock().unwrap();
        if !matches!(
            *status,
            CompletionStatus::DnaReadCompleted(_) | CompletionStatus::Failed(_)
        ) {
            *status = CompletionStatus::Failed(error_message);
        }
    }

    fn cleanup_temp_firmware_if_requested(cleanup_requested: bool, logger: &Logger) {
        if cleanup_requested
            && let Err(error) = fs::remove_file(TEMP_FIRMWARE_FILE)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            logger.warning(format!(
                "Failed to clean up temporary firmware file: {error}"
            ));
        }
    }

    pub(crate) fn process_terminator(&self) -> ProcessTerminator {
        match &*self.process_job.lock().unwrap() {
            Ok(process_job) => {
                // Bind this callback to the operation that created it. Looking
                // up a mutable slot at invocation time would let a stale monitor
                // terminate a replacement operation after reset.
                let process_job = Arc::downgrade(process_job);
                Arc::new(move || {
                    let Some(process_job) = process_job.upgrade() else {
                        // Dropping the final strong handle already triggered
                        // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.
                        return Ok(());
                    };

                    process_job.terminate_and_wait(
                        PROCESS_TERMINATION_TIMEOUT,
                        PROCESS_TERMINATION_POLL_INTERVAL,
                    )
                })
            }
            Err(error) => {
                let error = error.clone();
                Arc::new(move || Err(error.clone()))
            }
        }
    }

    fn attach_readers(&self, child: &mut Child, line_callback: LineCallback) -> ReaderThreads {
        // Wrap the callback in an Arc for sharing between threads
        let callback_arc = Arc::new(line_callback);
        let callbacks_enabled = Arc::new(AtomicBool::new(true));
        let (completion_tx, completion_rx) = mpsc::channel();
        let mut reader_threads = Vec::with_capacity(2);
        let mut expected_completions = 0;

        // For stdout
        if let Some(stdout) = child.stdout.take() {
            let stdout_logger = self.logger.clone();
            let callback_opt = Arc::clone(&callback_arc);
            let callbacks_enabled = Arc::clone(&callbacks_enabled);
            let completion_tx = completion_tx.clone();
            expected_completions += 1;

            reader_threads.push(thread::spawn(move || {
                let reader = BufReader::new(stdout);
                let result = Self::read_output_lines(reader, "stdout", |line| {
                    stdout_logger.output(line);

                    // Forward every line to the callback
                    if callbacks_enabled.load(Ordering::SeqCst)
                        && let Some(cb) = &*callback_opt
                    {
                        cb(line);
                    }
                });

                stdout_logger.debug("Stdout processor thread completed");
                let _ = completion_tx.send(result);
            }));
        }

        // For stderr
        if let Some(stderr) = child.stderr.take() {
            let stderr_logger = self.logger.clone();

            // Clone the Arc, not the inner callback
            let callback_opt_for_stderr = Arc::clone(&callback_arc);
            let callbacks_enabled = Arc::clone(&callbacks_enabled);
            let completion_tx = completion_tx.clone();
            expected_completions += 1;

            reader_threads.push(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                let result = Self::read_output_lines(reader, "stderr", |line| {
                    stderr_logger.error(line);

                    // Forward all stderr output so domain parsers can observe both
                    // progress stages and sector-write events. OpenOCD commonly
                    // writes informational messages to stderr.
                    if callbacks_enabled.load(Ordering::SeqCst)
                        && let Some(callback) = &*callback_opt_for_stderr
                    {
                        callback(line);
                    }
                });
                stderr_logger.debug("Stderr processor thread completed");
                let _ = completion_tx.send(result);
            }));
        }

        drop(completion_tx);
        ReaderThreads {
            handles: reader_threads,
            completion_rx,
            expected_completions,
            callbacks_enabled,
        }
    }

    fn read_output_lines<R, F>(reader: R, stream_name: &str, mut on_line: F) -> Result<(), String>
    where
        R: BufRead,
        F: FnMut(&str),
    {
        for line_result in reader.lines() {
            let line = line_result
                .map_err(|error| format!("Failed to read process {stream_name}: {error}"))?;
            on_line(&line);
        }

        Ok(())
    }

    pub fn set_completion_status(&self, status: CompletionStatus) {
        *self.completion_status.lock().unwrap() = status;
    }

    pub fn get_completion_status_arc(&self) -> Arc<Mutex<CompletionStatus>> {
        Arc::clone(&self.completion_status)
    }

    #[cfg(test)]
    pub(crate) fn block_restart_for_test(&self, reason: &str) {
        self.operation_state.lock().unwrap().restart_blocked = Some(reason.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_programmer::DnaInfo;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const TREE_STARTED_MARKER_ENV: &str = "DMA_TOOLS_TREE_STARTED_MARKER";
    const TREE_COMPLETED_MARKER_ENV: &str = "DMA_TOOLS_TREE_COMPLETED_MARKER";

    fn unique_marker_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dma-tools-process-test-{}-{label}-{nonce}",
            std::process::id()
        ))
    }

    fn wait_for_terminal_status(executor: &ProcessExecutor) -> CompletionStatus {
        wait_for_terminal_status_arc(&executor.get_completion_status_arc())
    }

    fn wait_for_terminal_status_arc(
        completion_status: &Arc<Mutex<CompletionStatus>>,
    ) -> CompletionStatus {
        let deadline = Instant::now() + Duration::from_secs(5);

        loop {
            let status = completion_status.lock().unwrap().clone();
            if matches!(
                status,
                CompletionStatus::Completed
                    | CompletionStatus::DnaReadCompleted(_)
                    | CompletionStatus::Failed(_)
            ) {
                return status;
            }

            assert!(
                Instant::now() < deadline,
                "Timed out waiting for terminal process status, last status was {status:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn invalid_utf8_in_process_output_is_reported() {
        let result =
            ProcessExecutor::read_output_lines(Cursor::new(vec![0xff, b'\n']), "stdout", |_| {
                panic!("invalid output must not reach the line callback")
            });

        let error = result.expect_err("invalid UTF-8 must fail the output reader");
        assert!(error.contains("stdout"));
        assert!(error.contains("stream did not contain valid UTF-8"));
    }

    #[test]
    fn reader_completion_propagates_stream_errors() {
        let (completion_tx, completion_rx) = mpsc::channel();
        completion_tx
            .send(Err("synthetic stdout failure".to_string()))
            .unwrap();
        drop(completion_tx);

        let error = ReaderThreads {
            handles: Vec::new(),
            completion_rx,
            expected_completions: 1,
            callbacks_enabled: Arc::new(AtomicBool::new(true)),
        }
        .finish()
        .expect_err("a stream read error must fail reader cleanup");

        assert!(!error.blocks_restart());
        assert_eq!(error.into_message(), "synthetic stdout failure");
    }

    #[test]
    fn reader_drain_timeout_blocks_restart() {
        let (_completion_tx, completion_rx) = mpsc::channel::<Result<(), String>>();

        let error = ReaderThreads {
            handles: Vec::new(),
            completion_rx,
            expected_completions: 1,
            callbacks_enabled: Arc::new(AtomicBool::new(true)),
        }
        .finish_with_timeout(Duration::ZERO)
        .expect_err("an open output stream must time out");

        assert!(error.blocks_restart());
        assert!(error.into_message().contains("did not close"));
    }

    #[test]
    fn cleanup_blocker_is_published_with_terminal_status_and_prevents_restart() {
        let executor = ProcessExecutor::new(Logger::new("ProcessRestartSafetyTest"));
        executor.reset().unwrap();
        let reason = "synthetic unconfirmed process cleanup";

        {
            // Match the production publication order: safety first, then status,
            // while holding the operation-state lock across both writes.
            let mut operation_state = executor.operation_state.lock().unwrap();
            operation_state.restart_blocked = Some(reason.to_string());
            *executor.completion_status.lock().unwrap() =
                CompletionStatus::Failed(reason.to_string());
        }

        let (status, safe_to_restart) = executor.completion_snapshot();
        assert_eq!(status, CompletionStatus::Failed(reason.to_string()));
        assert!(!safe_to_restart);

        let error = executor
            .retire_for_restart()
            .expect_err("an unconfirmed cleanup must permanently block replacement");
        assert!(error.contains(reason));
    }

    #[test]
    fn completion_callback_status_is_not_overwritten_on_success() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset().unwrap();

        let completion_status = executor.get_completion_status_arc();
        let command = ProcessExecutor::prepare_command("cmd", &["/C", "exit /B 0"]);

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: Some(Box::new(move |command_succeeded| {
                        assert!(command_succeeded);
                        *completion_status.lock().unwrap() =
                            CompletionStatus::DnaReadCompleted(DnaInfo {
                                dna_value: "0x1".to_string(),
                                dna_raw_value: "1".to_string(),
                                device_type: "test".to_string(),
                            });
                    })),
                },
            )
            .expect("test command should start");

        match wait_for_terminal_status(&executor) {
            CompletionStatus::DnaReadCompleted(info) => {
                assert_eq!(info.dna_value, "0x1");
            }
            status => panic!("expected DNA completion from callback, got {status:?}"),
        }
    }

    #[test]
    fn completion_callback_receives_failure_status() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset().unwrap();

        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = Arc::clone(&callback_called);
        let command = ProcessExecutor::prepare_command("cmd", &["/C", "exit /B 7"]);

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: Some(Box::new(move |command_succeeded| {
                        assert!(!command_succeeded);
                        callback_called_clone.store(true, Ordering::SeqCst);
                    })),
                },
            )
            .expect("test command should start");

        match wait_for_terminal_status(&executor) {
            CompletionStatus::Failed(error) => {
                assert!(error.contains("exit code"));
                assert!(callback_called.load(Ordering::SeqCst));
            }
            status => panic!("expected failed completion, got {status:?}"),
        }
    }

    #[test]
    fn terminal_status_waits_until_output_callbacks_are_drained() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset().unwrap();

        let callback_finished = Arc::new(AtomicBool::new(false));
        let callback_finished_clone = Arc::clone(&callback_finished);
        let command = ProcessExecutor::prepare_command("cmd", &["/C", "echo final-line"]);

        executor
            .execute_command(
                command,
                Some(Box::new(move |line| {
                    if line.contains("final-line") {
                        thread::sleep(Duration::from_millis(100));
                        callback_finished_clone.store(true, Ordering::SeqCst);
                    }
                })),
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: None,
                },
            )
            .expect("test command should start");

        assert_eq!(
            wait_for_terminal_status(&executor),
            CompletionStatus::Completed
        );
        assert!(
            callback_finished.load(Ordering::SeqCst),
            "terminal status became visible before the final output callback completed"
        );
    }

    #[test]
    fn nonzero_exit_preserves_callback_failure() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset().unwrap();

        let completion_status = executor.get_completion_status_arc();
        let command = ProcessExecutor::prepare_command("cmd", &["/C", "exit /B 9"]);

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: Some(Box::new(move |command_succeeded| {
                        assert!(!command_succeeded);
                        *completion_status.lock().unwrap() =
                            CompletionStatus::Failed("localized command failure".to_string());
                    })),
                },
            )
            .expect("test command should start");

        assert_eq!(
            wait_for_terminal_status(&executor),
            CompletionStatus::Failed("localized command failure".to_string())
        );
    }

    #[test]
    fn dropping_executor_terminates_its_owned_process() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset().unwrap();
        let completion_status = executor.get_completion_status_arc();
        let command = ProcessExecutor::prepare_command(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ],
        );

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: None,
                },
            )
            .expect("long-running test command should start");

        let dropped_at = Instant::now();
        drop(executor);

        let _terminal_status = wait_for_terminal_status_arc(&completion_status);
        assert!(
            dropped_at.elapsed() < Duration::from_secs(5),
            "dropping the executor must not leave its 30-second child running"
        );
    }

    #[test]
    fn stale_terminator_cannot_stop_a_replacement_operation() {
        let executor = ProcessExecutor::new(Logger::new("ProcessGenerationTest"));
        executor.reset().unwrap();
        let stale_terminator = executor.process_terminator();

        executor.reset().unwrap();
        let command = ProcessExecutor::prepare_command(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Milliseconds 150",
            ],
        );
        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: None,
                },
            )
            .expect("replacement command should start");

        stale_terminator().expect("retired job should already be safely closed");
        assert_eq!(
            wait_for_terminal_status(&executor),
            CompletionStatus::Completed,
            "a callback from the previous generation must not terminate the replacement"
        );
    }

    #[test]
    fn reset_retires_an_active_generation_before_starting_the_next() {
        let executor = ProcessExecutor::new(Logger::new("ProcessGenerationTest"));
        executor.reset().unwrap();
        let stale_callback_called = Arc::new(AtomicBool::new(false));
        let callback_flag = Arc::clone(&stale_callback_called);
        let long_command = ProcessExecutor::prepare_command(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ],
        );
        executor
            .execute_command(
                long_command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: Some(Box::new(move |_| {
                        callback_flag.store(true, Ordering::SeqCst);
                    })),
                },
            )
            .expect("old generation should start");

        let reset_at = Instant::now();
        executor
            .reset()
            .expect("reset should retire the active owned process tree");
        assert!(
            reset_at.elapsed() < Duration::from_secs(3),
            "reset must not wait for the command's 30-second natural exit"
        );

        let replacement = ProcessExecutor::prepare_command("cmd", &["/C", "exit /B 0"]);
        executor
            .execute_command(
                replacement,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: None,
                },
            )
            .expect("replacement generation should start");

        assert_eq!(
            wait_for_terminal_status(&executor),
            CompletionStatus::Completed
        );
        thread::sleep(Duration::from_millis(150));
        assert_eq!(
            executor.get_completion_status(),
            CompletionStatus::Completed
        );
        assert!(
            !stale_callback_called.load(Ordering::SeqCst),
            "a retired generation must not invoke completion callbacks"
        );
    }

    #[test]
    fn bounded_child_termination_does_not_wait_for_natural_exit() {
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("long-running test process should start");

        let started_at = Instant::now();
        ProcessExecutor::terminate_child_bounded(
            &mut child,
            Duration::from_secs(2),
            Duration::from_millis(10),
        )
        .expect("direct child termination should be confirmed");

        assert!(
            started_at.elapsed() < Duration::from_secs(3),
            "cleanup must not wait for the process's 30-second natural exit"
        );
    }

    #[test]
    fn terminal_status_retires_descendants_that_outlive_the_direct_child() {
        let started_marker = unique_marker_path("started");
        let completed_marker = unique_marker_path("completed");
        let current_exe = std::env::current_exe().expect("test executable path should be known");
        let current_exe = current_exe
            .to_str()
            .expect("test executable path should be valid UTF-8");

        let executor = ProcessExecutor::new(Logger::new("ProcessTreeTest"));
        executor.reset().unwrap();
        let mut command = ProcessExecutor::prepare_command(
            current_exe,
            &[
                "--ignored",
                "--exact",
                "device_programmer::process::tests::process_tree_parent_helper",
                "--nocapture",
                "--test-threads=1",
            ],
        );
        command.env(TREE_STARTED_MARKER_ENV, &started_marker);
        command.env(TREE_COMPLETED_MARKER_ENV, &completed_marker);

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: false,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: None,
                },
            )
            .expect("process-tree parent should start");

        assert_eq!(
            wait_for_terminal_status(&executor),
            CompletionStatus::Completed,
            "a successful direct child should remain successful after descendant cleanup"
        );
        assert!(
            started_marker.exists(),
            "the helper must prove its descendant started before exiting"
        );

        thread::sleep(Duration::from_millis(900));
        assert!(
            !completed_marker.exists(),
            "terminal status must not leave the descendant alive to finish later"
        );

        let _ = fs::remove_file(started_marker);
        let _ = fs::remove_file(completed_marker);
    }

    #[test]
    #[ignore = "helper process launched by terminal_status_retires_descendants_that_outlive_the_direct_child"]
    #[allow(clippy::zombie_processes)] // Deliberately leaves a Job-owned descendant for the parent test.
    fn process_tree_parent_helper() {
        let started_marker = std::env::var_os(TREE_STARTED_MARKER_ENV)
            .expect("parent helper requires a started marker path");
        let current_exe = std::env::current_exe().expect("test executable path should be known");
        let _descendant = Command::new(current_exe)
            .args([
                "--ignored",
                "--exact",
                "device_programmer::process::tests::process_tree_descendant_helper",
                "--nocapture",
                "--test-threads=1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("descendant helper should start");

        let deadline = Instant::now() + Duration::from_secs(2);
        while !std::path::Path::new(&started_marker).exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            std::path::Path::new(&started_marker).exists(),
            "descendant did not start within the helper deadline"
        );
    }

    #[test]
    #[ignore = "helper process launched by process_tree_parent_helper"]
    fn process_tree_descendant_helper() {
        let started_marker = std::env::var_os(TREE_STARTED_MARKER_ENV)
            .expect("descendant helper requires a started marker path");
        let completed_marker = std::env::var_os(TREE_COMPLETED_MARKER_ENV)
            .expect("descendant helper requires a completed marker path");

        fs::write(started_marker, b"started").expect("started marker should be writable");
        thread::sleep(Duration::from_millis(750));
        fs::write(completed_marker, b"completed").expect("completed marker should be writable");
    }
}
