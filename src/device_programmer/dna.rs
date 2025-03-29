use crate::device_programmer::process::{CommandOptions, ProcessExecutor};
use crate::device_programmer::{CompletionStatus, DNA_OUTPUT_FILE, FlashingOption, SCRIPT_DIR};
use crate::utils::logger::Logger;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Configuration constants
const DNA_READ_WAIT_MS: u64 = 50;
const DNA_RETRY_WAIT_MS: u64 = 200;
const DNA_MAX_ATTEMPTS: usize = 5;
const MIN_VALID_DNA_FILE_SIZE: u64 = 10;

pub struct DnaReader {
    logger: Logger,
    thread_running: Arc<AtomicBool>,
}

impl DnaReader {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            thread_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn execute(&self, option: &FlashingOption, executor: &ProcessExecutor) {
        if !option.is_dna_read() {
            self.logger.error("Invalid option for DNA read operation");
            executor.set_completion_status(CompletionStatus::Failed(
                "Invalid option for DNA read".to_string(),
            ));
            return;
        }

        // Mark operation as in progress right from the start
        executor.set_completion_status(CompletionStatus::InProgress(
            "Initializing DNA read operation...".to_string(),
        ));

        // Clean up any existing DNA output file before starting
        Self::cleanup_dna_output_file(&self.logger);

        let (cmd, config) = option.get_command_args();
        let exe_path = format!("{}/{}", SCRIPT_DIR, cmd);
        let config_path = format!("{}/{}", SCRIPT_DIR, config);

        // Start background processing thread first (only sets up the thread)
        self.start_dna_processing_thread(executor);

        // Then execute the command - this ensures we don't miss any data
        if !self.run_dna_command(&exe_path, &config_path, executor) {
            executor.set_completion_status(CompletionStatus::Failed(
                "Failed to execute DNA read command".to_string(),
            ));
        }
    }

    fn run_dna_command(
        &self,
        exe_path: &str,
        config_path: &str,
        executor: &ProcessExecutor,
    ) -> bool {
        let command =
            ProcessExecutor::prepare_command(exe_path, &["-f", config_path, "-c", "exit"]);

        self.logger
            .debug(format!("Executing DNA read command: {:?}", command));

        let options = CommandOptions {
            update_duration: true,
            cleanup_temp_files: true,
        };

        match executor.execute_command(command, None, options) {
            Ok(_) => true,
            Err(e) => {
                self.logger
                    .error(format!("Failed to execute DNA read: {}", e));
                false
            }
        }
    }

    pub fn stop_processing_thread(&self) {
        self.thread_running.store(false, Ordering::SeqCst);
        self.logger.warning("DNA processing thread stop requested");
    }

