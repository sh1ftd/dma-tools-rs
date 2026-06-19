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
const DNA_RETRY_WAIT_MS: u64 = 200;
const DNA_MAX_ATTEMPTS: usize = 5;
const MIN_VALID_DNA_FILE_SIZE: u64 = 10;

pub struct DnaReader {
    logger: Logger,
    parse_enabled: Arc<AtomicBool>,
}

impl DnaReader {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            parse_enabled: Arc::new(AtomicBool::new(false)),
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

        let parse_callback = self.create_dna_parse_callback(executor, lang);

        // Parse only after OpenOCD exits successfully, so slow output writes do not race the parser.
        if !self.run_dna_command(&exe_path, &config_path, executor, parse_callback) {
            self.stop_output_parsing();
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
        parse_callback: Box<dyn FnOnce(bool) + Send + 'static>,
    ) -> bool {
        let command =
            ProcessExecutor::prepare_command(exe_path, &["-f", config_path, "-c", "exit"]);

        self.logger
            .debug(format!("Executing DNA read command: {command:?}"));

        let options = CommandOptions {
            log_duration: true,
            cleanup_temp_files: false,
            duration_target: None,
            on_complete: Some(parse_callback),
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

    pub fn stop_output_parsing(&self) {
        self.parse_enabled.store(false, Ordering::SeqCst);
        self.logger.debug("DNA output parsing stop requested");
    }

    fn create_dna_parse_callback(
        &self,
        executor: &ProcessExecutor,
        lang: &Language,
    ) -> Box<dyn FnOnce(bool) + Send + 'static> {
        let lang = *lang; // Capture copy for thread

        let logger_clone = self.logger.clone();
        let completion_status = Arc::clone(&executor.get_completion_status_arc());
        let parse_enabled = Arc::clone(&self.parse_enabled);

        parse_enabled.store(true, Ordering::SeqCst);

        Box::new(move |command_succeeded| {
            if !command_succeeded {
                logger_clone.warning("DNA command failed before output parsing could run.");
                parse_enabled.store(false, Ordering::SeqCst);
                return;
            }

            if !parse_enabled.load(Ordering::SeqCst) {
                logger_clone.warning("DNA output parsing was stopped before processing");
                *completion_status.lock().unwrap() = CompletionStatus::Failed(
                    "DNA output parsing was stopped before completion.".to_string(),
                );
                return;
            }

            // Update status to "Waiting for device response"
            *completion_status.lock().unwrap() =
                CompletionStatus::InProgress(translate(TextKey::DnaRetrieving, &lang).to_string());

            let dna_path = PathBuf::from(DNA_OUTPUT_FILE);

            // Delete potential incomplete/corrupt files
            Self::cleanup_incomplete_files(&dna_path, &logger_clone);

            // We need to track if we found the file during any attempt
            let mut found_dna_file = false;

            // First try multiple attempts to find the file
            for attempt in 1..=DNA_MAX_ATTEMPTS {
                if let Some(path) = Self::find_dna_file(&dna_path, &logger_clone) {
                    found_dna_file = true;
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

            parse_enabled.store(false, Ordering::SeqCst);
        })
    }

    fn cleanup_incomplete_files(path: &Path, logger: &Logger) {
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

    fn find_dna_file(path: &Path, logger: &Logger) -> Option<PathBuf> {
        match fs::metadata(path) {
            Ok(metadata) => {
                if metadata.is_file() && metadata.len() >= MIN_VALID_DNA_FILE_SIZE {
                    logger.debug(format!("Found DNA file at {}", path.display()));
                    return Some(path.to_path_buf());
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

                match Self::extract_dna_from_contents(&contents, logger, lang) {
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

    fn extract_dna_from_contents(
        contents: &str,
        logger: &Logger,
        lang: &Language,
    ) -> Result<DnaInfo, String> {
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

        Err(translate(TextKey::DnaInfoNotFound, lang).to_string())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Language;

    fn logger() -> Logger {
        Logger::new("DnaTest")
    }

    #[test]
    fn parses_ch347_dna_output() {
        let contents = "\
Open On-Chip Debugger 0.12.0-rc3 (2024-01-26)
CH347 Open Succ
Info : starting gdb server for xc7.tap on pipe
DNA = 0011001000001110011000010011010101110100101101000010101 (0x00641CC26AE96854)
";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_ok(), "Should parse valid CH347 DNA output");
        let info = result.unwrap();
        assert_eq!(info.dna_value, "0x00641CC26AE96854");
        assert_eq!(
            info.dna_raw_value,
            "0011001000001110011000010011010101110100101101000010101"
        );
        assert_eq!(info.device_type, "CH347");
    }

    #[test]
    fn parses_ftdi_dna_output() {
        let contents = "\
Open On-Chip Debugger
Info : ftdi: initialized
DNA = 1100010010100000111010011100010100011001 (0xC4A0E9C519)
";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.device_type, "FTDI");
        assert_eq!(info.dna_value, "0xC4A0E9C519");
    }

    #[test]
    fn rejects_output_without_dna_line() {
        let contents = "Open On-Chip Debugger\nInfo : ftdi: initialized\nDone.\n";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_binary() {
        let contents = "DNA = 001122INVALID (0xABC)\n";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_hex() {
        let contents = "DNA = 0011 (NOTHEX)\n";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_err());
    }

    #[test]
    fn detects_unknown_device() {
        let contents = "DNA = 0011 (0xAB)\n";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().device_type, "Unknown");
    }

    #[test]
    fn handles_empty_input() {
        let result = DnaReader::extract_dna_from_contents("", &logger(), &Language::English);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_dna_line_without_parens() {
        let contents = "DNA = 001100100000\n";
        let result = DnaReader::extract_dna_from_contents(contents, &logger(), &Language::English);
        assert!(result.is_err());
    }

    #[test]
    fn stage_messages_are_non_empty() {
        let statuses = [
            CompletionStatus::NotCompleted,
            CompletionStatus::InProgress("working".into()),
            CompletionStatus::Completed,
            CompletionStatus::Failed("err".into()),
            CompletionStatus::DnaReadCompleted(DnaInfo {
                dna_value: "0x1".into(),
                dna_raw_value: "1".into(),
                device_type: "T".into(),
            }),
        ];
        for s in &statuses {
            let msg = DnaReader::get_dna_read_stage(s, &Language::English);
            assert!(!msg.is_empty(), "Stage for {s:?} should not be empty");
        }
    }

    #[test]
    fn failed_stage_contains_error() {
        let msg = DnaReader::get_dna_read_stage(
            &CompletionStatus::Failed("oops".into()),
            &Language::English,
        );
        assert!(
            msg.contains("oops"),
            "Failed stage should contain 'oops': {msg}"
        );
    }
}
