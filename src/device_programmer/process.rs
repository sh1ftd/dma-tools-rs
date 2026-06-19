use crate::device_programmer::{CREATE_NO_WINDOW, CompletionStatus, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type LineCallback = Option<Box<dyn Fn(&str) + Send + Sync + 'static>>;
type CompletionCallback = Option<Box<dyn FnOnce(bool) + Send + 'static>>;

pub struct ProcessExecutor {
    logger: Logger,
    completion_status: Arc<Mutex<CompletionStatus>>,
    start_time: Arc<Mutex<Option<Instant>>>,
    terminated_flag: Arc<AtomicBool>,
}

pub struct CommandOptions {
    pub log_duration: bool,
    pub cleanup_temp_files: bool,
    pub duration_target: Option<Arc<Mutex<Option<Duration>>>>,
    pub on_complete: CompletionCallback,
}

impl ProcessExecutor {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            completion_status: Arc::new(Mutex::new(CompletionStatus::NotCompleted)),
            start_time: Arc::new(Mutex::new(None)),
            terminated_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn reset(&self) {
        *self.completion_status.lock().unwrap() = CompletionStatus::NotCompleted;
        *self.start_time.lock().unwrap() = Some(Instant::now());
    }

    pub fn get_completion_status(&self) -> CompletionStatus {
        self.completion_status.lock().unwrap().clone()
    }

    pub fn prepare_command(exe_path: &str, args: &[&str]) -> Command {
        let mut command = Command::new(exe_path);
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.creation_flags(CREATE_NO_WINDOW); // Hide the window on Windows
        command
    }

    pub fn execute_command(
        &self,
        mut command: Command,
        on_line_callback: LineCallback,
        options: CommandOptions,
    ) -> Result<(), String> {
        match command.spawn() {
            Ok(mut child) => {
                self.attach_readers(&mut child, on_line_callback);

                // Wait in a separate thread for the process to complete
                let logger = self.logger.clone();
                let completion_status = Arc::clone(&self.completion_status);
                let start_time = Arc::clone(&self.start_time);

                thread::spawn(move || {
                    let mut options = options;

                    match child.wait() {
                        Ok(exit_status) => {
                            let elapsed = start_time.lock().unwrap().map(|start| start.elapsed());

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

                                *completion_status.lock().unwrap() =
                                    CompletionStatus::Failed(error_msg.clone());
                            }
                        }
                        Err(e) => {
                            if let Some(on_complete) = options.on_complete.take() {
                                on_complete(false);
                            }

                            let error_msg = format!("Failed to wait for process: {e}");
                            logger.error(&error_msg);
                            *completion_status.lock().unwrap() =
                                CompletionStatus::Failed(error_msg.clone());
                        }
                    }

                    // Clean up temporary file
                    if options.cleanup_temp_files
                        && let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE)
                    {
                        logger.debug(format!("Failed to clean up temporary firmware file: {e}"));
                    }
                });

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to start process: {e}");
                self.logger.error(&error_msg);
                *self.completion_status.lock().unwrap() =
                    CompletionStatus::Failed(error_msg.clone());

                if options.cleanup_temp_files
                    && let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE)
                {
                    self.logger
                        .warning(format!("Failed to clean up temporary firmware file: {e}"));
                }

                Err(error_msg)
            }
        }
    }

    fn attach_readers(&self, child: &mut Child, line_callback: LineCallback) {
        // Wrap the callback in an Arc for sharing between threads
        let callback_arc = Arc::new(line_callback);

        // For stdout
        if let Some(stdout) = child.stdout.take() {
            let stdout_logger = self.logger.clone();
            let callback_opt = Arc::clone(&callback_arc);

            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    stdout_logger.output(&line);

                    // Forward every line to the callback
                    if let Some(cb) = &*callback_opt {
                        cb(&line);
                    }
                }

                stdout_logger.debug("Stdout processor thread completed");
            });
        }

        // For stderr
        if let Some(stderr) = child.stderr.take() {
            let stderr_logger = self.logger.clone();

            // Clone the Arc, not the inner callback
            let callback_opt_for_stderr = Arc::clone(&callback_arc);

            // Clone the terminated_flag to pass to the thread
            let terminated_flag = Arc::clone(&self.terminated_flag);

            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    stderr_logger.error(&line);

                    // Look for sector lines in stderr
                    if line.contains("sector") && line.contains("took") {
                        stderr_logger.debug(format!("Stderr sector line: {line}"));

                        // Check for termination before processing
                        if let Some(cb) = &*callback_opt_for_stderr {
                            // Now we can access the terminated_flag
                            let terminated = terminated_flag.load(Ordering::SeqCst);
                            if !terminated {
                                stderr_logger.debug("Calling callback with sector line");
                                cb(&line);
                            } else {
                                stderr_logger
                                    .debug("Skipping callback - process already terminated");
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn set_completion_status(&self, status: CompletionStatus) {
        *self.completion_status.lock().unwrap() = status;
    }

    pub fn get_completion_status_arc(&self) -> Arc<Mutex<CompletionStatus>> {
        Arc::clone(&self.completion_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_programmer::DnaInfo;

    fn wait_for_terminal_status(executor: &ProcessExecutor) -> CompletionStatus {
        let deadline = Instant::now() + Duration::from_secs(5);

        loop {
            let status = executor.get_completion_status();
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
    fn completion_callback_status_is_not_overwritten_on_success() {
        let executor = ProcessExecutor::new(Logger::new("ProcessExecutorTest"));
        executor.reset();

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
        executor.reset();

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
}
