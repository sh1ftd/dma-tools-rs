use crate::device_programmer::monitor::OperationMonitor;
use crate::device_programmer::process::ProcessExecutor;
use crate::device_programmer::{FlashingOption, SCRIPT_DIR, TEMP_FIRMWARE_FILE};
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
    ) {
        // Copy firmware to temporary location
        if let Err(e) = fs::copy(firmware_path, TEMP_FIRMWARE_FILE) {
            let error_msg = format!("Failed to prepare firmware file: {}", e);
            self.logger.error(&error_msg);
            executor.set_completion_status(crate::device_programmer::CompletionStatus::Failed(
                error_msg,
            ));
            return;
        }

        self.logger.info(format!(
            "Starting firmware flash operation with option: {}",
            option.get_display_name()
        ));
        self.logger
            .info(format!("Firmware file: {}", firmware_path.display()));

        let (cmd, config) = option.get_command_args();
        let command_str = format!(
            "{}/{} -f {}/{} -c \"program {}; exit\"",
            SCRIPT_DIR,
            cmd,
            SCRIPT_DIR,
            config,
            firmware_path.display()
        );
        self.logger.command(format!("Executing: {}", command_str));

        let exe_path = format!("{}/{}", SCRIPT_DIR, cmd);
        let config_path = format!("{}/{}", SCRIPT_DIR, config);

        // Create the command
        let command = ProcessExecutor::prepare_command(
            &exe_path,
            &[
                "-f",
                &config_path,
                "-c",
                &format!("program {}; exit", firmware_path.display()),
            ],
        );

        // Track the child process for possible termination
        let child_process = Arc::new(Mutex::new(None::<u32>));

        // Create a monitor callback
        let monitor_callback =
            monitor.create_line_monitor(self.logger.clone(), Arc::clone(&child_process));

        // Execute the command
        if let Err(e) = executor.execute_command(
            command,
            Some(monitor_callback),
            true, // update duration
            true, // cleanup temp file
        ) {
            self.logger
                .error(format!("Failed to execute firmware flash: {}", e));
        } else {
            // Make sure to update the shared duration from the executor's timing
            if let Some(start) = *executor.get_start_time().lock().unwrap() {
                let elapsed = start.elapsed();
                *duration.lock().unwrap() = Some(elapsed);
            }
        }
    }
}
