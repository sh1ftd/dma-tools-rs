pub mod dna;
mod firmware;
mod monitor;
mod operation;
mod process;
pub mod types;

use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

// Re-export the main types and functionality
pub use dna::DnaReader;
pub use firmware::FirmwareFlasher;
pub use operation::{FinalizationOutcome, FlashAssessment, OperationSnapshot, OperationStage};
pub use process::ProcessExecutor;
pub use types::{CompletionStatus, DnaInfo, FlashingOption};

use crate::utils::localization::Language;
use crate::utils::localization::{TextKey, translate};
use crate::utils::logger::Logger;
use monitor::OperationMonitor;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Windows-specific and configuration constants
pub const CREATE_NO_WINDOW: u32 = 0x08000000;
pub const TEMP_FIRMWARE_FILE: &str = "FIRMWARE.bin";
pub const DNA_OUTPUT_FILE: &str = "OpenOCD/openocd_output.log";
pub const SCRIPT_DIR: &str = ".";

/// Main manager class for flashing operations
pub struct FlashingManager {
    duration: Arc<Mutex<Option<Duration>>>,
    current_option: Option<FlashingOption>,
    logger: Logger,
    monitor: OperationMonitor,
    process_executor: ProcessExecutor,
    dna_reader: DnaReader,
    firmware_flasher: FirmwareFlasher,
    cleanup_enabled: bool,
    original_firmware_path: Option<PathBuf>,
    cleanup_done: Arc<AtomicBool>,
}

