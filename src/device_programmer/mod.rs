pub mod dna;
mod firmware;
mod monitor;
mod process;
pub mod types;

// Re-export the main types and functionality
pub use dna::DnaReader;
pub use firmware::FirmwareFlasher;
pub use process::ProcessExecutor;
pub use types::{CompletionStatus, DnaInfo, FlashingOption};

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
    last_status_change: Arc<Mutex<Option<Instant>>>,
    last_status: Arc<Mutex<Option<CompletionStatus>>>,
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
            last_status_change: Arc::new(Mutex::new(Some(Instant::now()))),
            last_status: Arc::new(Mutex::new(None)),
        }
    }

    pub fn execute_flash(&mut self, firmware_path: &Path, option: &FlashingOption) {
        self.initialize_operation(option.clone());
        if let Err(e) = self.firmware_flasher.execute(
            firmware_path,
            option,
            &self.monitor,
            &self.process_executor,
            Arc::clone(&self.duration),
        ) {
            self.process_executor
                .set_completion_status(CompletionStatus::Failed(e));
        }
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

    pub fn stop_monitor_thread(&mut self) {
        self.monitor.stop_monitor_thread();
    }

    pub fn stop_dna_thread(&self) {
        self.dna_reader.stop_processing_thread();
    }

    pub fn get_last_status_change_time(&self) -> Option<Instant> {
        *self.last_status_change.lock().unwrap()
    }

    // Private methods
    fn initialize_operation(&mut self, option: FlashingOption) {
        self.monitor.stop_monitor_thread();

        self.start_time = Some(Instant::now());
        self.current_option = Some(option.clone());
        self.monitor.reset_counters();
        self.process_executor.reset();

        *self.last_status.lock().unwrap() = None;
        *self.last_status_change.lock().unwrap() = Some(Instant::now());

        // Set an explicit in-progress status to prevent flashing
        let is_dna_read = option.is_dna_read();
        if is_dna_read {
            self.process_executor
                .set_completion_status(CompletionStatus::InProgress(
                    "Starting DNA read operation...".to_string(),
                ));
        } else {
            self.process_executor
                .set_completion_status(CompletionStatus::InProgress(
                    "Starting flash operation...".to_string(),
                ));
        }
    }

    pub fn get_status(&self) -> CompletionStatus {
        let current_status = self.process_executor.get_completion_status();

        if self.monitor.was_terminated_early() {
            if matches!(current_status, CompletionStatus::Failed(_)) {
                CompletionStatus::Failed(
                    "Operation terminated early due to connection issues (check logs for details)"
                        .to_string(),
                )
            } else {
                CompletionStatus::Failed(
                    "Operation terminated early due to connection issues".to_string(),
                )
            }
        } else {
            current_status
        }
    }

    pub fn check_if_completed(&self) -> bool {
        matches!(
            self.get_status(),
            CompletionStatus::Completed
                | CompletionStatus::DnaReadCompleted(_)
                | CompletionStatus::Failed(_)
        )
    }
}
