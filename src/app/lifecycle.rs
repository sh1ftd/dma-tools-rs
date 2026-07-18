use super::flows::RetryPlan;
use super::{AppState, FirmwareToolApp};
use crate::device_programmer::{
    CompletionStatus, FinalizationOutcome, FlashAssessment, OperationSnapshot,
};
use crate::ui::status::ResultAction;
use crate::utils::file_checker::{CheckStatus, SUCCESS_TRANSITION_DELAY};
use eframe::egui;
use std::thread;
use std::time::{Duration, Instant};

const ANIMATION_FRAME_RATE_MS: u64 = 16;
const INITIAL_CHECK_DELAY_MS: u128 = 100;
const FIRST_FIRMWARE_SCAN_INTERVAL_MS: u64 = 100;
const SUBSEQUENT_FIRMWARE_SCAN_INTERVAL_MS: u64 = 3000;
const FIRMWARE_SCAN_INDICATOR_DURATION_MS: u128 = 500;
const DNA_MIN_DISPLAY_TIME_MS: u64 = 100;

const MAX_AUTO_RETRIES: u32 = 10;
const RETRY_COOLDOWN_MS: u64 = 2500;
const MAX_CLEANUP_RETRIES: u32 = 3;
const CLEANUP_RETRY_BASE_DELAY_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultTone {
    Success,
    Error,
    None,
}

fn is_terminal_status(status: &CompletionStatus) -> bool {
    matches!(
        status,
        CompletionStatus::Completed
            | CompletionStatus::DnaReadCompleted(_)
            | CompletionStatus::Failed(_)
    )
}

fn result_tone(
    status: &CompletionStatus,
    is_dna_read: bool,
    assessment: &FlashAssessment,
) -> ResultTone {
    if is_dna_read {
        return match status {
            CompletionStatus::DnaReadCompleted(_) => ResultTone::Success,
            CompletionStatus::Completed | CompletionStatus::Failed(_) => ResultTone::Error,
            CompletionStatus::NotCompleted | CompletionStatus::InProgress(_) => ResultTone::None,
        };
    }

    match assessment {
        FlashAssessment::Success | FlashAssessment::SuccessWithLimitedSamples { .. } => {
            ResultTone::Success
        }
        FlashAssessment::ConnectionUnstable { .. }
        | FlashAssessment::Indeterminate
        | FlashAssessment::Failed(_)
        | FlashAssessment::UnexpectedDnaResult
        | FlashAssessment::NotApplicable => ResultTone::Error,
        FlashAssessment::Pending => ResultTone::None,
    }
}

fn cleanup_retry_delay(retry_attempt: u32) -> Duration {
    let exponent = retry_attempt.saturating_sub(1).min(16);
    Duration::from_millis(CLEANUP_RETRY_BASE_DELAY_MS.saturating_mul(1_u64 << exponent))
}

fn next_cleanup_retry(completed_retries: u32) -> Option<(u32, Duration)> {
    if completed_retries >= MAX_CLEANUP_RETRIES {
        return None;
    }

    let retry_attempt = completed_retries + 1;
    Some((retry_attempt, cleanup_retry_delay(retry_attempt)))
}

impl FirmwareToolApp {
    pub(super) fn setup_ui_and_animation(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(Duration::from_millis(ANIMATION_FRAME_RATE_MS));
    }

    pub(super) fn update_window_size(&mut self, ctx: &egui::Context) {
        let window_size_type = self.get_window_size_type();
        self.window_manager.set_window_size(ctx, window_size_type);
    }

    pub(super) fn handle_state_specific_logic(&mut self, ctx: &egui::Context) {
        self.reset_waiting_message_outside_flashing();
        self.stop_monitor_when_not_showing_operation();
        self.maybe_start_file_check();
        self.maybe_update_firmware_scan(ctx);
        self.maybe_transition_completed_operation();

        #[cfg(feature = "branding")]
        self.branding_manager.ensure_loaded(ctx);
    }

    fn reset_waiting_message_outside_flashing(&mut self) {
        if self.state != AppState::Flashing {
            self.operation.waiting_message_logged = false;
        }
    }

