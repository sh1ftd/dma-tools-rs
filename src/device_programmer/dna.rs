use crate::app::Language;
use crate::device_programmer::process::{CommandOptions, ProcessExecutor};
use crate::device_programmer::{
    CompletionStatus, DNA_OUTPUT_FILE, DnaInfo, FlashingOption, SCRIPT_DIR,
};
use crate::utils::localization::{TextKey, translate};
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

    pub fn execute(&self, option: &FlashingOption, executor: &ProcessExecutor, lang: &Language) {
        if !option.is_dna_read() {
            self.logger
                .error(translate(TextKey::DnaInvalidOption, lang));
            executor.set_completion_status(CompletionStatus::Failed(
                translate(TextKey::DnaInvalidOption, lang).to_string(),
            ));
            return;
        }

        // Mark operation as in progress right from the start
        executor.set_completion_status(CompletionStatus::InProgress(
            translate(TextKey::Initializing, lang).to_string(),
        ));

        // Clean up any existing DNA output file before starting
        Self::cleanup_dna_output_file(&self.logger);

        let (cmd, config) = option.get_command_args();
        let exe_path = format!("{SCRIPT_DIR}/{cmd}");
        let config_path = format!("{SCRIPT_DIR}/{config}");

        // Start background processing thread first (only sets up the thread)
        self.start_dna_processing_thread(executor, lang);

        // Then execute the command - this ensures we don't miss any data
        if !self.run_dna_command(&exe_path, &config_path, executor) {
            self.stop_processing_thread(); // Signal thread to stop if it hasn't already
            executor.set_completion_status(CompletionStatus::Failed(
                translate(TextKey::DnaCommandFailed, lang).to_string(),
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
            .debug(format!("Executing DNA read command: {command:?}"));

        let options = CommandOptions {
            update_duration: true,
            cleanup_temp_files: true,
        };

        match executor.execute_command(command, None, options) {
            Ok(_) => true,
            Err(e) => {
                self.logger
                    .error(format!("Failed to execute DNA read: {e}"));
                false
            }
        }
    }

    pub fn stop_processing_thread(&self) {
        self.thread_running.store(false, Ordering::SeqCst);
        self.logger.debug("DNA processing thread stop requested");
    }

    fn start_dna_processing_thread(&self, executor: &ProcessExecutor, lang: &Language) {
        if self.thread_running.load(Ordering::SeqCst) {
            self.logger
                .debug("DNA thread already running - not starting another");
            return;
        }

        let lang = *lang; // Capture copy for thread

        let logger_clone = self.logger.clone();
        let completion_status = Arc::clone(&executor.get_completion_status_arc());
        let thread_running = Arc::clone(&self.thread_running);

        thread_running.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            // Check if the command execution already failed before the thread started processing
            if let CompletionStatus::Failed(_) = *completion_status.lock().unwrap() {
                logger_clone.warning(
                    "DNA command execution failed before thread processing could complete.",
                );
                thread_running.store(false, Ordering::SeqCst);
                return;
            }

            if !thread_running.load(Ordering::SeqCst) {
                logger_clone.warning("DNA thread stopped before processing");
                return;
            }

            // Update status to "Waiting for device response"
            *completion_status.lock().unwrap() =
                CompletionStatus::InProgress(translate(TextKey::DnaRetrieving, &lang).to_string());

            thread::sleep(Duration::from_millis(DNA_READ_WAIT_MS));

            // After the wait, check if the file exists
            let dna_path = PathBuf::from(DNA_OUTPUT_FILE);

            // Delete potential incomplete/corrupt files
            Self::cleanup_incomplete_files(&dna_path, &logger_clone);

            // We need to track if we found the file during any attempt
            let mut found_dna_file = false;

            // First try multiple attempts to find the file
            for attempt in 1..=DNA_MAX_ATTEMPTS {
                if let Some(path) = Self::find_dna_file(&dna_path, &logger_clone) {
                    found_dna_file = true;
                    // Process the file...
                    // Process the file...
                    Self::parse_dna_file(&path, &logger_clone, &completion_status, &lang);
                    break; // Exit the loop if we found and processed the file
                }

                // Only wait between attempts if we haven't found it and still have attempts left
                if attempt < DNA_MAX_ATTEMPTS {
                    thread::sleep(Duration::from_millis(DNA_RETRY_WAIT_MS));
                }
            }

            // Only set to failed AFTER all attempts are exhausted AND the command didn't already fail
            if !found_dna_file {
                if let CompletionStatus::Failed(_) = *completion_status.lock().unwrap() {
                    logger_clone.warning("Command failed while waiting for DNA file.");
                } else {
                    let error_msg_native =
                        format!("DNA output file not found after {DNA_MAX_ATTEMPTS} attempts");
                    let error_msg = translate(TextKey::DnaFileNotFound, &lang)
                        .replace("{}", &DNA_MAX_ATTEMPTS.to_string());
                    logger_clone.error(&error_msg_native);
                    *completion_status.lock().unwrap() = CompletionStatus::Failed(error_msg);
                }
            }

            thread_running.store(false, Ordering::SeqCst);
        });
    }

    fn cleanup_incomplete_files(path: &PathBuf, logger: &Logger) {
        if let Ok(metadata) = fs::metadata(path) {
            // If the file is too small, it's likely incomplete
            if metadata.len() < MIN_VALID_DNA_FILE_SIZE {
                logger.warning(format!(
                    "Found potentially incomplete DNA file (only {} bytes). Removing it.",
                    metadata.len()
                ));
                if let Err(e) = fs::remove_file(path) {
                    logger.error(format!("Failed to remove incomplete DNA file: {e}"));
                }
            }
        }
    }

    fn find_dna_file(path: &PathBuf, logger: &Logger) -> Option<PathBuf> {
        match fs::metadata(path) {
            Ok(metadata) => {
                if metadata.is_file() && metadata.len() >= MIN_VALID_DNA_FILE_SIZE {
                    logger.debug(format!("Found DNA file at {}", path.display()));
                    return Some(path.clone());
                } else {
                    logger.debug(format!(
                        "Found file at {} but size is only {} bytes",
                        path.display(),
                        metadata.len()
                    ));
                }
            }
            Err(_) => {
                logger.debug(format!("DNA file not found at {}", path.display()));
            }
        }
        None
    }

    fn parse_dna_file(
        path: &Path,
        logger: &Logger,
        completion_status: &Arc<Mutex<CompletionStatus>>,
        lang: &Language,
    ) {
        match fs::read_to_string(path) {
            Ok(contents) => {
                logger.debug(format!(
                    "Successfully read DNA file: {} bytes",
                    contents.len()
                ));

                match Self::extract_dna_from_contents(&contents, logger) {
                    Ok(dna_info) => {
                        logger.info(format!(
                            "DNA read completed successfully: {}",
                            dna_info.dna_value
                        ));
                        *completion_status.lock().unwrap() =
                            CompletionStatus::DnaReadCompleted(dna_info);
                    }
                    Err(e) => {
                        logger.error(format!("Failed to extract DNA from contents: {e}"));
                        *completion_status.lock().unwrap() = CompletionStatus::Failed(
                            translate(TextKey::DnaExtractFailed, lang).replace("{}", &e),
                        );
                    }
                }
            }
            Err(e) => {
                let error_msg_native = format!(
                    "Failed to read DNA output file at {}: {}",
                    path.to_string_lossy(),
                    e
                );
                let error_msg = translate(TextKey::DnaFileReadError, lang)
                    .replace("{}", &path.to_string_lossy())
                    .replacen("{}", &e.to_string(), 1);

                logger.error(&error_msg_native);
                *completion_status.lock().unwrap() = CompletionStatus::Failed(error_msg);
            }
        }
    }

    fn extract_dna_from_contents(contents: &str, logger: &Logger) -> Result<DnaInfo, String> {
        // Detect device type
        let device_type = if contents.contains("CH347 Open Succ") {
            logger.debug("Detected CH347 device");
            "CH347"
        } else if contents.contains("ftdi:") {
            logger.debug("Detected FTDI device");
            "FTDI"
        } else {
            logger.warning("Unknown device type detected");
            "Unknown"
        };

        // Try standard DNA line format first: "DNA = <binary> (<hex>)"
        if let Some(dna_line) = contents
            .lines()
            .find(|line| line.trim().starts_with("DNA ="))
        {
            logger.debug(format!("Found DNA line: {dna_line}"));

            let parts: Vec<&str> = dna_line.split('=').collect();
            if parts.len() > 1 {
                let value_part = parts[1].trim();
                // Split binary and hex parts (hex is in parentheses)
                if let Some(hex_start) = value_part.find('(') {
                    let binary_part = value_part[..hex_start].trim();
                    if let Some(hex_end) = value_part.find(')') {
                        let hex_part = value_part[hex_start + 1..hex_end].trim();

                        // Basic validation
                        let is_binary = binary_part.chars().all(|c| c == '0' || c == '1');
                        let is_hex = hex_part.starts_with("0x")
                            && hex_part.len() > 2
                            && hex_part[2..].chars().all(|c| c.is_ascii_hexdigit());

                        if is_binary && is_hex {
                            return Ok(DnaInfo {
                                dna_value: hex_part.to_string(),
                                dna_raw_value: binary_part.to_string(),
                                device_type: device_type.to_string(),
                            });
                        } else {
                            logger.debug(format!(
                                "Failed validation: binary={is_binary}, hex={is_hex}"
                            ));
                        }
                    } else {
                        logger.debug("Could not find closing parenthesis ')'");
                    }
                } else {
                    logger.debug("Could not find opening parenthesis '('");
                }
            } else {
                logger.debug("Could not split line by '='");
            }
        } else {
            logger.debug("Did not find line starting with 'DNA ='");
        }

        Err(translate(TextKey::DnaInfoNotFound, &crate::app::Language::English).to_string())
    }

    pub fn cleanup_dna_output_file(logger: &Logger) {
        // Don't report an error if the file doesn't exist
        match fs::remove_file(DNA_OUTPUT_FILE) {
            Ok(_) => logger.debug("Successfully removed previous DNA output file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                logger.debug("No previous DNA output file found (which is expected)");
            }
            Err(e) => {
                // Other errors should still be logged
                logger.debug(format!("Could not remove previous DNA output file: {e}"));
            }
        }
    }

    // Get user-friendly status message for the current DNA read stage
    pub fn get_dna_read_stage(current_status: &CompletionStatus, lang: &Language) -> String {
        match current_status {
            CompletionStatus::NotCompleted => translate(TextKey::DnaWaitingStart, lang).to_string(),
            CompletionStatus::InProgress(_) => translate(TextKey::DnaRetrieving, lang).to_string(),
            // Handle the new status
            CompletionStatus::DnaReadCompleted(_) => {
                translate(TextKey::DnaReadSuccessStatus, lang).to_string()
            }
            CompletionStatus::Completed => {
                translate(TextKey::DnaOperationCompleted, lang).to_string()
            }
            CompletionStatus::Failed(err) => {
                translate(TextKey::DnaReadFailedStatus, lang).replace("{}", err)
            }
        }
    }

    // Converts binary DNA to Verilog hex format with 'h' prefix
    pub fn convert_dna_to_verilog_hex(binary: &str) -> String {
        // Parse the binary string into a number and convert to hex
        let value = u128::from_str_radix(binary, 2).unwrap();

        // Convert to hex (uppercase, no '0x' prefix)
        let mut hex_str = format!("{value:X}");

        // Make sure the result is exactly 16 characters long
        let current_len = hex_str.len();
        if current_len < 16 {
            // Add required number of zeros at the end
            hex_str.extend(std::iter::repeat_n('0', 16 - current_len));
        }

        // Add 'h' prefix
        format!("h{hex_str}")
    }
}
