use super::PcileechTestState;
use super::parser::{finalize_result, find_error_message};
use crate::device_programmer::CREATE_NO_WINDOW;
use crate::utils::process_job::{CREATE_SUSPENDED, ProcessJob};
use std::borrow::Cow;
use std::ffi::OsString;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError};
use std::thread;
use std::time::{Duration, Instant};

const PCILEECH_TOOL_PATH: &str = "tools\\memflow-base\\memflow-base.exe";
const PCILEECH_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PROCESS_TERMINATION_GRACE: Duration = Duration::from_secs(2);
const STREAM_DRAIN_GRACE: Duration = Duration::from_secs(1);
const OUTPUT_LIMIT_GRACE: Duration = Duration::from_secs(1);
const MAX_STDOUT_CAPTURE_BYTES: usize = 1024 * 1024;
const MAX_STDERR_CAPTURE_BYTES: usize = 1024 * 1024;
const MAX_EVENTS_PER_DRAIN: usize = 256;
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;
const OUTPUT_CHANNEL_CAPACITY: usize = 32;
const OUTPUT_SEND_RETRY: Duration = Duration::from_millis(1);

#[derive(Clone, Debug, Default)]
pub(super) struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub(super) struct RunOutcome {
    pub(super) state: PcileechTestState,
    pub(super) safe_to_restart: bool,
}

impl RunOutcome {
    pub(super) fn safe(state: PcileechTestState) -> Self {
        Self {
            state,
            safe_to_restart: true,
        }
    }

    pub(super) fn unsafe_to_restart(state: PcileechTestState) -> Self {
        Self {
            state,
            safe_to_restart: false,
        }
    }
}

#[derive(Debug, Clone)]
struct RunConfig {
    executable: PathBuf,
    args: Vec<OsString>,
    poll_interval: Duration,
    termination_grace: Duration,
    stream_drain_grace: Duration,
    output_limit_grace: Duration,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from(PCILEECH_TOOL_PATH),
            args: ["-c", "pcileech", "--headless"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            poll_interval: PCILEECH_POLL_INTERVAL,
            termination_grace: PROCESS_TERMINATION_GRACE,
            stream_drain_grace: STREAM_DRAIN_GRACE,
            output_limit_grace: OUTPUT_LIMIT_GRACE,
        }
    }
}

pub(super) fn run_pcileech_test(cancellation: &CancellationToken) -> RunOutcome {
    run_with_config(&RunConfig::default(), cancellation)
}

