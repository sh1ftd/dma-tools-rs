use crate::device_programmer::{CREATE_NO_WINDOW, CompletionStatus, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

type LineCallback = Option<Box<dyn Fn(&str) + Send + Sync + 'static>>;

pub struct ProcessExecutor {
    logger: Logger,
    completion_status: Arc<Mutex<CompletionStatus>>,
    start_time: Arc<Mutex<Option<Instant>>>,
    terminated_flag: Arc<AtomicBool>,
}

pub struct CommandOptions {
    pub update_duration: bool,
    pub cleanup_temp_files: bool,
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
                self.attach_readers(&mut child, &options, on_line_callback);

                // Wait in a separate thread for the process to complete
                let logger = self.logger.clone();
                let completion_status = Arc::clone(&self.completion_status);
                let start_time = Arc::clone(&self.start_time);

                thread::spawn(move || {
                    match child.wait() {
                        Ok(exit_status) => {
                            // Update execution duration
                            if options.update_duration {
                                if let Some(start) = *start_time.lock().unwrap() {
                                    let elapsed = start.elapsed();

                                    // Format duration in a readable way based on the actual time
                                    if elapsed.as_secs() > 0 {
                                        // If operation took more than a second, show seconds.milliseconds
                                        let seconds = elapsed.as_secs();
                                        let millis = elapsed.subsec_millis();
                                        logger.info(format!(
                                            "Operation took {}.{:03}s",
                                            seconds, millis
                                        ));
                                    } else {
                                        // For very quick operations, show milliseconds
                                        logger.info(format!(
                                            "Operation took {}ms",
                                            elapsed.as_millis()
                                        ));
                                    }
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
                    if options.cleanup_temp_files {
                        if let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE) {
                            logger.debug(format!(
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

                if options.cleanup_temp_files {
                    if let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE) {
                        self.logger
                            .warning(format!("Failed to clean up temporary firmware file: {}", e));
                    }
                }

                Err(error_msg)
            }
        }
    }

    fn attach_readers(
        &self,
        child: &mut Child,
        _options: &CommandOptions,
        line_callback: LineCallback,
    ) {
        // Create a channel for sector line communication
        let (_sector_tx, sector_rx) = mpsc::channel::<String>();

        // Wrap the callback in an Arc for sharing between threads
        let callback_arc = Arc::new(line_callback);

        // For stdout
        if let Some(stdout) = child.stdout.take() {
            let stdout_logger = self.logger.clone();
            let callback_opt = Arc::clone(&callback_arc); // Clone the Arc, not the callback
            let rx = sector_rx;

            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                let mut lines_iter = reader.lines(); // Create a lines iterator outside the loop
                let mut stdout_done = false;

                // Use a non-blocking approach to handle both stdout and sector lines
                loop {
                    // Process any available sector lines (non-blocking)
                    match rx.try_recv() {
                        Ok(sector_line) => {
                            stdout_logger.info(format!("RECEIVED sector line: {}", sector_line));

                            if let Some(cb) = &*callback_opt {
                                stdout_logger.info("About to call callback with sector line");
                                cb(&sector_line);
                                stdout_logger.info("Callback completed for sector line");
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            // No sector lines available right now, that's fine
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            // Stderr channel closed, if stdout is also done, we can exit
                            if stdout_done {
                                break;
                            }
                        }
                    }

                    // If stdout isn't done, try to read more lines
                    if !stdout_done {
                        let mut reader_empty = true;

                        // Try to read a line from stdout using the iterator
                        if let Some(line_result) = lines_iter.next() {
                            reader_empty = false;
                            if let Ok(line) = line_result {
                                stdout_logger.output(&line);

                                // Process the stdout line
                                if line.contains("sector") && line.contains("took") {
                                    stdout_logger.warning(format!("STDOUT SECTOR LINE: {}", line));
                                    if let Some(cb) = &*callback_opt {
                                        cb(&line);
                                    }
                                } else if let Some(cb) = &*callback_opt {
                                    cb(&line);
                                }
                            }
                        }

                        // If no lines were read, stdout might be done
                        if reader_empty {
                            stdout_done = true;
                        }
                    } else {
                        // If stdout is done, try to check if there are any more messages
                        match rx.try_recv() {
                            Ok(sector_line) => {
                                // There was one more message, process it
                                stdout_logger.info(format!("Extra sector line: {}", sector_line));
                                if let Some(cb) = &*callback_opt {
                                    cb(&sector_line);
                                }
                            }
                            Err(mpsc::TryRecvError::Empty) => {
                                // Channel is empty, safe to exit
                                break;
                            }
                            Err(mpsc::TryRecvError::Disconnected) => {
                                // Channel is closed, safe to exit
                                break;
                            }
                        }
                    }

                    // Small delay to prevent CPU hogging
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }

                // Final check for any remaining sector lines
                while let Ok(sector_line) = rx.recv() {
                    stdout_logger.info(format!("FINAL sector line: {}", sector_line));
                    if let Some(cb) = &*callback_opt {
                        cb(&sector_line);
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
                        stderr_logger.debug(format!("Stderr sector line: {}", line));

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

    pub fn get_start_time(&self) -> &Arc<Mutex<Option<Instant>>> {
        &self.start_time
    }

    pub fn get_completion_status_arc(&self) -> Arc<Mutex<CompletionStatus>> {
        Arc::clone(&self.completion_status)
    }
}
