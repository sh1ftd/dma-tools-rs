use crate::device_programmer::{CREATE_NO_WINDOW, CompletionStatus, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

type LineCallback = Option<Box<dyn Fn(&str) + Send + 'static>>;

pub struct ProcessExecutor {
    logger: Logger,
    completion_status: Arc<Mutex<CompletionStatus>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl ProcessExecutor {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            completion_status: Arc::new(Mutex::new(CompletionStatus::NotCompleted)),
            start_time: Arc::new(Mutex::new(None)),
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
        update_duration: bool,
        cleanup_temp_file: bool,
    ) -> Result<(), String> {
        match command.spawn() {
            Ok(mut child) => {
                let logger_clone = self.logger.clone();
                let completion_status = Arc::clone(&self.completion_status);
                let start_time = Arc::clone(&self.start_time);
                let pid = child.id();

                // Create a shareable reference to the child process ID
                let child_process = Arc::new(Mutex::new(Some(pid)));

                self.handle_command_stdout(
                    &mut child,
                    logger_clone.clone(),
                    on_line_callback,
                    child_process,
                );
                self.handle_command_stderr(&mut child, logger_clone.clone());

                // Wait in a separate thread for the process to complete
                let logger = self.logger.clone();
                thread::spawn(move || {
                    match child.wait() {
                        Ok(exit_status) => {
                            // Update execution duration
                            if update_duration {
                                if let Some(start) = *start_time.lock().unwrap() {
                                    let elapsed = start.elapsed();
                                    logger
                                        .info(format!("Operation took {}ms", elapsed.as_millis()));
                                }
                            }

                            if exit_status.success() {
                                logger.success("Command completed successfully");
                                *completion_status.lock().unwrap() = CompletionStatus::Completed;
                            } else {
                                let error_msg = format!(
                                    "Command failed with exit code: {:?}",
                                    exit_status.code()
                                );
                                logger.error(&error_msg);
                                *completion_status.lock().unwrap() =
                                    CompletionStatus::Failed(error_msg.clone());
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to wait for process: {}", e);
                            logger.error(&error_msg);
                            *completion_status.lock().unwrap() =
                                CompletionStatus::Failed(error_msg.clone());
                        }
                    }

                    // Clean up temporary file
                    if cleanup_temp_file {
                        if let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE) {
                            logger.warning(format!(
                                "Failed to clean up temporary firmware file: {}",
                                e
                            ));
                        }
                    }
                });

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to start process: {}", e);
                self.logger.error(&error_msg);
                *self.completion_status.lock().unwrap() =
                    CompletionStatus::Failed(error_msg.clone());

                if cleanup_temp_file {
                    if let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE) {
                        self.logger
                            .warning(format!("Failed to clean up temporary firmware file: {}", e));
                    }
                }

                Err(error_msg)
            }
        }
    }

    fn handle_command_stdout(
        &self,
        child: &mut Child,
        logger: Logger,
        on_line_callback: LineCallback,
        child_process: Arc<Mutex<Option<u32>>>,
    ) {
        if let Some(stdout) = child.stdout.take() {
            let stdout_logger = logger.clone();
            let stdout_reader = BufReader::new(stdout);
            let callback = on_line_callback;
            let _process_id = Arc::clone(&child_process);

            thread::spawn(move || {
                for line in stdout_reader.lines().map_while(Result::ok) {
                    stdout_logger.output(&line);

                    // Call the callback if provided
                    if let Some(ref cb) = callback {
                        cb(&line);
                    }
                }
            });
        }
    }

    fn handle_command_stderr(&self, child: &mut Child, logger: Logger) {
        if let Some(stderr) = child.stderr.take() {
            let stderr_logger = logger;
            let stderr_reader = BufReader::new(stderr);

            thread::spawn(move || {
                for line in stderr_reader.lines().map_while(Result::ok) {
                    stderr_logger.error(&line);
                }
            });
        }
    }

    #[allow(dead_code)]
    pub fn terminate_process(&self, pid: u32) {
        self.logger
            .warning(format!("Forcibly terminating process {}", pid));

        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }

        #[cfg(not(windows))]
        {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        }
    }

    pub fn set_completion_status(&self, status: CompletionStatus) {
        *self.completion_status.lock().unwrap() = status;
    }

    pub fn get_start_time(&self) -> &Arc<Mutex<Option<Instant>>> {
        &self.start_time
    }

    pub fn get_completion_status_arc(&self) -> Arc<Mutex<CompletionStatus>> {
        Arc::clone(&self.completion_status)
    }
}
