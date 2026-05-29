use crate::device_programmer::monitor::OperationMonitor;
use crate::device_programmer::process::{CommandOptions, ProcessExecutor};
use crate::device_programmer::{CompletionStatus, FlashingOption, SCRIPT_DIR, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct FirmwareFlasher {
    logger: Logger,
}

impl FirmwareFlasher {
    pub fn new(logger: Logger) -> Self {
        Self { logger }
    }

    pub fn execute(
        &self,
        firmware_path: &Path,
        option: &FlashingOption,
        monitor: &OperationMonitor,
        executor: &ProcessExecutor,
        duration: Arc<Mutex<Option<Duration>>>,
    ) -> Result<(), String> {
        // Copy firmware to temporary location
        self.copy_firmware_to_temp(firmware_path, executor)?;

        // Create the command
        let (command, command_str) = self.create_flash_command(option);

        // Log operation information
        self.log_flash_operation(firmware_path, option, &command_str);

        // Execute and track the operation
        self.run_flash_operation(command, monitor, executor, duration)
    }

    fn create_flash_command(&self, option: &FlashingOption) -> (std::process::Command, String) {
        let (exe_path, command_str, args) = self.prepare_flash_command(option);
        let command = ProcessExecutor::prepare_command(
            &exe_path,
            &args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
        );
        (command, command_str)
    }

    fn run_flash_operation(
        &self,
        command: std::process::Command,
        monitor: &OperationMonitor,
        executor: &ProcessExecutor,
        duration: Arc<Mutex<Option<Duration>>>,
    ) -> Result<(), String> {
        // Create a monitor callback (only pass the logger)
        let monitor_callback = monitor.create_line_monitor(self.logger.clone());

        // Execute the command and get the child process
        match executor.execute_command(
            command,
            Some(monitor_callback),
            CommandOptions {
                log_duration: true,
                cleanup_temp_files: true,
                duration_target: Some(Arc::clone(&duration)),
            },
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to execute firmware flash: {e}");
                self.logger.error(&error_msg);
                Err(error_msg)
            }
        }
    }

    fn log_flash_operation(
        &self,
        firmware_path: &Path,
        option: &FlashingOption,
        command_str: &str,
    ) {
        self.logger.info(format!(
            "Starting firmware flash operation with option: {}",
            option.get_display_name()
        ));
        self.logger
            .info(format!("Firmware file: {}", firmware_path.display()));
        self.logger.command(format!("Executing: {command_str}"));
    }

    fn prepare_flash_command(&self, option: &FlashingOption) -> (String, String, Vec<String>) {
        let (cmd, config) = option.get_command_args();
        let exe_path = format!("{SCRIPT_DIR}/{cmd}");
        let config_path = format!("{SCRIPT_DIR}/{config}");

        let program_arg = format!("program {TEMP_FIRMWARE_FILE}; exit");
        let args = vec![
            "-f".to_string(),
            config_path.clone(),
            "-c".to_string(),
            program_arg.clone(),
        ];

        // For logging purposes
        let command_str = format!("{exe_path} -f {config_path} -c \"{program_arg}\"");

        (exe_path, command_str, args)
    }

    fn copy_firmware_to_temp(
        &self,
        firmware_path: &Path,
        executor: &ProcessExecutor,
    ) -> Result<(), String> {
        fs::copy(firmware_path, TEMP_FIRMWARE_FILE)
            .map(|_| ())
            .map_err(|e| {
                let error_msg = format!("Failed to prepare firmware file: {e}");
                self.logger.error(&error_msg);
                executor.set_completion_status(CompletionStatus::Failed(error_msg.clone()));
                error_msg
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_command_programs_prepared_temp_firmware() {
        let flasher = FirmwareFlasher::new(Logger::new("FirmwareFlasherTest"));
        let (_, command_str, args) = flasher.prepare_flash_command(&FlashingOption::CH347_35T);

        assert_eq!(args[3], format!("program {TEMP_FIRMWARE_FILE}; exit"));
        assert!(command_str.contains(&format!("program {TEMP_FIRMWARE_FILE}; exit")));
    }
}
