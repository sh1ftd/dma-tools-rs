use super::output::{self, OutputGeneration, OutputWaitError};
use super::parser::DnaParseError;
use crate::device_programmer::process::{CommandOptions, ProcessExecutor};
use crate::device_programmer::{CompletionStatus, DNA_OUTPUT_FILE, FlashingOption, SCRIPT_DIR};
use crate::utils::localization::{Language, TextKey, format_translation, translate};
use crate::utils::logger::Logger;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const DNA_RETRY_WAIT: Duration = Duration::from_millis(200);
const DNA_MAX_ATTEMPTS: usize = 5;

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
            let message = translate(TextKey::DnaInvalidOption, lang);
            self.logger.error(message);
            executor.set_completion_status(CompletionStatus::Failed(message.to_string()));
            return;
        }

        executor.set_completion_status(CompletionStatus::InProgress(
            translate(TextKey::Initializing, lang).to_string(),
        ));

        let output_path = Path::new(DNA_OUTPUT_FILE);
        let output_generation = match output::prepare_output_file(output_path, &self.logger) {
            Ok(generation) => generation,
            Err(error) => {
                let native_message = format!(
                    "Failed to prepare DNA output file at {}: {error}",
                    output_path.display()
                );
                let message = localized_file_error(output_path, &error, lang);
                self.logger.error(native_message);
                executor.set_completion_status(CompletionStatus::Failed(message));
                return;
            }
        };

        let (command, config) = option.get_command_args();
        let executable_path = format!("{SCRIPT_DIR}/{command}");
        let config_path = format!("{SCRIPT_DIR}/{config}");
        let parse_callback = self.create_parse_callback(executor, lang, output_generation);

        if !self.run_command(&executable_path, &config_path, executor, parse_callback) {
            self.stop_output_parsing();
            executor.set_completion_status(CompletionStatus::Failed(
                translate(TextKey::DnaCommandFailed, lang).to_string(),
            ));
        }
    }

    fn run_command(
        &self,
        executable_path: &str,
        config_path: &str,
        executor: &ProcessExecutor,
        parse_callback: Box<dyn FnOnce(bool) + Send + 'static>,
    ) -> bool {
        let command =
            ProcessExecutor::prepare_command(executable_path, &["-f", config_path, "-c", "exit"]);
        self.logger
            .debug(format!("Executing DNA read command: {command:?}"));

        executor
            .execute_command(
                command,
                None,
                CommandOptions {
                    log_duration: true,
                    cleanup_temp_files: false,
                    duration_target: None,
                    on_complete: Some(parse_callback),
                },
            )
            .inspect_err(|error| {
                self.logger
                    .error(format!("Failed to execute DNA read: {error}"));
            })
            .is_ok()
    }

    fn stop_output_parsing(&self) {
        self.parse_enabled.store(false, Ordering::SeqCst);
        self.logger.debug("DNA output parsing stop requested");
    }

    fn create_parse_callback(
        &self,
        executor: &ProcessExecutor,
        lang: &Language,
        output_generation: OutputGeneration,
    ) -> Box<dyn FnOnce(bool) + Send + 'static> {
        let language = *lang;
        let logger = self.logger.clone();
        let completion_status = executor.get_completion_status_arc();
        let parse_enabled = Arc::clone(&self.parse_enabled);
        parse_enabled.store(true, Ordering::SeqCst);

        Box::new(move |command_succeeded| {
            if !command_succeeded {
                logger.warning("DNA command failed before output parsing could run.");
                *completion_status.lock().unwrap() = CompletionStatus::Failed(
                    translate(TextKey::DnaCommandFailed, &language).to_string(),
                );
                parse_enabled.store(false, Ordering::SeqCst);
                return;
            }

            if !parse_enabled.load(Ordering::SeqCst) {
                logger.warning("DNA output parsing was stopped before processing");
                *completion_status.lock().unwrap() = CompletionStatus::Failed(
                    translate(TextKey::DnaCommandFailed, &language).to_string(),
                );
                return;
            }

            *completion_status.lock().unwrap() = CompletionStatus::InProgress(
                translate(TextKey::DnaRetrieving, &language).to_string(),
            );

            let output_path = Path::new(DNA_OUTPUT_FILE);
            match output::wait_for_parsed_output(
                output_path,
                output_generation,
                DNA_MAX_ATTEMPTS,
                DNA_RETRY_WAIT,
                &logger,
            ) {
                Ok(dna_info) => {
                    logger.info(format!(
                        "DNA read completed successfully: {}",
                        dna_info.dna_value
                    ));
                    *completion_status.lock().unwrap() =
                        CompletionStatus::DnaReadCompleted(dna_info);
                }
                Err(error) => {
                    let message = localized_output_error(output_path, &error, &language);
                    logger.error(format!(
                        "Failed to obtain valid DNA output after {DNA_MAX_ATTEMPTS} attempts: {error}"
                    ));
                    *completion_status.lock().unwrap() = CompletionStatus::Failed(message);
                }
            }

            parse_enabled.store(false, Ordering::SeqCst);
        })
    }
}