    fn stop_monitor_when_not_showing_operation(&mut self) {
        if self.state != AppState::Result && self.state != AppState::Flashing {
            self.operation.manager.stop_monitor_thread();
        }
    }

    fn maybe_start_file_check(&mut self) {
        if !self.file_check.started
            && self.file_check.app_started_at.elapsed().as_millis() > INITIAL_CHECK_DELAY_MS
        {
            self.file_check.checker.start_check();
            self.file_check.started = true;
        }
    }

    fn maybe_update_firmware_scan(&mut self, ctx: &egui::Context) {
        if self.state == AppState::FirmwareSelection {
            self.handle_firmware_scanning(ctx);
        }
    }

    fn maybe_transition_completed_operation(&mut self) {
        if self.state != AppState::Flashing {
            return;
        }

        // Process output readers are drained before a terminal status becomes visible, so this
        // snapshot is the stable source for result classification, cleanup eligibility, and sound.
        let snapshot = self.operation.manager.snapshot();

        let min_display_time_elapsed = self
            .operation
            .dna_started_at
            .is_none_or(|t| t.elapsed() > Duration::from_millis(DNA_MIN_DISPLAY_TIME_MS));

        let operation_completed = is_terminal_status(&snapshot.status);

        if operation_completed && !self.cleanup_ready_for_transition(&snapshot) {
            return;
        }

        if operation_completed && min_display_time_elapsed {
            if self.maybe_auto_retry_flash(&snapshot) {
                return;
            }

            self.operation.manager.stop_monitor_thread();
            self.transition_to_result(&snapshot);
        } else if operation_completed && !self.operation.waiting_message_logged {
            self.logger
                .debug("Operation completed but waiting for minimum display time");
            self.operation.waiting_message_logged = true;
        }
    }

    fn cleanup_ready_for_transition(&mut self, snapshot: &OperationSnapshot) -> bool {
        // Cleanup is promised only for a successful flash. Never delete the source for an
        // indeterminate, failed, DNA, or otherwise unexpected terminal assessment.
        if !snapshot.assessment.allows_source_cleanup() {
            self.operation.reset_cleanup_retry();
            return true;
        }

        if self
            .operation
            .cleanup_retry_ready_at
            .is_some_and(|ready_at| Instant::now() < ready_at)
        {
            return false;
        }

        match self.operation.manager.finalize_completed_operation() {
            FinalizationOutcome::NotTerminal => false,
            FinalizationOutcome::CleanupFailed { path, error } => {
                if let Some((retry_attempt, delay)) =
                    next_cleanup_retry(self.operation.cleanup_retry_attempt)
                {
                    self.operation.cleanup_retry_attempt = retry_attempt;
                    self.operation.cleanup_retry_ready_at = Some(Instant::now() + delay);
                    self.logger.warning(format!(
                        "Could not clean up {}: {error}. Retrying in {}ms ({}/{}).",
                        path.display(),
                        delay.as_millis(),
                        retry_attempt,
                        MAX_CLEANUP_RETRIES
                    ));
                    false
                } else {
                    self.logger.error(format!(
                        "Could not clean up {} after {} attempts: {error}. The source firmware was not removed.",
                        path.display(),
                        MAX_CLEANUP_RETRIES + 1
                    ));
                    self.operation.reset_cleanup_retry();
                    true
                }
            }
            FinalizationOutcome::NotApplicable
            | FinalizationOutcome::CleanupNotRequested { .. }
            | FinalizationOutcome::SourcePreserved { .. }
            | FinalizationOutcome::SourceRemoved { .. }
            | FinalizationOutcome::SourceAlreadyMissing { .. }
            | FinalizationOutcome::CleanupAlreadyCompleted => {
                self.operation.reset_cleanup_retry();
                true
            }
            FinalizationOutcome::SourceUnavailable { .. } => {
                self.logger.error(
                    "Cleanup was requested after a successful flash, but the original firmware path is unavailable. The source firmware was not removed.",
                );
                self.operation.reset_cleanup_retry();
                true
            }
        }
    }