    fn start_dna_processing_thread(&self, executor: &ProcessExecutor) {
        if self.thread_running.load(Ordering::SeqCst) {
            self.logger
                .warning("DNA thread already running - not starting another");
            return;
        }

        let logger_clone = self.logger.clone();
        let completion_status = Arc::clone(&executor.get_completion_status_arc());
        let thread_running = Arc::clone(&self.thread_running);

        thread_running.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            if !thread_running.load(Ordering::SeqCst) {
                logger_clone.warning("DNA thread stopped before processing");
                return;
            }

            // Update status to "Waiting for device response"
            *completion_status.lock().unwrap() =
                CompletionStatus::InProgress("Waiting for device response...".to_string());

            thread::sleep(Duration::from_millis(DNA_READ_WAIT_MS));

            // After the wait, check if the file exists
            let possible_paths = Self::get_possible_dna_file_paths();

            // Delete potential incomplete/corrupt files
            Self::cleanup_incomplete_files(&possible_paths, &logger_clone);

            // We need to track if we found the file during any attempt
            let mut found_dna_file = false;

            // First try multiple attempts to find the file
            for attempt in 1..=DNA_MAX_ATTEMPTS {
                if let Some(path) = Self::find_dna_file(&possible_paths, &logger_clone) {
                    found_dna_file = true;
                    // Process the file...
                    Self::parse_dna_file(&path, &logger_clone, &completion_status);
                    break; // Exit the loop if we found and processed the file
                }

                // Only wait between attempts if we haven't found it and still have attempts left
                if attempt < DNA_MAX_ATTEMPTS {
                    thread::sleep(Duration::from_millis(DNA_RETRY_WAIT_MS));
                }
            }

            // Only set to failed AFTER all attempts are exhausted
            if !found_dna_file {
                let error_msg = format!(
                    "DNA output file not found after {} attempts",
                    DNA_MAX_ATTEMPTS
                );
                logger_clone.error(&error_msg);
                *completion_status.lock().unwrap() = CompletionStatus::Failed(error_msg);
            }

            thread_running.store(false, Ordering::SeqCst);
        });
    }

    fn cleanup_incomplete_files(paths: &[PathBuf], logger: &Logger) {
        for path in paths {
            if path.exists() {
                if let Ok(metadata) = std::fs::metadata(path) {
                    let size = metadata.len();
                    logger.debug(format!(
                        "Found existing file at {} (size: {} bytes)",
                        path.display(),
                        size
                    ));

                    if size < MIN_VALID_DNA_FILE_SIZE {
                        // If file is suspiciously small
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
    }

    fn get_possible_dna_file_paths() -> Vec<PathBuf> {
        // For testing
        // vec![PathBuf::from("OpenOCD/test.log")]

        vec![
            PathBuf::from(DNA_OUTPUT_FILE),
            PathBuf::from(format!("./{}", DNA_OUTPUT_FILE)),
            std::env::current_dir()
                .unwrap_or_default()
                .join(DNA_OUTPUT_FILE),
        ]
    }

    fn find_dna_file(possible_paths: &[PathBuf], logger: &Logger) -> Option<PathBuf> {
        for path in possible_paths {
            let path_str = path.to_string_lossy();
            logger.debug(format!("Trying path: {}", path_str));

            if path.exists() && path.is_file() {
                logger.debug(format!("Found DNA output file at: {}", path_str));
                return Some(path.clone());
            }
        }
        None
    }

    fn parse_dna_file(
        path: &Path,
        logger: &Logger,
        completion_status: &Arc<Mutex<CompletionStatus>>,
    ) {
        match fs::read_to_string(path) {
            Ok(contents) => {
                logger.debug(format!("DNA file contents: {}", contents));
                Self::extract_dna_from_contents(&contents, logger, completion_status);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to read DNA output file at {}: {}",
                    path.to_string_lossy(),
                    e
                );
                logger.error(&error_msg);
                *completion_status.lock().unwrap() = CompletionStatus::Failed(error_msg);
            }
        }
    }

    fn extract_dna_from_contents(
        contents: &str,
        logger: &Logger,
        completion_status: &Arc<Mutex<CompletionStatus>>,
    ) {
        if contents.contains("CH347 Open Succ") {
            logger.debug("Detected CH347 device");
        } else if contents.contains("ftdi:") {
            logger.debug("Detected FTDI device");
        } else {
            logger.warning("Unknown device type detected");
        }

        if let Some(dna_line) = contents.lines().find(|line| line.contains("DNA =")) {
            logger.debug(format!("Found DNA line: {}", dna_line));
            Self::extract_dna_hex_value(dna_line, logger, completion_status);
        } else {
            logger.error("DNA line not found in output file");
            *completion_status.lock().unwrap() =
                CompletionStatus::Failed("DNA information not found in output file".to_string());
        }
    }

    fn extract_dna_hex_value(
        dna_line: &str,
        logger: &Logger,
        completion_status: &Arc<Mutex<CompletionStatus>>,
    ) {
        // Parsing with trimming to handle different whitespace patterns
        let dna_line = dna_line.trim();

        if let Some(hex_start) = dna_line.find("(0x") {
            if let Some(hex_end) = dna_line[hex_start..].find(")") {
                let dna_hex = &dna_line[hex_start + 1..hex_start + hex_end];

                // Verify the hex value format
                if dna_hex.starts_with("0x") && dna_hex.len() > 2 {
                    logger.success(format!("DNA read completed successfully: {}", dna_hex));

                    // Store the device type along with the DNA value
                    let device_type = if dna_line.contains("CH347") {
                        "CH347"
                    } else if dna_line.contains("ftdi") {
                        "FTDI"
                    } else {
                        "Unknown"
                    };

                    logger.debug(format!("Device type identified as: {}", device_type));
                    *completion_status.lock().unwrap() = CompletionStatus::Completed;
                } else {
                    logger.error(format!("Invalid hex format: {}", dna_hex));
                    *completion_status.lock().unwrap() =
                        CompletionStatus::Failed("Invalid DNA hex format".to_string());
                }
            } else {
                logger.error("Failed to parse DNA hex value from output file");
                *completion_status.lock().unwrap() =
                    CompletionStatus::Failed("Failed to parse DNA hex value".to_string());
            }
        } else {
            logger.error("DNA hex value not found in output file");
            *completion_status.lock().unwrap() =
                CompletionStatus::Failed("DNA hex value not found".to_string());
        }
    }

    pub fn cleanup_dna_output_file(logger: &Logger) {
        // Don't report an error if the file doesn't exist
        match fs::remove_file(crate::device_programmer::DNA_OUTPUT_FILE) {
            Ok(_) => logger.debug("Successfully removed previous DNA output file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                logger.debug("No previous DNA output file found (which is expected)");
            }
            Err(e) => {
                // Other errors should still be logged
                logger.debug(format!("Could not remove previous DNA output file: {}", e));
            }
        }
    }

    // Get user-friendly status message for the current DNA read stage
    pub fn get_dna_read_stage(current_status: &CompletionStatus) -> String {
        match current_status {
            CompletionStatus::NotCompleted => "Waiting to start DNA read...".to_string(),
            CompletionStatus::InProgress(_) => "Retrieving device DNA...".to_string(),
            CompletionStatus::Completed => "DNA read successful!".to_string(),
            CompletionStatus::Failed(err) => format!("DNA read failed: {}", err),
        }
    }
}
