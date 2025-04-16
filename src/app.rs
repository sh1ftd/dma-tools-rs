use crate::APP_TITLE;
use crate::assets::IconManager;
use crate::device_programmer::{CompletionStatus, FlashingManager, FlashingOption, dna::DnaReader};
use crate::ui;
use crate::ui::file_select::FileCheckRenderContext;
use crate::ui::status::ResultAction;
use crate::utils::file_checker::{CheckStatus, FileChecker, SUCCESS_TRANSITION_DELAY};
use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::logger::Logger;
use crate::utils::win_utils::setup_window_controls;
use crate::utils::window::{WindowManager, WindowSizeType};
use eframe::egui;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

// Constants for timing
const ANIMATION_FRAME_RATE_MS: u64 = 16;
const INITIAL_CHECK_DELAY_MS: u128 = 100;
const FIRST_FIRMWARE_SCAN_INTERVAL_MS: u64 = 100;
const SUBSEQUENT_FIRMWARE_SCAN_INTERVAL_MS: u64 = 3000;
const FIRMWARE_SCAN_INDICATOR_DURATION_MS: u128 = 500;
const DNA_MIN_DISPLAY_TIME_MS: u64 = 100;
const STATUS_STABILITY_WAIT_MS: u64 = 250;

// UI constants
const TOP_PADDING: f32 = 8.0;
const BOTTOM_PADDING: f32 = 18.0;
const LOG_SECTION_PADDING: f32 = 12.0;
const HORIZONTAL_MARGIN: f32 = 20.0;
const VERTICAL_MARGIN: f32 = 10.0;

#[derive(PartialEq, Eq)]
pub enum AppState {
    FileCheck,
    OperationSelection,
    FirmwareSelection,
    FlashingOptions,
    Flashing,
    Result,
}

pub struct FirmwareToolApp {
    window_manager: WindowManager,
    state: AppState,
    file_checker: FileChecker,
    firmware_manager: FirmwareManager,
    flashing_manager: FlashingManager,
    selected_firmware: Option<PathBuf>,
    selected_option: Option<FlashingOption>,
    check_started: bool,
    start_time: Instant,
    last_firmware_scan: Instant,
    firmware_scanning: bool,
    check_success_display_time: Option<Instant>,
    logger: Logger,
    previous_log_state: bool,
    icon_manager: IconManager,
    dna_read_start_time: Option<Instant>,
    dna_read_in_progress: bool,
    waiting_message_logged: bool,
}

impl FirmwareToolApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let logger = Logger::new("AppLogger");
        logger.info(format!("{} Tool started", APP_TITLE));

        setup_window_controls();

        Self {
            window_manager: WindowManager::new(),
            state: AppState::FileCheck,
            file_checker: FileChecker::new(),
            firmware_manager: FirmwareManager::new(),
            flashing_manager: FlashingManager::new_with_logger(logger.clone()),
            selected_firmware: None,
            selected_option: None,
            check_started: false,
            start_time: Instant::now(),
            last_firmware_scan: Instant::now(),
            firmware_scanning: false,
            check_success_display_time: None,
            logger,
            previous_log_state: false,
            icon_manager: IconManager::new(),
            dna_read_start_time: None,
            dna_read_in_progress: false,
            waiting_message_logged: false,
        }
    }

    fn is_dna_read_operation(&self) -> bool {
        self.selected_option
            .as_ref()
            .is_some_and(|option| option.is_dna_read())
    }

    fn is_flash_operation(&self) -> bool {
        self.selected_option
            .as_ref()
            .is_some_and(|option| option.is_flash_operation())
    }

    fn get_window_size_type(&self) -> WindowSizeType {
        match self.state {
            AppState::FileCheck => {
                if let CheckStatus::Complete(result) = self.file_checker.get_status() {
                    if result.error_count > 0 {
                        return WindowSizeType::MissingFiles;
                    }
                }
                WindowSizeType::FileCheck
            }
            AppState::OperationSelection => WindowSizeType::OperationSelection,
            AppState::FirmwareSelection => WindowSizeType::FileSelection,
            AppState::FlashingOptions => {
                match (self.is_dna_read_operation(), self.is_flash_operation()) {
                    (true, _) => WindowSizeType::ReadOptionSelection,
                    (_, true) => WindowSizeType::FlashOptionSelection,
                    _ => WindowSizeType::FlashOptionSelection, // fallback but should never happen
                }
            }
            AppState::Flashing | AppState::Result => WindowSizeType::OperationResult,
        }
    }

    fn handle_file_check_state(&mut self) -> AppState {
        let check_status = self.file_checker.get_status();

        match check_status {
            CheckStatus::Complete(ref result) if result.error_count == 0 => {
                // First time we detect success, record the time
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

        // Reset scanning flag after showing indicator briefly
        if self.firmware_scanning
            && self.last_firmware_scan.elapsed().as_millis() > FIRMWARE_SCAN_INDICATOR_DURATION_MS
        {
            self.firmware_scanning = false;
        }
    }

    fn should_show_log(&self) -> bool {
        matches!(self.state, AppState::Flashing | AppState::Result)
    }
}

impl eframe::App for FirmwareToolApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.setup_ui_and_animation(ctx);
        self.update_window_size(frame);
        self.handle_state_specific_logic(ctx);
        self.render_main_ui(ctx);
    }
}