fn run_with_config(config: &RunConfig, cancellation: &CancellationToken) -> RunOutcome {
    if cancellation.is_cancelled() {
        return RunOutcome::safe(PcileechTestState::Failed(
            "PCILeech test was cancelled".to_string(),
        ));
    }

    let (mut child, process_job) = match spawn_pcileech_tool(config) {
        Ok(process) => process,
        Err(outcome) => return outcome,
    };

    let (stdout, stderr) = match take_process_streams(&mut child) {
        Ok(streams) => streams,
        Err(error) => {
            return finish_after_capture_failure(&mut child, &process_job, config, error);
        }
    };

    let (output_sender, output_receiver) = mpsc::sync_channel(OUTPUT_CHANNEL_CAPACITY);
    let reader_stop = Arc::new(AtomicBool::new(false));
    let reader_threads = [
        spawn_output_reader(
            stdout,
            OutputStream::Stdout,
            output_sender.clone(),
            Arc::clone(&reader_stop),
        ),
        spawn_output_reader(
            stderr,
            OutputStream::Stderr,
            output_sender,
            Arc::clone(&reader_stop),
        ),
    ];

    let mut output = CollectedOutput::default();
    let mut output_limit_reached_at = None;
    let stop_reason = loop {
        let _ = drain_available_output(&output_receiver, &mut output);

        // Capture failures are runner failures, not tool diagnostics. They must
        // never be hidden by a success signature in an earlier output chunk.
        if let Some(error) = output.read_error.clone() {
            break StopReason::Error(error);
        }

        if output.truncation_error.is_some() && output_limit_reached_at.is_none() {
            output_limit_reached_at = Some(Instant::now());
        }
        if output_limit_reached_at
            .is_some_and(|reached_at| reached_at.elapsed() >= config.output_limit_grace)
        {
            break StopReason::OutputLimit(
                output
                    .truncation_error
                    .clone()
                    .expect("a limit timestamp requires a truncation error"),
            );
        }

        if cancellation.is_cancelled() {
            break StopReason::Cancelled;
        }

        match child.try_wait() {
            Ok(Some(status)) => break StopReason::Exited(status),
            Ok(None) => {}
            Err(error) => {
                break StopReason::Error(format!("Failed to monitor test tool: {error}"));
            }
        }

        let mut wait_for = config.poll_interval;
        if let Some(reached_at) = output_limit_reached_at {
            wait_for = wait_for.min(
                config
                    .output_limit_grace
                    .saturating_sub(reached_at.elapsed()),
            );
        }
        match output_receiver.recv_timeout(wait_for) {
            Ok(event) => {
                let _ = output.record(event);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(wait_for),
        }
    };

    // Always terminate and confirm the Job, even if the root process already
    // exited. A root can leave descendants behind, and only the Job tracks
    // those descendants after the original PID disappears.
    let termination_error = terminate_process_tree_bounded(&mut child, &process_job, config).err();
    // Closing a kill-on-close Job is the final fail-safe if explicit
    // termination or accounting failed. The error above still marks the run
    // unsafe because this fallback cannot be synchronously confirmed.
    drop(process_job);

    let reader_error = finish_output_readers(
        &output_receiver,
        &mut output,
        reader_threads,
        &reader_stop,
        config,
    )
    .err();

    if let Some(cleanup_failure) = cleanup_failure_outcome(termination_error, reader_error) {
        return cleanup_failure;
    }

    let final_stdout = output.stdout_text();
    let final_diagnostics = output.diagnostic_text();
    let observed_error = find_error_message(&final_diagnostics);
    let truncation_error = output.truncation_error.clone();

    let state = match stop_reason {
        StopReason::Cancelled => {
            PcileechTestState::Failed("PCILeech test was cancelled".to_string())
        }
        StopReason::OutputLimit(error) => finalize_result(&final_stdout, None, Some(error)),
        StopReason::Error(error) => PcileechTestState::Failed(error),
        StopReason::Exited(status) => {
            let process_error = (!status.success())
                .then(|| format!("PCILeech test exited with code: {:?}", status.code()));
            finalize_result(
                &final_stdout,
                None,
                observed_error.or(process_error).or(truncation_error),
            )
        }
    };

    RunOutcome::safe(state)
}

fn finish_after_capture_failure(
    child: &mut Child,
    process_job: &ProcessJob,
    config: &RunConfig,
    capture_error: String,
) -> RunOutcome {
    match terminate_process_tree_bounded(child, process_job, config) {
        Ok(()) => RunOutcome::safe(PcileechTestState::Failed(capture_error)),
        Err(termination_error) => RunOutcome::unsafe_to_restart(PcileechTestState::Failed(
            format!("{capture_error}; {termination_error}"),
        )),
    }
}

fn spawn_pcileech_tool(config: &RunConfig) -> Result<(Child, ProcessJob), RunOutcome> {
    let process_job = ProcessJob::new_kill_on_close().map_err(|error| {
        RunOutcome::safe(PcileechTestState::Failed(format!(
            "Failed to create test process containment: {error}"
        )))
    })?;

    let executable = resolve_executable_path(&config.executable);
    let mut command = Command::new(&executable);
    command
        .args(&config.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    if let Some(runtime_directory) = executable
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        // memflow discovers dynamic connector/OS plugins in the process working
        // directory. Launch beside the bundled DLLs so a shortcut or shell with
        // a different working directory cannot make the inventory appear empty.
        command.current_dir(runtime_directory);
    }

    let mut child = command.spawn().map_err(|error| {
        RunOutcome::safe(PcileechTestState::Failed(format!(
            "Failed to start test tool: {error}"
        )))
    })?;

    if let Err(assign_error) = process_job.assign_and_resume(&child) {
        let direct_cleanup_error = terminate_direct_child_bounded(&mut child, config).err();
        let job_cleanup_error = process_job
            .terminate_remaining_and_wait(config.termination_grace, config.poll_interval)
            .map_err(|error| format!("Failed to retire test process job: {error}"))
            .err();
        let mut error = format!("Failed to contain test process tree: {assign_error}");
        if let Some(cleanup_error) = &direct_cleanup_error {
            error.push_str(&format!("; {cleanup_error}"));
        }
        if let Some(cleanup_error) = &job_cleanup_error {
            error.push_str(&format!("; {cleanup_error}"));
        }
        if direct_cleanup_error.is_some() || job_cleanup_error.is_some() {
            return Err(RunOutcome::unsafe_to_restart(PcileechTestState::Failed(
                error,
            )));
        }

        // Assignment happens before resume, so any descendant could only be in
        // the Job. Both the direct child and the Job were synchronously
        // confirmed empty, which permits a safe retry.
        return Err(RunOutcome::safe(PcileechTestState::Failed(error)));
    }

    Ok((child, process_job))
}

fn resolve_executable_path(configured_path: &Path) -> PathBuf {
    if configured_path.is_absolute() || configured_path.parent().is_none() {
        return configured_path.to_path_buf();
    }

    if let Ok(current_directory) = std::env::current_dir() {
        let working_directory_candidate = current_directory.join(configured_path);
        if working_directory_candidate.is_file() {
            return working_directory_candidate;
        }
    }

    if let Ok(current_executable) = std::env::current_exe()
        && let Some(executable_directory) = current_executable.parent()
    {
        let installed_candidate = executable_directory.join(configured_path);
        if installed_candidate.is_file() {
            return installed_candidate;
        }
    }

    configured_path.to_path_buf()
}

fn take_process_streams(child: &mut Child) -> Result<(ChildStdout, ChildStderr), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture test tool stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture test tool stderr".to_string())?;
    Ok((stdout, stderr))
}

