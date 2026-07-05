use super::{AppState, FirmwareToolApp};
use crate::device_programmer::{CompletionStatus, FlashingManager, dna::DnaReader};
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
const STATUS_STABILITY_WAIT_MS: u64 = 250;

const MAX_AUTO_RETRIES: u32 = 10;
const RETRY_COOLDOWN_MS: u64 = 2500;

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
            self.waiting_message_logged = false;
        }
    }

    fn stop_monitor_when_not_showing_operation(&mut self) {
        if self.state != AppState::Result && self.state != AppState::Flashing {
            self.flashing_manager.stop_monitor_thread();
        }
    }

    fn maybe_start_file_check(&mut self) {
        if !self.check_started && self.start_time.elapsed().as_millis() > INITIAL_CHECK_DELAY_MS {
            self.file_checker.start_check();
            self.check_started = true;
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

        let status = self.flashing_manager.get_status();

        let min_display_time_elapsed = self
            .dna_read_start_time
            .is_none_or(|t| t.elapsed() > Duration::from_millis(DNA_MIN_DISPLAY_TIME_MS));

        if self.flashing_manager.check_if_completed()
            && !matches!(status, CompletionStatus::InProgress(_))
            && min_display_time_elapsed
        {
            if self.status_changed_too_recently() {
                return;
            }

            if self.maybe_auto_retry_flash() {
                return;
            }

            self.stop_dna_output_parsing_before_result();
            self.transition_to_result(status);
        } else if self.flashing_manager.check_if_completed() && !self.waiting_message_logged {
            self.logger
                .debug("Operation completed but waiting for minimum display time");
            self.waiting_message_logged = true;
        }
    }

    fn status_changed_too_recently(&self) -> bool {
        if let Some(last_state_change) = self.flashing_manager.get_last_status_change_time()
            && last_state_change.elapsed() < Duration::from_millis(STATUS_STABILITY_WAIT_MS)
        {
            self.logger
                .debug("Status changed recently - waiting for stability");
            return true;
        }

        false
    }

    fn maybe_auto_retry_flash(&mut self) -> bool {
        if !(self.flashing_manager.was_terminated_early()
            && self.auto_retry_attempt < MAX_AUTO_RETRIES
            && !self.dna_read_in_progress)
        {
            return false;
        }

        if self.retry_cooldown_start.is_none() {
            self.auto_retry_attempt += 1;
            self.logger.info(format!(
                "Connection unstable — retrying automatically (attempt {}/{})",
                self.auto_retry_attempt, MAX_AUTO_RETRIES
            ));
            self.retry_cooldown_start = Some(Instant::now());
            return true;
        }

        if let Some(cooldown_start) = self.retry_cooldown_start
            && cooldown_start.elapsed() < Duration::from_millis(RETRY_COOLDOWN_MS)
        {
            let remaining_ms =
                RETRY_COOLDOWN_MS.saturating_sub(cooldown_start.elapsed().as_millis() as u64);
            self.logger
                .debug(format!("Retry cooldown: {}ms remaining", remaining_ms));
            return true;
        }

        self.retry_cooldown_start = None;
        self.flashing_manager.stop_monitor_thread();
        self.flashing_manager = FlashingManager::new_with_logger(self.logger.clone());

        if let Some(option) = &self.selected_option
            && let Some(firmware) = &self.selected_firmware
        {
            self.logger.info(format!(
                "Retrying flash operation (attempt {}/{})",
                self.auto_retry_attempt, MAX_AUTO_RETRIES
            ));
            self.flashing_manager
                .execute_flash(firmware, option, &self.language);
            self.waiting_message_logged = false;
            return true;
        }

        false
    }

    fn stop_dna_output_parsing_before_result(&mut self) {
        if self.dna_read_in_progress {
            self.logger
                .debug("Stopping DNA output parsing before showing results");
            self.flashing_manager.stop_dna_output_parsing();

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn transition_to_result(&mut self, status: CompletionStatus) {
        self.logger
            .debug("State changing to Result after all conditions met");

        {
            use crate::utils::win_utils::{play_error_beep, play_success_beep};
            match status {
                CompletionStatus::Completed | CompletionStatus::DnaReadCompleted(_) => {
                    play_success_beep()
                }
                CompletionStatus::Failed(_) => play_error_beep(),
                _ => {}
            }
        }

        self.state = AppState::Result;
        self.dna_read_in_progress = false;
        self.waiting_message_logged = false;
    }

    pub(super) fn handle_file_check_state(&mut self) -> AppState {
        let check_status = self.file_checker.get_status();

        match check_status {
            CheckStatus::Complete(ref result) if result.error_count == 0 => {
                if self.check_success_display_time.is_none() {
                    *self.file_checker.get_status_mut() = CheckStatus::Success(Instant::now());
                    self.check_success_display_time = Some(Instant::now());
                }
                AppState::FileCheck
            }
            CheckStatus::Success(start_time) => {
                if start_time.elapsed() > Duration::from_secs(SUCCESS_TRANSITION_DELAY) {
                    *self.file_checker.get_status_mut() = CheckStatus::ReadyToTransition;
                    self.check_success_display_time = None;
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
        let scan_interval = if self.firmware_manager.get_scan_count() <= 1 {
            FIRST_FIRMWARE_SCAN_INTERVAL_MS
        } else {
            SUBSEQUENT_FIRMWARE_SCAN_INTERVAL_MS
        };

        let should_scan = !self.firmware_scanning
            && (self.firmware_manager.get_scan_count() == 0
                || self.last_firmware_scan.elapsed().as_millis() >= scan_interval as u128);

        if should_scan {
            self.firmware_manager.scan_firmware_files();
            self.last_firmware_scan = Instant::now();
            self.firmware_scanning = true;

            let ctx = ctx.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(
                    FIRMWARE_SCAN_INDICATOR_DURATION_MS as u64,
                ));
                ctx.request_repaint();
            });
        }

        if self.firmware_scanning
            && self.last_firmware_scan.elapsed().as_millis() > FIRMWARE_SCAN_INDICATOR_DURATION_MS
        {
            self.firmware_scanning = false;
        }
    }

    pub(super) fn handle_result_action(&mut self, action: ResultAction) {
        self.flashing_manager.stop_monitor_thread();

        match action {
            ResultAction::MainMenu => {
                self.state = AppState::OperationSelection;
                self.selected_firmware = None;
                self.selected_option = None;
                self.auto_retry_attempt = 0;
                self.retry_cooldown_start = None;
            }
            ResultAction::TryAgain => {
                self.auto_retry_attempt = 0;
                self.retry_cooldown_start = None;

                self.flashing_manager = FlashingManager::new_with_logger(self.logger.clone());

                if let Some(option) = &self.selected_option {
                    if option.is_dna_read() {
                        self.state = AppState::Flashing;
                        self.dna_read_start_time = Some(Instant::now());
                        self.dna_read_in_progress = true;

                        DnaReader::cleanup_dna_output_file(&self.logger);

                        self.flashing_manager
                            .execute_dna_read(option, &self.language);
                    } else if let Some(firmware) = &self.selected_firmware {
                        self.state = AppState::Flashing;
                        self.flashing_manager
                            .execute_flash(firmware, option, &self.language);
                    }
                }
            }
        }
    }
}