fn localized_parse_error(error: DnaParseError, lang: &Language) -> String {
    match error {
        DnaParseError::InformationNotFound => translate(TextKey::DnaInfoNotFound, lang).to_string(),
    }
}

fn localized_output_error(path: &Path, error: &OutputWaitError, lang: &Language) -> String {
    match error {
        OutputWaitError::Unavailable => {
            let attempts = DNA_MAX_ATTEMPTS.to_string();
            format_translation(translate(TextKey::DnaFileNotFound, lang), &[&attempts])
        }
        OutputWaitError::Malformed(error) => {
            let detail = localized_parse_error(*error, lang);
            format_translation(translate(TextKey::DnaExtractFailed, lang), &[&detail])
        }
        OutputWaitError::Read(error) => localized_file_error(path, error, lang),
        OutputWaitError::Stale => localized_file_error(path, error, lang),
    }
}

fn localized_file_error(path: &Path, error: &dyn std::fmt::Display, lang: &Language) -> String {
    format_file_error(
        translate(TextKey::DnaFileReadError, lang),
        &path.to_string_lossy(),
        &error.to_string(),
    )
}

fn format_file_error(template: &str, path: &str, error: &str) -> String {
    format_translation(template, &[path, error])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn file_error_formatting_replaces_path_and_error_independently() {
        let message = format_file_error(
            "Failed to read DNA output file at {}: {}",
            "OpenOCD/openocd_output.log",
            "access denied",
        );

        assert_eq!(
            message,
            "Failed to read DNA output file at OpenOCD/openocd_output.log: access denied"
        );
    }

    #[test]
    fn localized_file_error_contains_both_details() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let message = localized_file_error(
            Path::new("OpenOCD/openocd_output.log"),
            &error,
            &Language::English,
        );

        assert!(message.contains("OpenOCD/openocd_output.log"));
        assert!(message.contains("access denied"));
    }

    #[test]
    fn arabic_file_error_contains_path_and_native_error() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let message = localized_file_error(
            Path::new("OpenOCD/openocd_output.log"),
            &error,
            &Language::Arabic,
        );

        assert!(message.contains("OpenOCD/openocd_output.log"));
        assert!(message.contains("access denied"));
        assert!(!message.contains("{}"));
        assert!(!message.contains("}{"));
    }

    #[test]
    fn arabic_unavailable_error_contains_attempt_count() {
        let message = localized_output_error(
            Path::new("OpenOCD/openocd_output.log"),
            &OutputWaitError::Unavailable,
            &Language::Arabic,
        );

        assert!(message.contains(&DNA_MAX_ATTEMPTS.to_string()));
        assert!(!message.contains("{}"));
        assert!(!message.contains("}{"));
    }

    #[test]
    fn arabic_malformed_error_contains_parse_detail() {
        let detail = localized_parse_error(DnaParseError::InformationNotFound, &Language::Arabic);
        let message = localized_output_error(
            Path::new("OpenOCD/openocd_output.log"),
            &OutputWaitError::Malformed(DnaParseError::InformationNotFound),
            &Language::Arabic,
        );

        assert!(message.contains(&detail));
        assert!(!message.contains("{}"));
        assert!(!message.contains("}{"));
    }

    #[test]
    fn localized_template_does_not_reparse_braces_inside_inserted_values() {
        let message = format_translation("first {} second {}", &["a{}b", "error"]);

        assert_eq!(message, "first a{}b second error");
    }

    #[test]
    fn command_failure_callback_sets_localized_status() {
        let logger = Logger::new("DnaReaderTest");
        let reader = DnaReader::new(logger.clone());
        let executor = ProcessExecutor::new(logger);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dma-tools-dna-callback-{nonce}.log"));
        let generation = output::prepare_output_file(&path, &Logger::new("DnaReaderTest")).unwrap();
        let callback = reader.create_parse_callback(&executor, &Language::German, generation);

        callback(false);

        assert_eq!(
            executor.get_completion_status(),
            CompletionStatus::Failed(
                translate(TextKey::DnaCommandFailed, &Language::German).to_string()
            )
        );
    }
}