fn terminate_process_tree_bounded(
    child: &mut Child,
    process_job: &ProcessJob,
    config: &RunConfig,
) -> Result<(), String> {
    process_job
        .terminate_and_wait(config.termination_grace, config.poll_interval)
        .map_err(|error| format!("Failed to terminate test process tree: {error}"))?;
    wait_for_exit_bounded(child, config, "test tool").map(|_| ())
}

fn terminate_direct_child_bounded(child: &mut Child, config: &RunConfig) -> Result<(), String> {
    let inspection_error = match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => None,
        Err(error) => Some(format!("Failed to inspect direct test process: {error}")),
    };

    // A failed status probe does not establish that the child is gone. Still
    // attempt termination so an uncontained suspended process cannot survive
    // this error path, then independently confirm the eventual outcome.
    let termination_error = child
        .kill()
        .map_err(|error| format!("Failed to terminate direct test process: {error}"))
        .err();

    match wait_for_exit_bounded(child, config, "direct test process") {
        Ok(_) => Ok(()),
        Err(confirmation_error) => {
            let mut errors = Vec::new();
            if let Some(error) = inspection_error {
                errors.push(error);
            }
            if let Some(error) = termination_error {
                errors.push(error);
            }
            errors.push(confirmation_error);
            Err(errors.join("; "))
        }
    }
}