impl FirmwareToolApp {
    fn setup_ui_and_animation(&mut self, ctx: &egui::Context) {
        // Setup the UI style on first frame
        self.window_manager.setup_style(ctx);

        // Request animation frame rate
        ctx.request_repaint_after(Duration::from_millis(ANIMATION_FRAME_RATE_MS));
    }

    fn update_window_size(&mut self, frame: &mut eframe::Frame) {
        let window_size_type = self.get_window_size_type();
        self.window_manager.set_window_size(frame, window_size_type);
    }

    fn handle_state_specific_logic(&mut self, ctx: &egui::Context) {
        // Reset the flag whenever we're not in Flashing state
        if self.state != AppState::Flashing {
            self.waiting_message_logged = false;
        }

        // Stop monitor thread if we're leaving the Result state
        if self.state != AppState::Result && self.state != AppState::Flashing {
            self.flashing_manager.stop_monitor_thread();
        }

        // Start file check if not already started
        if !self.check_started && self.start_time.elapsed().as_millis() > INITIAL_CHECK_DELAY_MS {
            self.file_checker.start_check();
            self.check_started = true;
        }

        // Handle firmware scanning when in FirmwareSelection state
        if self.state == AppState::FirmwareSelection {
            self.handle_firmware_scanning(ctx);
        }

        // Check for operation completion with forced minimum display time
        if self.state == AppState::Flashing {
            let status = self.flashing_manager.get_status();

            // Force a minimum display time EVEN IF the operation reports completion
            let min_display_time_elapsed = self
                .dna_read_start_time
                .is_none_or(|t| t.elapsed() > Duration::from_millis(DNA_MIN_DISPLAY_TIME_MS));

            if self.flashing_manager.check_if_completed()
                && !matches!(status, CompletionStatus::InProgress(_))
                && min_display_time_elapsed
            {
                if let Some(last_state_change) = self.flashing_manager.get_last_status_change_time()
                {
                    if last_state_change.elapsed() < Duration::from_millis(STATUS_STABILITY_WAIT_MS)
                    {
                        // Status changed too recently - wait a bit longer for stability
                        self.logger
                            .debug("Status changed recently - waiting for stability");
                        return;
                    }
                }

                // Stop any running DNA thread before transitioning
                if self.dna_read_in_progress {
                    self.logger
                        .debug("Stopping DNA thread before showing results");
                    self.flashing_manager.stop_dna_thread();

                    thread::sleep(Duration::from_millis(100));
                }

                self.logger
                    .debug("State changing to Result after all conditions met");
                self.state = AppState::Result;
                self.dna_read_in_progress = false;
                self.waiting_message_logged = false;
            } else if self.flashing_manager.check_if_completed() {
                // Only log once
                if !self.waiting_message_logged {
                    self.logger
                        .debug("Operation completed but waiting for minimum display time");
                    self.waiting_message_logged = true;
                }
            }
        }

        // Ensure icons are loaded
        self.icon_manager.ensure_loaded(ctx);
    }

    fn render_main_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Add some padding at the top
            ui.add_space(TOP_PADDING);

            egui::Frame::none()
                .inner_margin(egui::style::Margin::symmetric(
                    HORIZONTAL_MARGIN,
                    VERTICAL_MARGIN,
                ))
                .show(ui, |ui| {
                    self.render_state_content(ui);
                });