impl FlashingManager {
    pub fn new_with_logger(logger: Logger) -> Self {
        let monitor = OperationMonitor::new(logger.clone());
        let process_executor = ProcessExecutor::new(logger.clone());

        Self {
            duration: Arc::new(Mutex::new(None)),
            current_option: None,
            logger: logger.clone(),
            monitor,
            process_executor,
            dna_reader: DnaReader::new(logger.clone()),
            firmware_flasher: FirmwareFlasher::new(logger),
            cleanup_enabled: false,
            original_firmware_path: None,
            cleanup_done: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_cleanup_enabled(&mut self, enabled: bool) {
        self.cleanup_enabled = enabled;
    }

    pub fn cleanup_enabled(&self) -> bool {
        self.cleanup_enabled
    }

    pub fn execute_flash(
        &mut self,
        firmware_path: &Path,
        option: &FlashingOption,
        lang: &Language,
    ) {
        if let Err(error) = self.initialize_operation(option.clone(), lang) {
            let error = format!("Failed to initialize firmware operation: {error}");
            self.logger.error(&error);
            self.process_executor
                .set_completion_status(CompletionStatus::Failed(error));
            return;
        }
        self.original_firmware_path = Some(firmware_path.to_path_buf());

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

    pub fn execute_dna_read(&mut self, option: &FlashingOption, lang: &Language) {
        if let Err(error) = self.initialize_operation(option.clone(), lang) {
            let error = format!("Failed to initialize DNA operation: {error}");
            self.logger.error(&error);
            self.process_executor
                .set_completion_status(CompletionStatus::Failed(error));
            return;
        }
        self.dna_reader
            .execute(option, &self.process_executor, lang);
    }

    pub fn get_duration(&self) -> Option<Duration> {
        *self.duration.lock().unwrap()
    }

    pub fn stop_monitor_thread(&mut self) {
        self.monitor.stop_monitor_thread();
    }

    pub fn retire_for_restart(&mut self) -> Result<(), String> {
        self.monitor.stop_monitor_thread();
        self.process_executor.retire_for_restart()
    }

    #[cfg(test)]
    pub(crate) fn block_restart_for_test(&self, reason: &str) {
        self.process_executor.block_restart_for_test(reason);
    }

    // Private methods
    fn initialize_operation(
        &mut self,
        option: FlashingOption,
        lang: &Language,
    ) -> Result<(), String> {
        self.monitor.stop_monitor_thread();
        *self.duration.lock().unwrap() = None;
        self.current_option = Some(option.clone());
        self.monitor.reset_counters();
        self.cleanup_done.store(false, AtomicOrdering::SeqCst);

        // Clear all type/progress metadata before a fallible ownership reset so
        // an initialization error cannot be rendered or retried as the previous
        // operation's result.
        self.process_executor.reset()?;

        // Set an explicit in-progress status to prevent flashing
        let msg = translate(TextKey::StartingOperation, lang).to_string();

        self.process_executor
            .set_completion_status(CompletionStatus::InProgress(msg));
        Ok(())
    }

    #[cfg(test)]
    pub fn get_status(&self) -> CompletionStatus {
        self.process_executor.get_completion_status()
    }

    /// Returns true if the monitor detected too few normal sector writes
    /// (0ms/1ms writes indicate the data wasn't actually written).
    pub fn was_terminated_early(&self) -> bool {
        self.monitor.was_terminated_early()
    }

    pub fn snapshot(&self) -> OperationSnapshot {
        let (status, safe_to_restart) = self.process_executor.completion_snapshot();
        let progress = self.monitor.progress_snapshot();
        let terminated_early = self.monitor.was_terminated_early();
        let assessment = if self
            .current_option
            .as_ref()
            .is_some_and(FlashingOption::is_dna_read)
        {
            operation::FlashAssessment::NotApplicable
        } else {
            operation::assess_flash(&status, progress.sector_stats, terminated_early)
        };

        OperationSnapshot {
            status,
            safe_to_restart,
            option: self.current_option.clone(),
            stage: progress.stage,
            current_sector: progress.current_sector,
            sector_stats: progress.sector_stats,
            duration: self.get_duration(),
            assessment,
            terminated_early,
        }
    }

    pub fn finalize_completed_operation(&self) -> FinalizationOutcome {
        let snapshot = self.snapshot();
        let completed = matches!(
            snapshot.status,
            CompletionStatus::Completed
                | CompletionStatus::DnaReadCompleted(_)
                | CompletionStatus::Failed(_)
        );

        if !completed {
            return FinalizationOutcome::NotTerminal;
        }

        if snapshot
            .option
            .as_ref()
            .is_none_or(|option| !option.is_flash_operation())
        {
            return FinalizationOutcome::NotApplicable;
        }

        if !self.cleanup_enabled {
            return FinalizationOutcome::CleanupNotRequested {
                assessment: snapshot.assessment,
            };
        }

        if !snapshot.assessment.allows_source_cleanup() {
            return FinalizationOutcome::SourcePreserved {
                assessment: snapshot.assessment,
            };
        }

        let Some(path) = self.original_firmware_path.clone() else {
            return FinalizationOutcome::SourceUnavailable {
                assessment: snapshot.assessment,
            };
        };

        // Claim cleanup atomically so concurrent finalization calls cannot race.
        // A transient removal failure releases the claim for a later retry.
        if self
            .cleanup_done
            .compare_exchange(false, true, AtomicOrdering::AcqRel, AtomicOrdering::Acquire)
            .is_err()
        {
            return FinalizationOutcome::CleanupAlreadyCompleted;
        }

        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.logger.info(format!(
                    "Successfully cleaned up original firmware file: {}",
                    path.display()
                ));
                FinalizationOutcome::SourceRemoved { path }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.logger.debug(format!(
                    "Original firmware file was already removed: {}",
                    path.display()
                ));
                FinalizationOutcome::SourceAlreadyMissing { path }
            }
            Err(error) => {
                self.cleanup_done.store(false, AtomicOrdering::Release);
                self.logger.error(format!(
                    "Failed to clean up original firmware file: {error}"
                ));
                FinalizationOutcome::CleanupFailed {
                    path,
                    error: error.to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_firmware_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dma-tools-{name}-{nonce}.bin"))
    }

    fn record_normal_sectors(manager: &FlashingManager, count: usize) {
        let callback = manager.monitor.create_line_monitor(
            Logger::new("FinalizationTest"),
            manager.process_executor.process_terminator(),
        );
        for sector in 0..count {
            callback(&format!("Info : sector {sector} took 25 ms"));
        }
        manager.monitor.stop_monitor_thread();
    }

    #[test]
    fn successful_finalization_cleans_source_exactly_once() {
        let path = temporary_firmware_path("cleanup-success");
        fs::write(&path, b"firmware").unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::Completed);
        record_normal_sectors(&manager, 10);

        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourceRemoved { path: path.clone() }
        );
        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::CleanupAlreadyCompleted
        );

        assert!(!path.exists());
        assert!(manager.cleanup_done.load(AtomicOrdering::SeqCst));
    }

    #[test]
    fn finalization_retries_after_transient_removal_failure() {
        let path = temporary_firmware_path("cleanup-retry");
        fs::write(&path, b"firmware").unwrap();
        let locked_file = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::Completed);
        record_normal_sectors(&manager, 1);

        assert!(matches!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::CleanupFailed { path: failed_path, .. }
                if failed_path == path
        ));
        assert!(path.exists());
        assert!(!manager.cleanup_done.load(AtomicOrdering::SeqCst));