fn wait_for_exit_bounded(
    child: &mut Child,
    config: &RunConfig,
    process_name: &str,
) -> Result<ExitStatus, String> {
    let deadline = Instant::now() + config.termination_grace;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(
                    config
                        .poll_interval
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(None) => {
                return Err(format!(
                    "{process_name} did not terminate within {} seconds",
                    config.termination_grace.as_secs_f32()
                ));
            }
            Err(error) => {
                return Err(format!(
                    "Failed to confirm {process_name} termination: {error}"
                ));
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
enum OutputEvent {
    Chunk(OutputStream, Vec<u8>),
    ReadError(OutputStream, String),
    Closed(OutputStream),
}

#[derive(Default)]
struct StreamCapture {
    complete: Vec<u8>,
    pending: Vec<u8>,
    captured_bytes: usize,
    closed: bool,
}

impl StreamCapture {
    fn record_chunk(&mut self, chunk: &[u8], limit: usize) -> (Vec<u8>, usize) {
        let remaining = limit.saturating_sub(self.captured_bytes);
        let accepted = remaining.min(chunk.len());
        self.captured_bytes += accepted;
        self.pending.extend_from_slice(&chunk[..accepted]);

        let Some(complete_len) = self
            .pending
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
        else {
            return (Vec::new(), accepted);
        };

        let tail = self.pending.split_off(complete_len);
        let completed = std::mem::replace(&mut self.pending, tail);
        self.complete.extend_from_slice(&completed);
        (completed, accepted)
    }

    fn close(&mut self) -> Vec<u8> {
        self.closed = true;
        let final_line = std::mem::take(&mut self.pending);
        self.complete.extend_from_slice(&final_line);
        final_line
    }

    fn text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.complete)
    }
}

#[derive(Default)]
struct CollectedOutput {
    stdout: StreamCapture,
    stderr: StreamCapture,
    diagnostics: Vec<u8>,
    read_error: Option<String>,
    truncation_error: Option<String>,
}

impl CollectedOutput {
    fn record(&mut self, event: OutputEvent) -> bool {
        match event {
            OutputEvent::Chunk(stream, chunk) => {
                let (capture_limit, stream_name) = match stream {
                    OutputStream::Stdout => (MAX_STDOUT_CAPTURE_BYTES, "stdout"),
                    OutputStream::Stderr => (MAX_STDERR_CAPTURE_BYTES, "stderr"),
                };
                let (completed, accepted) =
                    self.stream_mut(stream).record_chunk(&chunk, capture_limit);
                self.diagnostics.extend_from_slice(&completed);

                if accepted < chunk.len() {
                    self.truncation_error.get_or_insert_with(|| {
                        format!("Test tool {stream_name} output exceeded 1 MiB")
                    });
                }

                accepted != 0
            }
            OutputEvent::ReadError(stream, error) => {
                self.read_error.get_or_insert(error);
                self.close_stream(stream);
                false
            }
            OutputEvent::Closed(stream) => {
                self.close_stream(stream);
                false
            }
        }
    }

    fn stream_mut(&mut self, stream: OutputStream) -> &mut StreamCapture {
        match stream {
            OutputStream::Stdout => &mut self.stdout,
            OutputStream::Stderr => &mut self.stderr,
        }
    }

    fn close_stream(&mut self, stream: OutputStream) {
        if self.stream_mut(stream).closed {
            return;
        }

        let final_line = self.stream_mut(stream).close();
        if !final_line.is_empty() {
            self.diagnostics.extend_from_slice(&final_line);
            // A stream EOF frames its final unterminated line. Keep an
            // explicit separator so another stream can never splice into it.
            self.diagnostics.push(b'\n');
        }
    }

    fn streams_closed(&self) -> bool {
        self.stdout.closed && self.stderr.closed
    }

    fn stdout_text(&self) -> Cow<'_, str> {
        self.stdout.text()
    }

    fn diagnostic_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.diagnostics)
    }
}