            self.handle_log_display(ui, ctx);
        });
    }

    fn render_state_content(&mut self, ui: &mut egui::Ui) {
        match self.state {
            AppState::FileCheck => self.render_file_check_state(ui),
            AppState::OperationSelection => self.render_operation_selection(ui),
            AppState::FirmwareSelection => self.render_firmware_selection(ui),
            AppState::FlashingOptions => self.render_flashing_options(ui),
            AppState::Flashing => self.render_flashing(ui),
            AppState::Result => self.render_result(ui),
        }
    }

    fn render_file_check_state(&mut self, ui: &mut egui::Ui) {
        let check_status = self.file_checker.get_status();
        self.state = self.handle_file_check_state();

        let mut continue_callback = |continue_anyway: bool| {
            if continue_anyway {
                if let CheckStatus::Complete(result) = &check_status {
                    if result.error_count > 0 {
                        self.state = AppState::OperationSelection;
                    }
                }
            } else {
                std::process::exit(1);
            }
        };

        let mut rescan_callback = || {
            // Reset the check state
            self.check_started = false;
            // Clear the previous check results
            self.file_checker = FileChecker::new();
        };

        // Add empty callback for the unused start check parameter
        let _empty_callback = || {};

        ui::file_select::render_file_check(&mut FileCheckRenderContext {
            ui,
            check_status: &check_status,
            on_continue: &mut continue_callback,
            on_rescan: &mut rescan_callback,
        });
    }

    fn render_operation_selection(&mut self, ui: &mut egui::Ui) {
        let mut operation_callback = |operation_type| match operation_type {
            ui::operation::OperationType::FlashFirmware => {
                self.state = AppState::FirmwareSelection;

                self.firmware_manager.scan_firmware_files();
                self.last_firmware_scan = Instant::now();
                self.firmware_scanning = true;
            }
            ui::operation::OperationType::ReadDNA => {
                self.state = AppState::FlashingOptions;
                self.selected_option = Some(FlashingOption::DnaCH347);
            }
        };

        ui::operation::render_operation_selection(ui, &mut operation_callback);
    }

    fn render_firmware_selection(&mut self, ui: &mut egui::Ui) {
        // Get cleanup value before the callback
        let cleanup_enabled = self.firmware_manager.get_cleanup_enabled();

        let mut select_callback = |selected: Option<PathBuf>| {
            self.selected_firmware = selected;
            self.state = AppState::FlashingOptions;
            // Pass cleanup option to flashing manager
            self.flashing_manager.set_cleanup_enabled(cleanup_enabled);
        };

        // Store the scan count before passing the mutable reference
        let scan_count = self.firmware_manager.get_scan_count();
        let is_scanning = self.firmware_scanning || scan_count <= 1;

        ui::file_select::render_firmware_selection(
            ui,
            &mut self.firmware_manager,
            &mut select_callback,
            is_scanning,
        );
    }

    fn render_flashing_options(&mut self, ui: &mut egui::Ui) {
        // Store references to these fields
        let app_state = &mut self.state;
        let selected_option = &mut self.selected_option;
        let selected_firmware = &self.selected_firmware;
        let flashing_manager = &mut self.flashing_manager;
        let dna_read_start_time = &mut self.dna_read_start_time;
        let dna_read_in_progress = &mut self.dna_read_in_progress;

        let mut option_callback = |option: FlashingOption| {
            *selected_option = Some(option.clone());

            if option.is_dna_read() {
                *app_state = AppState::Flashing;

                // Set these flags for the initial DNA read
                *dna_read_start_time = Some(Instant::now());
                *dna_read_in_progress = true;

                flashing_manager.execute_dna_read(&option);
            } else if let Some(firmware) = selected_firmware {
                *app_state = AppState::Flashing;
                flashing_manager.execute_flash(firmware, &option);
            }
        };

        // Use the appropriate render function based on operation type
        if selected_firmware.is_some() {
            ui::options::render_flash_options(ui, &mut option_callback);
        } else {
            ui::options::render_dna_read_options(ui, &mut option_callback);
        }
    }

    fn render_flashing(&mut self, ui: &mut egui::Ui) {
        // Render the progress UI
        ui::status::render_flashing_progress(ui, &self.flashing_manager);
    }

    fn render_result(&mut self, ui: &mut egui::Ui) {
        // Create an action holder to capture the result
        let mut action_to_take = None;

        {
            let mut action_callback = |action: ResultAction| {
                action_to_take = Some(action);
            };

            ui::status::render_result_screen(
                ui,
                &self.flashing_manager,
                &mut action_callback,
                &self.icon_manager,
            );
        }

        // Handle the action outside the closure, after the UI rendering
        if let Some(action) = action_to_take {
            self.handle_result_action(action);
        }
    }

    fn handle_result_action(&mut self, action: ResultAction) {
        // Stop the monitor thread when an operation completes
        self.flashing_manager.stop_monitor_thread();

        match action {
            ResultAction::Exit => {
                std::process::exit(0);
            }
            ResultAction::MainMenu => {
                self.state = AppState::OperationSelection;
                self.selected_firmware = None;
                self.selected_option = None;
            }
            ResultAction::TryAgain => {
                // Force a complete reset of the flashing manager
                self.flashing_manager = FlashingManager::new_with_logger(self.logger.clone());

                // Re-run the same operation
                if let Some(option) = &self.selected_option {
                    if option.is_dna_read() {
                        // Re-run DNA read with the same option
                        self.state = AppState::Flashing;
                        self.dna_read_start_time = Some(Instant::now()); // SET the timer, don't reset to None
                        self.dna_read_in_progress = true; // Set the in-progress flag

                        // Clean up any existing DNA file before retrying
                        DnaReader::cleanup_dna_output_file(&self.logger);

                        self.flashing_manager.execute_dna_read(option);
                    } else if let Some(firmware) = &self.selected_firmware {
                        // Re-run flashing with the same firmware and option
                        self.state = AppState::Flashing;
                        self.flashing_manager.execute_flash(firmware, option);
                    }
                }
            }
        }
    }

    fn handle_log_display(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Only add space and separator when log will be shown
        let show_log = self.should_show_log();

        // Just update the tracking state without trying to resize the window
        if show_log != self.previous_log_state {
            self.previous_log_state = show_log;
            // Request a full repaint when log visibility changes
            ctx.request_repaint();
        }

        if show_log {
            // Add space and separator before log view
            ui.add_space(LOG_SECTION_PADDING);
            ui.separator();

            // Show log view at the bottom of all screens
            ui::log_view::render_log_view(ui, &self.logger);
        } else {
            // Add some padding at the bottom for consistent UI
            ui.add_space(BOTTOM_PADDING);
        }
    }
}