    fn maybe_auto_retry_flash(&mut self, snapshot: &OperationSnapshot) -> bool {
        if !snapshot.safe_to_restart
            || !(self.operation.manager.was_terminated_early()
                && self.operation.retry_attempt < MAX_AUTO_RETRIES
                && !self.operation.dna_in_progress)
        {
            return false;
        }

        if self.operation.retry_cooldown_started_at.is_none() {
            self.operation.retry_attempt += 1;
            self.logger.info(format!(
                "Connection unstable — retrying automatically (attempt {}/{})",
                self.operation.retry_attempt, MAX_AUTO_RETRIES
            ));
            self.operation.retry_cooldown_started_at = Some(Instant::now());
            return true;
        }

        if let Some(cooldown_start) = self.operation.retry_cooldown_started_at
            && cooldown_start.elapsed() < Duration::from_millis(RETRY_COOLDOWN_MS)
        {
            let remaining_ms =
                RETRY_COOLDOWN_MS.saturating_sub(cooldown_start.elapsed().as_millis() as u64);
            self.logger
                .debug(format!("Retry cooldown: {}ms remaining", remaining_ms));
            return true;
        }

        self.operation.retry_cooldown_started_at = None;
        if let Err(error) = self.operation.replace_manager(self.logger.clone()) {
            self.logger.error(format!(
                "Automatic retry blocked because process retirement was not confirmed: {error}"
            ));
            return false;
        }

        if let Some(option) = &self.operation.selected_option
            && let Some(firmware) = &self.operation.selected_firmware
        {
            self.logger.info(format!(
                "Retrying flash operation (attempt {}/{})",
                self.operation.retry_attempt, MAX_AUTO_RETRIES
            ));
            self.operation
                .manager
                .execute_flash(firmware, option, &self.language);
            self.operation.waiting_message_logged = false;
            return true;
        }

        false
    }

    fn transition_to_result(&mut self, snapshot: &OperationSnapshot) {
        self.logger
            .debug("State changing to Result after all conditions met");

        {
            use crate::utils::win_utils::{play_error_beep, play_success_beep};
            let is_dna_read = snapshot
                .option
                .as_ref()
                .is_some_and(|option| option.is_dna_read());
            match result_tone(&snapshot.status, is_dna_read, &snapshot.assessment) {
                ResultTone::Success => play_success_beep(),
                ResultTone::Error => play_error_beep(),
                ResultTone::None => {}
            }
        }

        self.state = AppState::Result;
        self.operation.dna_in_progress = false;
        self.operation.waiting_message_logged = false;
    }

    pub(super) fn handle_file_check_state(&mut self) -> AppState {
        let check_status = self.file_check.checker.get_status();

        match check_status {
            CheckStatus::Complete(ref result) if result.error_count == 0 => {
                if self.file_check.success_display_at.is_none() {
                    *self.file_check.checker.get_status_mut() =
                        CheckStatus::Success(Instant::now());
                    self.file_check.success_display_at = Some(Instant::now());
                }
                AppState::FileCheck
            }
            CheckStatus::Success(start_time) => {
                if start_time.elapsed() > Duration::from_secs(SUCCESS_TRANSITION_DELAY) {
                    *self.file_check.checker.get_status_mut() = CheckStatus::ReadyToTransition;
                    self.file_check.success_display_at = None;
                    AppState::OperationSelection
                } else {
                    AppState::FileCheck
                }
            }
            CheckStatus::ReadyToTransition => AppState::OperationSelection,
            _ => AppState::FileCheck,
        }
    }