fn spawn_output_reader<T>(
    mut stream: T,
    stream_kind: OutputStream,
    sender: SyncSender<OutputEvent>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()>
where
    T: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];

        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }

            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    if !send_output_event(
                        &sender,
                        OutputEvent::Chunk(stream_kind, buffer[..bytes_read].to_vec()),
                        &stop,
                    ) {
                        return;
                    }
                }
                Err(error) => {
                    let _ = send_output_event(
                        &sender,
                        OutputEvent::ReadError(
                            stream_kind,
                            format!("Failed to read test tool output: {error}"),
                        ),
                        &stop,
                    );
                    break;
                }
            }
        }

        let _ = send_output_event(&sender, OutputEvent::Closed(stream_kind), &stop);
    })
}

fn send_output_event(
    sender: &SyncSender<OutputEvent>,
    mut event: OutputEvent,
    stop: &AtomicBool,
) -> bool {
    loop {
        if stop.load(Ordering::Acquire) {
            return false;
        }

        match sender.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Full(returned)) => {
                event = returned;
                thread::sleep(OUTPUT_SEND_RETRY);
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

fn drain_available_output(receiver: &Receiver<OutputEvent>, output: &mut CollectedOutput) -> bool {
    let mut had_output = false;
    for _ in 0..MAX_EVENTS_PER_DRAIN {
        match receiver.try_recv() {
            Ok(event) => had_output |= output.record(event),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
    had_output
}

#[derive(Debug)]
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

fn cleanup_failure_outcome(
    termination_error: Option<String>,
    reader_error: Option<ReaderCleanupError>,
) -> Option<RunOutcome> {
    let blocks_restart = termination_error.is_some()
        || reader_error
            .as_ref()
            .is_some_and(ReaderCleanupError::blocks_restart);
    let mut errors = Vec::new();
    if let Some(error) = termination_error {
        errors.push(error);
    }
    if let Some(error) = reader_error {
        errors.push(error.into_message());
    }
    if errors.is_empty() {
        return None;
    }

    let state = PcileechTestState::Failed(errors.join("; "));
    Some(if blocks_restart {
        RunOutcome::unsafe_to_restart(state)
    } else {
        RunOutcome::safe(state)
    })
}

fn finish_output_readers(
    receiver: &Receiver<OutputEvent>,
    output: &mut CollectedOutput,
    reader_threads: [thread::JoinHandle<()>; 2],
    stop: &AtomicBool,
    config: &RunConfig,
) -> Result<(), ReaderCleanupError> {
    let deadline = Instant::now() + config.stream_drain_grace;

    while !output.streams_closed() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }

        match receiver.recv_timeout(remaining) {
            Ok(event) => {
                let _ = output.record(event);
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = drain_available_output(receiver, output);

    if !output.streams_closed() {
        stop.store(true, Ordering::Release);
        return Err(ReaderCleanupError::PossiblePipeHolder(format!(
            "Test tool output streams did not close within {} seconds",
            config.stream_drain_grace.as_secs_f32()
        )));
    }

    let join_deadline = Instant::now() + config.stream_drain_grace;
    while reader_threads.iter().any(|handle| !handle.is_finished())
        && Instant::now() < join_deadline
    {
        thread::sleep(
            config
                .poll_interval
                .min(join_deadline.saturating_duration_since(Instant::now())),
        );
    }

    if reader_threads.iter().any(|handle| !handle.is_finished()) {
        stop.store(true, Ordering::Release);
        return Err(ReaderCleanupError::Bookkeeping(format!(
            "Test tool output readers did not finish within {} seconds",
            config.stream_drain_grace.as_secs_f32()
        )));
    }

    for handle in reader_threads {
        handle.join().map_err(|_| {
            ReaderCleanupError::Bookkeeping("Test tool output reader panicked".to_string())
        })?;
    }

    Ok(())
}

enum StopReason {
    Cancelled,
    OutputLimit(String),
    Error(String),
    Exited(ExitStatus),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pcileech_test::parser::find_success_line;

    fn record_chunk(output: &mut CollectedOutput, stream: OutputStream, bytes: &[u8]) {
        let _ = output.record(OutputEvent::Chunk(stream, bytes.to_vec()));
    }

    fn subprocess_config(helper_name: &str, output_limit_grace: Duration) -> RunConfig {
        RunConfig {
            executable: std::env::current_exe().unwrap(),
            args: [helper_name, "--ignored", "--nocapture", "--test-threads=1"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            poll_interval: Duration::from_millis(5),
            termination_grace: Duration::from_secs(1),
            stream_drain_grace: Duration::from_millis(500),
            output_limit_grace,
        }
    }

    #[test]
    fn reader_bookkeeping_failure_does_not_block_restart() {
        let outcome = cleanup_failure_outcome(
            None,
            Some(ReaderCleanupError::Bookkeeping(
                "output reader panicked".to_string(),
            )),
        )
        .expect("a bookkeeping error must produce a failed outcome");

        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Failed(error) if error.contains("output reader panicked"))
        );
    }

    #[test]
    fn open_output_stream_still_blocks_restart() {
        let outcome = cleanup_failure_outcome(
            None,
            Some(ReaderCleanupError::PossiblePipeHolder(
                "output stream remained open".to_string(),
            )),
        )
        .expect("an open-stream error must produce a failed outcome");

        assert!(!outcome.safe_to_restart);
    }

    #[test]
    fn split_stdout_line_is_visible_only_after_it_is_complete() {
        let mut output = CollectedOutput::default();
        record_chunk(
            &mut output,
            OutputStream::Stdout,
            b"ntdll.dll base address: 0x7",
        );

        assert_eq!(find_success_line(&output.stdout_text()), None);

        record_chunk(&mut output, OutputStream::Stdout, b"ffa0000\n");
        assert_eq!(
            find_success_line(&output.stdout_text()),
            Some("ntdll.dll base address: 0x7ffa0000".to_string())
        );
    }

    #[test]
    fn valid_hex_prefix_in_a_live_tail_cannot_end_the_run_early() {
        let mut output = CollectedOutput::default();
        record_chunk(
            &mut output,
            OutputStream::Stdout,
            b"ntdll.dll base address: 0x7ffa0000",
        );

        assert_eq!(find_success_line(&output.stdout_text()), None);

        record_chunk(&mut output, OutputStream::Stdout, b" trailing diagnostic\n");
        assert_eq!(find_success_line(&output.stdout_text()), None);
    }

    #[test]
    fn cross_stream_chunks_cannot_splice_a_success_line() {
        let mut output = CollectedOutput::default();
        record_chunk(
            &mut output,
            OutputStream::Stdout,
            b"ntdll.dll base address: 0x",
        );
        record_chunk(&mut output, OutputStream::Stderr, b"7ffa0000\n");
        let _ = output.record(OutputEvent::Closed(OutputStream::Stdout));

        assert_eq!(find_success_line(&output.stdout_text()), None);
        assert_eq!(find_success_line(&output.diagnostic_text()), None);
    }

    #[test]
    fn a_complete_success_line_on_stderr_is_diagnostic_only() {
        let mut output = CollectedOutput::default();
        record_chunk(
            &mut output,
            OutputStream::Stderr,
            b"ntdll.dll base address: 0x7ffa0000\n",
        );

        assert_eq!(find_success_line(&output.stdout_text()), None);
    }

    #[test]
    fn eof_frames_an_unterminated_final_stdout_line() {
        let mut output = CollectedOutput::default();
        record_chunk(
            &mut output,
            OutputStream::Stdout,
            b"ntdll.dll base address: 0x7ffa0000",
        );
        assert_eq!(find_success_line(&output.stdout_text()), None);

        let _ = output.record(OutputEvent::Closed(OutputStream::Stdout));
        assert!(find_success_line(&output.stdout_text()).is_some());
    }

    fn command_config(command: &str) -> RunConfig {
        RunConfig {
            executable: PathBuf::from("cmd"),
            args: ["/C", command].into_iter().map(OsString::from).collect(),
            poll_interval: Duration::from_millis(5),
            termination_grace: Duration::from_millis(200),
            stream_drain_grace: Duration::from_millis(50),
            output_limit_grace: Duration::from_millis(100),
        }
    }

    #[test]
    fn reports_missing_executable() {
        let config = RunConfig {
            executable: PathBuf::from("definitely-missing-pcileech-test.exe"),
            ..RunConfig::default()
        };

        let outcome = run_with_config(&config, &CancellationToken::default());
        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Failed(error) if error.contains("Failed to start"))
        );
    }

    #[test]
    fn detects_success_after_streams_complete() {
        let config = command_config("echo memflow init & echo ntdll.dll base address: 0x7ffa0000");

        let outcome = run_with_config(&config, &CancellationToken::default());
        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Success(line) if line.contains("0x7ffa0000"))
        );
    }

    #[test]
    fn subprocess_runs_from_its_executable_directory() {
        let config = subprocess_config(
            "subprocess_reports_expected_working_directory_helper",
            Duration::from_millis(100),
        );

        let outcome = run_with_config(&config, &CancellationToken::default());

        assert!(
            matches!(outcome.state, PcileechTestState::Success(line) if line.contains("0x7ffa0000"))
        );
    }

    #[test]
    #[ignore = "subprocess helper for executable working-directory coverage"]
    fn subprocess_reports_expected_working_directory_helper() {
        let current_directory = std::env::current_dir().unwrap();
        let current_executable = std::env::current_exe().unwrap();
        let executable_directory = current_executable.parent().unwrap();

        if current_directory == executable_directory {
            println!("ntdll.dll base address: 0x7ffa0000");
        } else {
            eprintln!(
                "Error: expected working directory {}, got {}",
                executable_directory.display(),
                current_directory.display()
            );
        }
    }

    #[test]
    fn valid_success_wins_over_a_prior_output_error() {
        let config = command_config(
            "echo ntdll.dll base address: 0x7ffa0000 & echo Error: connector failed 1>&2",
        );

        let outcome = run_with_config(&config, &CancellationToken::default());
        assert!(matches!(outcome.state, PcileechTestState::Success(_)));
    }

    #[test]
    fn an_error_line_containing_the_signature_is_not_success() {
        let config = command_config("echo Error: missing ntdll.dll base address: 0x7ffa0000 1>&2");

        let outcome = run_with_config(&config, &CancellationToken::default());
        assert!(
            matches!(outcome.state, PcileechTestState::Failed(error) if error.contains("missing ntdll.dll"))
        );
    }

    #[test]
    fn waits_for_child_exit_after_success_output() {
        let config = subprocess_config(
            "success_output_before_process_exit_helper",
            Duration::from_millis(100),
        );
        let started_at = Instant::now();

        let outcome = run_with_config(&config, &CancellationToken::default());

        assert!(outcome.safe_to_restart);
        assert!(
            matches!(&outcome.state, PcileechTestState::Success(_)),
            "unexpected outcome: {:?}",
            outcome.state
        );
        assert!(
            started_at.elapsed() >= Duration::from_millis(200),
            "runner returned before the child exited after {:?}",
            started_at.elapsed()
        );
    }

    #[test]
    #[ignore = "subprocess helper for process-exit lifecycle coverage"]
    fn success_output_before_process_exit_helper() {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        writeln!(stdout, "ntdll.dll base address: 0x7ffa0000").unwrap();
        stdout.flush().unwrap();
        thread::sleep(Duration::from_millis(250));
    }

    #[test]
    fn cancellation_terminates_a_running_process_promptly() {
        let config = command_config("ping 127.0.0.1 -n 10 > nul");
        let cancellation = CancellationToken::default();
        let cancellation_worker = cancellation.clone();
        let started_at = Instant::now();
        let canceller = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            cancellation_worker.cancel();
        });

        let outcome = run_with_config(&config, &cancellation);
        canceller.join().unwrap();

        assert!(
            matches!(outcome.state, PcileechTestState::Failed(error) if error.contains("cancelled"))
        );
        assert!(outcome.safe_to_restart);
        assert!(
            started_at.elapsed() < Duration::from_secs(4),
            "cancellation took {:?}",
            started_at.elapsed()
        );
    }

    #[test]
    fn valid_stdout_success_survives_non_utf8_stderr() {
        let config = subprocess_config(
            "valid_stdout_success_with_non_utf8_stderr_helper",
            Duration::from_millis(100),
        );

        let outcome = run_with_config(&config, &CancellationToken::default());

        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Success(line) if line.contains("0x7ffa0000"))
        );
    }

    #[test]
    #[ignore = "subprocess helper for non-UTF-8 diagnostic coverage"]
    fn valid_stdout_success_with_non_utf8_stderr_helper() {
        use std::io::Write;

        let mut stderr = std::io::stderr();
        stderr
            .write_all(&[b'd', b'i', b'a', b'g', b':', b' ', 0xff, b'\n'])
            .unwrap();
        stderr.flush().unwrap();

        let mut stdout = std::io::stdout();
        writeln!(stdout, "ntdll.dll base address: 0x7ffa0000").unwrap();
        stdout.flush().unwrap();
    }

    #[test]
    fn noisy_stderr_cannot_consume_stdout_capture_budget() {
        let config = subprocess_config(
            "noisy_stderr_then_valid_stdout_success_helper",
            Duration::from_secs(2),
        );

        let outcome = run_with_config(&config, &CancellationToken::default());

        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Success(line) if line.contains("0x7ffa0000"))
        );
    }

    #[test]
    #[ignore = "subprocess helper for independent stream-limit coverage"]
    fn noisy_stderr_then_valid_stdout_success_helper() {
        use std::io::Write;

        let block = vec![b'x'; OUTPUT_CHUNK_BYTES];
        let mut stderr = std::io::stderr();
        for _ in 0..=(MAX_STDERR_CAPTURE_BYTES / OUTPUT_CHUNK_BYTES) {
            stderr.write_all(&block).unwrap();
        }
        stderr.flush().unwrap();

        let mut stdout = std::io::stdout();
        writeln!(stdout, "ntdll.dll base address: 0x7ffa0000").unwrap();
        stdout.flush().unwrap();
    }

    #[test]
    fn oversized_newline_free_output_is_bounded_and_fails() {
        let config = RunConfig {
            executable: std::env::current_exe().unwrap(),
            args: [
                "oversized_newline_free_output_helper",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            poll_interval: Duration::from_millis(5),
            termination_grace: Duration::from_secs(1),
            stream_drain_grace: Duration::from_millis(500),
            output_limit_grace: Duration::from_millis(100),
        };

        let outcome = run_with_config(&config, &CancellationToken::default());

        assert!(outcome.safe_to_restart);
        assert!(
            matches!(outcome.state, PcileechTestState::Failed(error) if error.contains("exceeded 1 MiB"))
        );
    }

    #[test]
    #[ignore = "subprocess helper for bounded-output coverage"]
    fn oversized_newline_free_output_helper() {
        use std::io::Write;

        let block = vec![b'x'; OUTPUT_CHUNK_BYTES];
        let mut stdout = std::io::stdout();
        for _ in 0..=(MAX_STDOUT_CAPTURE_BYTES / OUTPUT_CHUNK_BYTES) {
            stdout.write_all(&block).unwrap();
        }
        stdout.flush().unwrap();
        thread::sleep(Duration::from_secs(2));
    }
}
