mod dna;
mod firmware;
mod monitor;
mod process;
mod types;

// Re-export the main types and functionality
pub use dna::DnaReader;
pub use firmware::FirmwareFlasher;
pub use process::ProcessExecutor;
pub use types::{CompletionStatus, FlashingOption};

use crate::utils::logger::Logger;
use monitor::OperationMonitor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Windows-specific and configuration constants
pub const CREATE_NO_WINDOW: u32 = 0x08000000;
pub const TEMP_FIRMWARE_FILE: &str = "FIRMWARE.bin";
pub const DNA_OUTPUT_FILE: &str = "OpenOCD/openocd_output.log";
pub const SCRIPT_DIR: &str = ".";

/// Main manager class for flashing operations
pub struct FlashingManager {
    start_time: Option<Instant>,
    duration: Arc<Mutex<Option<Duration>>>,
    current_option: Option<FlashingOption>,
    logger: Logger,
    monitor: OperationMonitor,
    process_executor: ProcessExecutor,
    dna_reader: DnaReader,
    firmware_flasher: FirmwareFlasher,
}

impl FlashingManager {
    pub fn new_with_logger(logger: Logger) -> Self {
        let monitor = OperationMonitor::new();
        let process_executor = ProcessExecutor::new(logger.clone());

        Self {
            start_time: None,
            duration: Arc::new(Mutex::new(None)),
            current_option: None,
            logger: logger.clone(),
            monitor,
            process_executor,
            dna_reader: DnaReader::new(logger.clone()),
            firmware_flasher: FirmwareFlasher::new(logger),
        }
    }

    pub fn is_completed(&self) -> bool {
        self.get_completion_status() != CompletionStatus::NotCompleted
    }

    pub fn get_completion_status(&self) -> CompletionStatus {
        if self.monitor.was_terminated_early() {
            return CompletionStatus::Failed(
                "Operation terminated early due to connection issues (too many quick sector writes detected)".to_string()
            );
        }

        self.process_executor.get_completion_status()
    }

    pub fn execute_flash(&mut self, firmware_path: &Path, option: &FlashingOption) {
        self.initialize_operation(option.clone());
        self.firmware_flasher.execute(
            firmware_path,
            option,
            &self.monitor,
            &self.process_executor,
            Arc::clone(&self.duration),
        );
    }

    pub fn execute_dna_read(&mut self, option: &FlashingOption) {
        self.initialize_operation(option.clone());
        self.dna_reader.execute(option, &self.process_executor);
    }

    pub fn get_duration(&self) -> Option<Duration> {
        *self.duration.lock().unwrap()
    }

    pub fn get_current_option(&self) -> Option<&FlashingOption> {
        self.current_option.as_ref()
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    #[allow(dead_code)]
    pub fn set_auto_terminate(&self, enabled: bool) {
        self.monitor.set_auto_terminate(enabled);
    }

    #[allow(dead_code)]
    pub fn get_output_log(&self) -> Vec<String> {
        self.logger
            .get_entries()
            .iter()
            .map(|entry| entry.message.clone())
            .collect()
    }

    // Private methods
    fn initialize_operation(&mut self, option: FlashingOption) {
        self.start_time = Some(Instant::now());
        self.current_option = Some(option);
        self.monitor.reset_counters();
        self.process_executor.reset();
    }
}