    fn handle_firmware_scanning(&mut self, ctx: &egui::Context) {
        let scan_interval = if self.firmware_scan.manager.get_scan_count() <= 1 {
            FIRST_FIRMWARE_SCAN_INTERVAL_MS
        } else {
            SUBSEQUENT_FIRMWARE_SCAN_INTERVAL_MS
        };

        let should_scan = !self.firmware_scan.scanning
            && (self.firmware_scan.manager.get_scan_count() == 0
                || self.firmware_scan.last_scan.elapsed().as_millis() >= scan_interval as u128);

        if should_scan {
            self.firmware_scan.manager.scan_firmware_files();
            self.firmware_scan.mark_scan_started();

            let ctx = ctx.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(
                    FIRMWARE_SCAN_INDICATOR_DURATION_MS as u64,
                ));
                ctx.request_repaint();
            });
        }

        if self.firmware_scan.scanning
            && self.firmware_scan.last_scan.elapsed().as_millis()
                > FIRMWARE_SCAN_INDICATOR_DURATION_MS
        {
            self.firmware_scan.scanning = false;
        }
    }

    pub(super) fn handle_result_action(&mut self, action: ResultAction) {
        match action {
            ResultAction::MainMenu => {
                if !self.retire_result_operation("Main Menu") {
                    return;
                }
                self.state = AppState::OperationSelection;
                self.operation.clear_selection();
            }
            ResultAction::TryAgain => {
                let retry_plan = self.operation.retry_plan();

                match retry_plan {
                    RetryPlan::ReadDna(option) => {
                        if let Err(error) = self.operation.replace_manager(self.logger.clone()) {
                            self.logger.error(format!(
                                "Retry blocked because process retirement was not confirmed: {error}"
                            ));
                            return;
                        }
                        self.operation.reset_retry();
                        self.state = AppState::Flashing;
                        self.operation.dna_started_at = Some(Instant::now());
                        self.operation.dna_in_progress = true;

                        self.operation
                            .manager
                            .execute_dna_read(&option, &self.language);
                    }
                    RetryPlan::Flash { option, firmware } => {
                        if let Err(error) = self.operation.replace_manager(self.logger.clone()) {
                            self.logger.error(format!(
                                "Retry blocked because process retirement was not confirmed: {error}"
                            ));
                            return;
                        }
                        self.operation.reset_retry();
                        self.state = AppState::Flashing;
                        self.operation
                            .manager
                            .execute_flash(&firmware, &option, &self.language);
                    }
                    RetryPlan::ReselectFirmware => {
                        if !self.retire_result_operation("firmware reselection") {
                            return;
                        }
                        self.operation.reset_retry();
                        self.logger.info(
                            "The previous firmware was cleaned up; select a firmware file before retrying.",
                        );
                        self.operation.selected_firmware = None;
                        self.state = AppState::FirmwareSelection;
                        self.firmware_scan.manager.scan_firmware_files();
                        self.firmware_scan.mark_scan_started();
                    }
                    RetryPlan::NothingSelected => {
                        if !self.retire_result_operation("operation selection") {
                            return;
                        }
                        self.logger.warning(
                            "Cannot retry because no operation is selected; returning to operation selection.",
                        );
                        self.operation.clear_selection();
                        self.state = AppState::OperationSelection;
                    }
                }
            }
        }
    }

    fn retire_result_operation(&mut self, destination: &str) -> bool {
        match self.operation.manager.retire_for_restart() {
            Ok(()) => true,
            Err(error) => {
                self.logger.error(format!(
                    "Cannot continue to {destination} because process retirement was not confirmed: {error}"
                ));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_programmer::{DnaInfo, FlashingOption};

    fn test_app() -> FirmwareToolApp {
        let logger = crate::utils::logger::Logger::new("LifecycleRestartSafetyTest");
        FirmwareToolApp {
            window_manager: crate::utils::window::WindowManager::new(),
            state: AppState::Result,
            file_check: crate::app::flows::FileCheckFlow::new(),
            firmware_scan: crate::app::flows::FirmwareScanFlow::new(),
            operation: crate::app::flows::OperationFlow::new(logger.clone()),
            logger,
            previous_log_state: false,
            #[cfg(feature = "branding")]
            branding_manager: crate::branding::BrandingManager::new(),
            contact_copy_notification: None,
            icon_manager: crate::assets::IconManager::new(),
            language: crate::utils::localization::Language::English,
            pcileech_test: crate::pcileech_test::PcileechTestController::new(),
        }
    }

    #[test]
    fn indeterminate_completed_flash_uses_error_tone() {
        assert_eq!(
            result_tone(
                &CompletionStatus::Completed,
                false,
                &FlashAssessment::Indeterminate,
            ),
            ResultTone::Error
        );
    }

    #[test]
    fn only_successful_flash_assessments_use_success_tone() {
        assert_eq!(
            result_tone(
                &CompletionStatus::Completed,
                false,
                &FlashAssessment::Success,
            ),
            ResultTone::Success
        );
        assert_eq!(
            result_tone(
                &CompletionStatus::Completed,
                false,
                &FlashAssessment::SuccessWithLimitedSamples { total_sectors: 4 },
            ),
            ResultTone::Success
        );
        assert_eq!(
            result_tone(
                &CompletionStatus::Failed("unstable".to_string()),
                false,
                &FlashAssessment::ConnectionUnstable {
                    normal_writes: 2,
                    total_sectors: 10,
                },
            ),
            ResultTone::Error
        );
    }

    #[test]
    fn dna_tone_uses_typed_completion_status() {
        let info = DnaInfo {
            dna_value: "0x1".to_string(),
            dna_raw_value: "1".to_string(),
            device_type: "test".to_string(),
        };

        assert_eq!(
            result_tone(
                &CompletionStatus::DnaReadCompleted(info),
                true,
                &FlashAssessment::NotApplicable,
            ),
            ResultTone::Success
        );
        assert_eq!(
            result_tone(
                &CompletionStatus::Completed,
                true,
                &FlashAssessment::NotApplicable,
            ),
            ResultTone::Error
        );
    }

    #[test]
    fn cleanup_retry_delay_uses_bounded_exponential_backoff() {
        assert_eq!(cleanup_retry_delay(1), Duration::from_millis(100));
        assert_eq!(cleanup_retry_delay(2), Duration::from_millis(200));
        assert_eq!(cleanup_retry_delay(3), Duration::from_millis(400));
        assert_eq!(next_cleanup_retry(0), Some((1, Duration::from_millis(100))));
        assert_eq!(next_cleanup_retry(1), Some((2, Duration::from_millis(200))));
        assert_eq!(next_cleanup_retry(2), Some((3, Duration::from_millis(400))));
        assert_eq!(next_cleanup_retry(3), None);
    }

    #[test]
    fn terminal_status_classification_covers_all_results() {
        assert!(!is_terminal_status(&CompletionStatus::NotCompleted));
        assert!(!is_terminal_status(&CompletionStatus::InProgress(
            "working".to_string()
        )));
        assert!(is_terminal_status(&CompletionStatus::Completed));
        assert!(is_terminal_status(&CompletionStatus::Failed(
            "failed".to_string()
        )));
    }

    #[test]
    fn blocked_cleanup_prevents_main_menu_navigation() {
        let mut app = test_app();
        app.operation.selected_option = Some(FlashingOption::DnaCH347);
        app.operation
            .manager
            .block_restart_for_test("synthetic unconfirmed cleanup");

        app.handle_result_action(ResultAction::MainMenu);

        assert!(app.state == AppState::Result);
        assert_eq!(
            app.operation.selected_option,
            Some(FlashingOption::DnaCH347)
        );
    }

    #[test]
    fn blocked_cleanup_prevents_retry_escape_to_firmware_reselection() {
        let mut app = test_app();
        let selected_firmware = std::path::PathBuf::from("missing-retry-firmware.bin");
        app.operation.selected_option = Some(FlashingOption::CH347_35T);
        app.operation.selected_firmware = Some(selected_firmware.clone());
        app.operation
            .manager
            .block_restart_for_test("synthetic unconfirmed cleanup");

        app.handle_result_action(ResultAction::TryAgain);

        assert!(app.state == AppState::Result);
        assert_eq!(app.operation.selected_firmware, Some(selected_firmware));
        assert_eq!(
            app.operation.selected_option,
            Some(FlashingOption::CH347_35T)
        );
    }
}
