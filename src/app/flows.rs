use crate::device_programmer::{FlashingManager, FlashingOption};
use crate::utils::file_checker::FileChecker;
use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::logger::Logger;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RetryPlan {
    ReadDna(FlashingOption),
    Flash {
        option: FlashingOption,
        firmware: PathBuf,
    },
    ReselectFirmware,
    NothingSelected,
}

pub(super) struct FileCheckFlow {
    pub(super) checker: FileChecker,
    pub(super) started: bool,
    pub(super) app_started_at: Instant,
    pub(super) success_display_at: Option<Instant>,
}

impl FileCheckFlow {
    pub(super) fn new() -> Self {
        Self {
            checker: FileChecker::new(),
            started: false,
            app_started_at: Instant::now(),
            success_display_at: None,
        }
    }

    pub(super) fn reset(&mut self) {
        self.checker = FileChecker::new();
        self.started = false;
        self.success_display_at = None;
    }
}

pub(super) struct FirmwareScanFlow {
    pub(super) manager: FirmwareManager,
    pub(super) last_scan: Instant,
    pub(super) scanning: bool,
}

impl FirmwareScanFlow {
    pub(super) fn new() -> Self {
        Self {
            manager: FirmwareManager::new(),
            last_scan: Instant::now(),
            scanning: false,
        }
    }

    pub(super) fn mark_scan_started(&mut self) {
        self.last_scan = Instant::now();
        self.scanning = true;
    }
}

pub(super) struct OperationFlow {
    pub(super) manager: FlashingManager,
    pub(super) selected_firmware: Option<PathBuf>,
    pub(super) selected_option: Option<FlashingOption>,
    pub(super) dna_started_at: Option<Instant>,
    pub(super) dna_in_progress: bool,
    pub(super) waiting_message_logged: bool,
    pub(super) retry_attempt: u32,
    pub(super) retry_cooldown_started_at: Option<Instant>,
    pub(super) cleanup_retry_attempt: u32,
    pub(super) cleanup_retry_ready_at: Option<Instant>,
}

impl OperationFlow {
    pub(super) fn new(logger: Logger) -> Self {
        Self {
            manager: FlashingManager::new_with_logger(logger),
            selected_firmware: None,
            selected_option: None,
            dna_started_at: None,
            dna_in_progress: false,
            waiting_message_logged: false,
            retry_attempt: 0,
            retry_cooldown_started_at: None,
            cleanup_retry_attempt: 0,
            cleanup_retry_ready_at: None,
        }
    }

    pub(super) fn replace_manager(&mut self, logger: Logger) -> Result<(), String> {
        let cleanup_enabled = self.manager.cleanup_enabled();
        self.manager.retire_for_restart()?;
        self.manager = FlashingManager::new_with_logger(logger);
        self.manager.set_cleanup_enabled(cleanup_enabled);
        self.reset_cleanup_retry();
        Ok(())
    }

    pub(super) fn set_cleanup_enabled(&mut self, enabled: bool) {
        self.manager.set_cleanup_enabled(enabled);
    }

    pub(super) fn retry_plan(&self) -> RetryPlan {
        let Some(option) = self.selected_option.clone() else {
            return RetryPlan::NothingSelected;
        };

        if option.is_dna_read() {
            return RetryPlan::ReadDna(option);
        }

        match self.selected_firmware.as_ref() {
            Some(firmware) if firmware.is_file() => RetryPlan::Flash {
                option,
                firmware: firmware.clone(),
            },
            _ => RetryPlan::ReselectFirmware,
        }
    }

    pub(super) fn reset_retry(&mut self) {
        self.retry_attempt = 0;
        self.retry_cooldown_started_at = None;
        self.reset_cleanup_retry();
    }

    pub(super) fn reset_cleanup_retry(&mut self) {
        self.cleanup_retry_attempt = 0;
        self.cleanup_retry_ready_at = None;
    }

    pub(super) fn clear_selection(&mut self) {
        self.selected_firmware = None;
        self.selected_option = None;
        self.dna_started_at = None;
        self.dna_in_progress = false;
        self.waiting_message_logged = false;
        self.set_cleanup_enabled(false);
        self.reset_retry();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn temporary_firmware_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dma-tools-flow-{nonce}.bin"))
    }