        drop(locked_file);
        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourceRemoved { path: path.clone() }
        );
        assert!(!path.exists());
        assert!(manager.cleanup_done.load(AtomicOrdering::SeqCst));
    }

    #[test]
    fn missing_source_is_treated_as_completed_cleanup() {
        let path = temporary_firmware_path("cleanup-missing");
        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::Completed);
        record_normal_sectors(&manager, 1);

        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourceAlreadyMissing { path: path.clone() }
        );

        assert!(manager.cleanup_done.load(AtomicOrdering::SeqCst));
    }

    #[test]
    fn failed_or_incomplete_finalization_preserves_source() {
        let path = temporary_firmware_path("cleanup-preserved");
        fs::write(&path, b"firmware").unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::InProgress("working".to_string()));

        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::NotTerminal
        );

        manager
            .process_executor
            .set_completion_status(CompletionStatus::Failed("failed".to_string()));
        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourcePreserved {
                assessment: FlashAssessment::Failed("failed".to_string())
            }
        );

        assert!(path.exists());
        assert!(!manager.cleanup_done.load(AtomicOrdering::SeqCst));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn indeterminate_completion_preserves_requested_cleanup_source() {
        let path = temporary_firmware_path("cleanup-indeterminate");
        fs::write(&path, b"firmware").unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::Completed);

        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourcePreserved {
                assessment: FlashAssessment::Indeterminate
            }
        );
        assert!(path.exists());
        assert!(!manager.cleanup_done.load(AtomicOrdering::SeqCst));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn unstable_connection_preserves_source_and_raw_process_failure() {
        let path = temporary_firmware_path("cleanup-unstable");
        fs::write(&path, b"firmware").unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::CH347_35T);
        manager.original_firmware_path = Some(path.clone());
        let process_failure = CompletionStatus::Failed("owned process terminated".to_string());
        manager
            .process_executor
            .set_completion_status(process_failure.clone());

        let callback = manager
            .monitor
            .create_line_monitor(Logger::new("FinalizationTest"), Arc::new(|| Ok(())));
        for sector in 0..10 {
            callback(&format!("Info : sector {sector} took 1 ms"));
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while !manager.monitor.was_terminated_early() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        manager.monitor.stop_monitor_thread();

        assert_eq!(manager.get_status(), process_failure);
        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::SourcePreserved {
                assessment: FlashAssessment::ConnectionUnstable {
                    normal_writes: 0,
                    total_sectors: 10,
                }
            }
        );
        assert!(path.exists());
        assert!(!manager.cleanup_done.load(AtomicOrdering::SeqCst));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn dna_completion_never_cleans_a_previous_firmware_source() {
        let path = temporary_firmware_path("cleanup-dna");
        fs::write(&path, b"firmware").unwrap();

        let mut manager = FlashingManager::new_with_logger(Logger::new("FinalizationTest"));
        manager.set_cleanup_enabled(true);
        manager.current_option = Some(FlashingOption::DnaCH347);
        manager.original_firmware_path = Some(path.clone());
        manager
            .process_executor
            .set_completion_status(CompletionStatus::Completed);

        assert_eq!(
            manager.finalize_completed_operation(),
            FinalizationOutcome::NotApplicable
        );

        assert!(path.exists());
        assert!(!manager.cleanup_done.load(AtomicOrdering::SeqCst));
        fs::remove_file(path).unwrap();
    }
}
