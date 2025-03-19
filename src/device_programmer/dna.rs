use crate::device_programmer::process::ProcessExecutor;
use crate::device_programmer::{CompletionStatus, DNA_OUTPUT_FILE, FlashingOption, SCRIPT_DIR};
use crate::utils::logger::Logger;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Configuration constants
const DNA_READ_WAIT_MS: u64 = 50;

pub struct DnaReader {
    logger: Logger,
}

impl DnaReader {
    pub fn new(logger: Logger) -> Self {
        Self { logger }
    }

    pub fn execute(&self, option: &FlashingOption, executor: &ProcessExecutor) {
        if !option.is_dna_read() {
            self.logger.error("Invalid option for DNA read operation");
            executor.set_completion_status(CompletionStatus::Failed(
                "Invalid option for DNA read".to_string(),
            ));
            return;
        }

        // Clean up any existing DNA output file before starting
        if let Err(e) = std::fs::remove_file(DNA_OUTPUT_FILE) {
            // Only log as info since file may not exist, which is expected
            self.logger.info(format!(
                "Note: Could not remove previous DNA output file: {}",
                e
            ));
        }

        let (cmd, config) = option.get_command_args();
        let command_str = format!("{}/{} -f {}/{}", SCRIPT_DIR, cmd, SCRIPT_DIR, config);
        self.logger.command(format!("Executing: {}", command_str));

        let exe_path = format!("{}/{}", SCRIPT_DIR, cmd);
        let config_path = format!("{}/{}", SCRIPT_DIR, config);
        let logger_clone = self.logger.clone();

        // Get a clone of the completion status for the thread
        let completion_status = Arc::clone(&executor.get_completion_status_arc());

        let command =
            ProcessExecutor::prepare_command(&exe_path, &["-f", &config_path, "-c", "exit"]);

        if let Err(e) = executor.execute_command(
            command, None,  // No line callback needed
            false, // Don't update duration
            false, // No temp file to clean up
        ) {
            self.logger
                .error(format!("Failed to execute DNA read: {}", e));
            return;
        }

        // Wait a moment for the DNA file to be generated
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(DNA_READ_WAIT_MS));
            Self::process_dna_output(&logger_clone, &completion_status);
        });
    }

    fn process_dna_output(logger: &Logger, completion_status: &Arc<Mutex<CompletionStatus>>) {
        // Check multiple possible locations for the output file
        let possible_paths: Vec<String> = vec![
            DNA_OUTPUT_FILE.to_string(),
            format!("./{}", DNA_OUTPUT_FILE),
            std::env::current_dir()
                .unwrap_or_default()
                .join(DNA_OUTPUT_FILE)
                .to_string_lossy()
                .to_string(),
        ];

        logger.info("Looking for DNA output file...");
        let mut file_found = false;

        for path in &possible_paths {
            logger.info(format!("Trying path: {}", path));
            if let Ok(metadata) = std::fs::metadata(path) {
                if metadata.is_file() {
                    file_found = true;
                    logger.info(format!("Found DNA output file at: {}", path));

                    match std::fs::read_to_string(path) {
                        Ok(contents) => {
                            logger.info(format!("File contents: {}", contents));

                            // Extract DNA hex value from output
                            if let Some(dna_line) =
                                contents.lines().find(|line| line.contains("DNA ="))
                            {
                                logger.info(format!("Found DNA line: {}", dna_line));

                                if let Some(hex_start) = dna_line.find("(0x") {
                                    if let Some(hex_end) = dna_line[hex_start..].find(")") {
                                        let dna_hex = &dna_line[hex_start + 1..hex_start + hex_end];
                                        logger.success(format!(
                                            "DNA read completed successfully: {}",
                                            dna_hex
                                        ));
                                        *completion_status.lock().unwrap() =
                                            CompletionStatus::Completed;
                                    } else {
                                        logger.error(
                                            "Failed to parse DNA hex value from output file",
                                        );
                                        *completion_status.lock().unwrap() =
                                            CompletionStatus::Failed(
                                                "Failed to parse DNA hex value".to_string(),
                                            );
                                    }
                                } else {
                                    logger.error("DNA hex value not found in output file");
                                    *completion_status.lock().unwrap() = CompletionStatus::Failed(
                                        "DNA hex value not found".to_string(),
                                    );
                                }
                            } else {
                                logger.error("DNA line not found in output file");
                                *completion_status.lock().unwrap() = CompletionStatus::Failed(
                                    "DNA information not found in output file".to_string(),
                                );
                            }
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to read DNA output file at {}: {}", path, e);
                            logger.error(&error_msg);
                            *completion_status.lock().unwrap() =
                                CompletionStatus::Failed(error_msg);
                        }
                    }
                    break;
                }
            }
        }

        if !file_found {
            let current_dir = std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let error_msg = format!(
                "DNA output file not found in any expected location. Current directory: {}",
                current_dir
            );
            logger.error(&error_msg);
            *completion_status.lock().unwrap() = CompletionStatus::Failed(error_msg);
        }
    }
}