    #[test]
    fn file_check_reset_clears_transient_state() {
        let mut flow = FileCheckFlow::new();
        flow.started = true;
        flow.success_display_at = Some(Instant::now());

        flow.reset();

        assert!(!flow.started);
        assert!(flow.success_display_at.is_none());
    }

    #[test]
    fn operation_retry_reset_preserves_selection() {
        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.selected_option = Some(FlashingOption::DnaCH347);
        flow.retry_attempt = 3;
        flow.retry_cooldown_started_at = Some(Instant::now());

        flow.reset_retry();

        assert_eq!(flow.selected_option, Some(FlashingOption::DnaCH347));
        assert_eq!(flow.retry_attempt, 0);
        assert!(flow.retry_cooldown_started_at.is_none());
    }

    #[test]
    fn clearing_operation_selection_also_resets_retry() {
        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.selected_option = Some(FlashingOption::CH347_35T);
        flow.selected_firmware = Some(PathBuf::from("firmware.bin"));
        flow.retry_attempt = 2;
        flow.cleanup_retry_attempt = 2;
        flow.cleanup_retry_ready_at = Some(Instant::now());
        flow.dna_started_at = Some(Instant::now());
        flow.dna_in_progress = true;
        flow.waiting_message_logged = true;
        flow.set_cleanup_enabled(true);

        flow.clear_selection();

        assert!(flow.selected_option.is_none());
        assert!(flow.selected_firmware.is_none());
        assert!(flow.dna_started_at.is_none());
        assert!(!flow.dna_in_progress);
        assert!(!flow.waiting_message_logged);
        assert_eq!(flow.retry_attempt, 0);
        assert_eq!(flow.cleanup_retry_attempt, 0);
        assert!(flow.cleanup_retry_ready_at.is_none());
        assert!(!flow.manager.cleanup_enabled());
    }

    #[test]
    fn manager_replacement_preserves_cleanup_preference() {
        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.set_cleanup_enabled(true);

        flow.replace_manager(Logger::new("ReplacementManagerTest"))
            .unwrap();

        assert!(flow.manager.cleanup_enabled());
    }

    #[test]
    fn blocked_manager_replacement_preserves_the_existing_manager() {
        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.set_cleanup_enabled(true);
        flow.selected_option = Some(FlashingOption::DnaCH347);
        flow.manager
            .block_restart_for_test("synthetic unconfirmed cleanup");

        let error = flow
            .replace_manager(Logger::new("ReplacementManagerTest"))
            .expect_err("unsafe cleanup must reject manager replacement");

        assert!(error.contains("synthetic unconfirmed cleanup"));
        assert!(!flow.manager.snapshot().safe_to_restart);
        assert!(flow.manager.cleanup_enabled());
        assert_eq!(flow.selected_option, Some(FlashingOption::DnaCH347));
    }

    #[test]
    fn retry_plan_requires_reselection_after_cleanup_deleted_firmware() {
        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.selected_option = Some(FlashingOption::CH347_35T);
        flow.selected_firmware = Some(temporary_firmware_path());

        assert_eq!(flow.retry_plan(), RetryPlan::ReselectFirmware);
    }

    #[test]
    fn retry_plan_preserves_available_flash_and_dna_operations() {
        let path = temporary_firmware_path();
        fs::write(&path, b"firmware").unwrap();

        let mut flow = OperationFlow::new(Logger::new("OperationFlowTest"));
        flow.selected_option = Some(FlashingOption::CH347_35T);
        flow.selected_firmware = Some(path.clone());
        assert_eq!(
            flow.retry_plan(),
            RetryPlan::Flash {
                option: FlashingOption::CH347_35T,
                firmware: path.clone(),
            }
        );

        flow.selected_option = Some(FlashingOption::DnaCH347);
        assert_eq!(
            flow.retry_plan(),
            RetryPlan::ReadDna(FlashingOption::DnaCH347)
        );

        fs::remove_file(path).unwrap();
    }
}
