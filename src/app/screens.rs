use super::{AppState, FirmwareToolApp};
use crate::device_programmer::FlashingOption;
use crate::pcileech_test::PcileechTestSnapshot;
use crate::ui;
use crate::ui::file_select::FileCheckRenderContext;
use crate::ui::pcileech_test::PcileechAction;
use crate::ui::status::ResultAction;
use crate::utils::file_checker::CheckStatus;
use eframe::egui;
use std::path::PathBuf;
use std::time::Instant;

const TOP_PADDING: f32 = 8.0;
const BOTTOM_PADDING: f32 = 18.0;
const LOG_SECTION_PADDING: f32 = 12.0;
// Note: egui::Margin::symmetric takes i8 — keep values within [-128, 127]
const HORIZONTAL_MARGIN: i8 = 20;
const VERTICAL_MARGIN: i8 = 10;

impl FirmwareToolApp {
    #[allow(deprecated)]
    pub(super) fn render_main_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        egui::Panel::bottom("contact_footer")
            .show_separator_line(false)
            .show(ui, |ui| {
                self.render_contact_info(ui);
            });

        egui::CentralPanel::default().show(ui, |ui| {
            #[cfg(feature = "branding")]
            crate::branding::render_background(ui, &self.branding_manager);

            ui.add_space(TOP_PADDING);

            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(HORIZONTAL_MARGIN, VERTICAL_MARGIN))
                .show(ui, |ui| {
                    self.render_state_content(ui);
                });

            self.handle_log_display(ui, &ctx);
        });

        self.render_contact_copy_notification(&ctx);
    }

    fn render_state_content(&mut self, ui: &mut egui::Ui) {
        match self.state {
            AppState::FileCheck => self.render_file_check_state(ui),
            AppState::OperationSelection => self.render_operation_selection(ui),
            AppState::FirmwareSelection => self.render_firmware_selection(ui),
            AppState::FlashingOptions => self.render_flashing_options(ui),
            AppState::Flashing => self.render_flashing(ui),
            AppState::Result => self.render_result(ui),
            AppState::Drivers => self.render_drivers(ui),
            AppState::PcileechTest => self.render_pcileech_test(ui),
        }
    }

    fn render_file_check_state(&mut self, ui: &mut egui::Ui) {
        let check_status = self.file_check.checker.get_status();
        self.state = self.handle_file_check_state();

        let mut continue_callback = |continue_anyway: bool| {
            if continue_anyway {
                if let CheckStatus::Complete(result) = &check_status
                    && result.error_count > 0
                {
                    self.state = AppState::OperationSelection;
                }
            } else {
                std::process::exit(1);
            }
        };

        let mut rescan_callback = || {
            self.file_check.reset();
        };

        ui::file_select::render_file_check(&mut FileCheckRenderContext {
            ui,
            check_status: &check_status,
            on_continue: &mut continue_callback,
            on_rescan: &mut rescan_callback,
            language: &self.language,
        });
    }

    fn render_operation_selection(&mut self, ui: &mut egui::Ui) {
        let mut operation_callback = |operation_type| match operation_type {
            ui::operation::OperationType::FlashFirmware => {
                self.state = AppState::FirmwareSelection;

                self.firmware_scan.manager.scan_firmware_files();
                self.firmware_scan.mark_scan_started();
            }
            ui::operation::OperationType::ReadDNA => {
                self.state = AppState::FlashingOptions;
                self.operation.selected_option = Some(FlashingOption::DnaCH347);
            }
            ui::operation::OperationType::Drivers => {
                self.state = AppState::Drivers;
            }
            ui::operation::OperationType::TestPcileech => {
                self.state = AppState::PcileechTest;
            }
        };

        ui::operation::render_operation_selection(ui, &mut operation_callback, &self.language);
    }

    fn render_drivers(&mut self, ui: &mut egui::Ui) {
        let mut back_callback = || {
            self.state = AppState::OperationSelection;
        };
        crate::ui::drivers::render_drivers_screen(ui, &mut back_callback, &self.language);
    }

    fn render_pcileech_test(&mut self, ui: &mut egui::Ui) {
        if self.pcileech_test.acknowledge_back_if_ready() {
            self.state = AppState::OperationSelection;
            return;
        }

        self.pcileech_test.ensure_started();
        let test_snapshot: PcileechTestSnapshot = self.pcileech_test.snapshot();

        match crate::ui::pcileech_test::render_pcileech_test(
            ui,
            &test_snapshot.state,
            &self.language,
        ) {
            Some(PcileechAction::Back) => {
                self.pcileech_test.request_back();
                if self.pcileech_test.acknowledge_back_if_ready() {
                    self.state = AppState::OperationSelection;
                }
            }
            Some(PcileechAction::Retry) => self.pcileech_test.retry(),
            None => {}
        }
    }

    fn render_firmware_selection(&mut self, ui: &mut egui::Ui) {
        let mut selected_file = None;
        let mut go_back = false;

        let mut select_callback = |selected: Option<PathBuf>| {
            selected_file = Some(selected);
        };

        let scan_count = self.firmware_scan.manager.get_scan_count();
        let is_scanning = self.firmware_scan.scanning || scan_count <= 1;

        let mut back_callback = || {
            go_back = true;
        };

        ui::file_select::render_firmware_selection(
            ui,
            &mut self.firmware_scan.manager,
            &mut select_callback,
            &mut back_callback,
            is_scanning,
            &self.language,
        );

        if go_back {
            self.state = AppState::OperationSelection;
        } else if let Some(selected) = selected_file {
            let cleanup_enabled = self.firmware_scan.manager.get_cleanup_enabled();
            self.operation.selected_firmware = selected;
            self.state = AppState::FlashingOptions;
            self.operation.set_cleanup_enabled(cleanup_enabled);
        }
    }

    fn render_flashing_options(&mut self, ui: &mut egui::Ui) {
        let app_state = &mut self.state;
        let selected_option = &mut self.operation.selected_option;
        let selected_firmware = &self.operation.selected_firmware;
        let flashing_manager = &mut self.operation.manager;
        let dna_read_start_time = &mut self.operation.dna_started_at;
        let dna_read_in_progress = &mut self.operation.dna_in_progress;
        let auto_retry_attempt = &mut self.operation.retry_attempt;
        let retry_cooldown_start = &mut self.operation.retry_cooldown_started_at;
        let language = &self.language;
        let mut go_back = false;

        let mut option_callback = |option: FlashingOption| {
            *selected_option = Some(option.clone());

            *auto_retry_attempt = 0;
            *retry_cooldown_start = None;

            if option.is_dna_read() {
                *app_state = AppState::Flashing;

                *dna_read_start_time = Some(Instant::now());
                *dna_read_in_progress = true;

                flashing_manager.execute_dna_read(&option, language);
            } else if let Some(firmware) = selected_firmware {
                *app_state = AppState::Flashing;
                flashing_manager.execute_flash(firmware, &option, language);
            }
        };

        let mut back_callback = || {
            go_back = true;
        };

        if selected_firmware.is_some() {
            ui::options::render_flash_options(ui, &mut option_callback, &self.language);
        } else {
            ui::options::render_dna_read_options(
                ui,
                &mut option_callback,
                &mut back_callback,
                &self.language,
            );
        }

        if go_back {
            *app_state = AppState::OperationSelection;
            *selected_option = None;
        }
    }

    fn render_flashing(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.operation.manager.snapshot();
        ui::status::render_flashing_progress(ui, &snapshot, &self.language);
    }

    fn render_result(&mut self, ui: &mut egui::Ui) {
        let mut action_to_take = None;

        {
            let mut action_callback = |action: ResultAction| {
                action_to_take = Some(action);
            };

            let snapshot = self.operation.manager.snapshot();
            ui::status::render_result_screen(ui, &snapshot, &mut action_callback, &self.language);
        }

        if let Some(action) = action_to_take {
            self.handle_result_action(action);
        }
    }

    fn handle_log_display(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let show_log = self.should_show_log();

        if show_log != self.previous_log_state {
            self.previous_log_state = show_log;
            ctx.request_repaint();
        }

        if show_log {
            ui.add_space(LOG_SECTION_PADDING);
            ui.separator();

            ui::log_view::render_log_view(ui, &self.logger, &self.language);
        } else {
            ui.add_space(BOTTOM_PADDING);
        }
    }
}
