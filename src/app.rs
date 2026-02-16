use crate::APP_TITLE;
use crate::assets::IconManager;

#[cfg(feature = "branding")]
use crate::branding::BrandingManager;

use crate::device_programmer::{CompletionStatus, FlashingManager, FlashingOption, dna::DnaReader};
use crate::ui;
use crate::ui::file_select::FileCheckRenderContext;
use crate::ui::status::ResultAction;
use crate::utils::file_checker::{CheckStatus, FileChecker, SUCCESS_TRANSITION_DELAY};
use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::localization::{TextKey, translate};
use crate::utils::logger::Logger;
use crate::utils::win_utils::setup_window_controls;
use crate::utils::window::{WindowManager, WindowSizeType};
use eframe::egui;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use crate::utils::contact;

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
const HORIZONTAL_MARGIN: i8 = 20;
const VERTICAL_MARGIN: i8 = 10;

#[derive(PartialEq, Eq)]
pub enum AppState {
    FileCheck,
    OperationSelection,
    FirmwareSelection,
    FlashingOptions,
    Flashing,
    Result,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Language {
    English,
    Chinese,
    German,
    Portuguese,
    Arabic,
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
    #[cfg(feature = "branding")]
    branding_manager: BrandingManager,
    contact_copy_notification: Option<(String, Instant)>,
    language: Language,
}

impl FirmwareToolApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let logger = Logger::new("AppLogger");
        logger.info(format!("{APP_TITLE} Tool started"));

        setup_window_controls();

        let window_manager = WindowManager::new();
        window_manager.setup_fonts(&cc.egui_ctx);
        window_manager.setup_style(&cc.egui_ctx);

        Self {
            window_manager,
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
            #[cfg(feature = "branding")]
            branding_manager: BrandingManager::new(),
            contact_copy_notification: None,
            language: Language::English,
        }
    }

    // ... existing helper methods ...

    fn render_contact_icon(
        ui: &mut egui::Ui,
        icon: &egui::TextureHandle,
        copy_text: &str,
        tooltip: &str,
        notification_msg: String,
        notification: &mut Option<(String, Instant)>,
    ) {
        // Allocate space for the icon button (fixed layout size to prevent jitter)
        let button_size = egui::Vec2::splat(34.0);
        let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

        // Handle interactions
        if response.clicked() {
            ui.ctx().copy_text(copy_text.to_string());
            *notification = Some((notification_msg, Instant::now()));
        }
        let response = response.on_hover_text(tooltip);

        // Determine visual properties
        let is_hovered = response.hovered();

        // Animate scale
        let target_scale = if is_hovered { 1.15 } else { 1.0 };
        let scale = ui
            .ctx()
            .animate_value_with_time(response.id.with("scale"), target_scale, 0.1);

        // Render icon
        if ui.is_rect_visible(rect) {
            let icon_size = 28.0 * scale;
            let icon_rect =
                egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(icon_size));

            // Tint: Light Gray by default, White on hover
            let tint = if is_hovered {
                egui::Color32::WHITE
            } else {
                egui::Color32::LIGHT_GRAY
            };

            ui.painter().image(
                icon.id(),
                icon_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                tint,
            );
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
                if let CheckStatus::Complete(result) = self.file_checker.get_status()
                    && result.error_count > 0
                {
                    return WindowSizeType::MissingFiles;
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force dark mode if system is overriding it
        if !ctx.style().visuals.dark_mode {
            self.window_manager.setup_style(ctx);
        }

        self.setup_ui_and_animation(ctx);
        self.update_window_size(ctx);
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

    fn update_window_size(&mut self, ctx: &egui::Context) {
        let window_size_type = self.get_window_size_type();
        self.window_manager.set_window_size(ctx, window_size_type);
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
                    && last_state_change.elapsed() < Duration::from_millis(STATUS_STABILITY_WAIT_MS)
                {
                    // Status changed too recently - wait a bit longer for stability
                    self.logger
                        .debug("Status changed recently - waiting for stability");
                    return;
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

        // Ensure branding texture is loaded when branding feature is enabled
        #[cfg(feature = "branding")]
        self.branding_manager.ensure_loaded(ctx);
    }

    fn render_main_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("contact_footer")
            .show_separator_line(false)
            .show(ctx, |ui| {
                self.render_contact_info(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Render brand background first (behind all other content)
            #[cfg(feature = "branding")]
            crate::branding::render_background(ui, &self.branding_manager);

            // Add some padding at the top
            ui.add_space(TOP_PADDING);

            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(HORIZONTAL_MARGIN, VERTICAL_MARGIN))
                .show(ui, |ui| {
                    self.render_state_content(ui);
                });

            self.handle_log_display(ui, ctx);
        });

        // Floating notification overlay
        if let Some((msg, time)) = &self.contact_copy_notification
            && time.elapsed() < Duration::from_secs(2)
        {
            egui::Area::new(egui::Id::new("copy_notification"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -85.0))
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_black_alpha(192)) // ~75% opacity
                        .corner_radius(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            // Max width = (3 * 34.0) buttons + (2 * 4.0) spaces = 110.0
                            ui.set_max_width(110.0);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(msg)
                                        .color(egui::Color32::GREEN)
                                        .size(14.0),
                                );
                            });
                        });
                });
            ctx.request_repaint();
        }
    }

    fn render_contact_info(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let base_flag_size = 28.0;

            // US Flag
            if let Some(icon) = self.icon_manager.us_flag() {
                // Allocate space for the flag button
                let button_size = egui::Vec2::splat(34.0);
                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                // Handle click
                if response.clicked() {
                    self.language = Language::English;
                }
                let response = response.on_hover_text("English");

                // Animate scale on hover
                let is_hovered = response.hovered();
                let target_scale = if is_hovered { 1.15 } else { 1.0 };
                let scale = ui.ctx().animate_value_with_time(
                    response.id.with("flag_scale"),
                    target_scale,
                    0.1,
                );

                // Render flag
                if ui.is_rect_visible(rect) {
                    let flag_size = base_flag_size * scale;
                    let flag_rect =
                        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(flag_size));

                    // Dim the flag if not selected
                    let tint = if self.language == Language::English {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(100)
                    };

                    ui.painter().image(
                        icon.id(),
                        flag_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }
            }

            ui.add_space(8.0);

            // CN Flag
            if let Some(icon) = self.icon_manager.cn_flag() {
                // Allocate space for the flag button
                let button_size = egui::Vec2::splat(38.0);
                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                // Handle click
                if response.clicked() {
                    self.language = Language::Chinese;
                }
                let response = response.on_hover_text("中文");

                // Animate scale on hover
                let is_hovered = response.hovered();
                let target_scale = if is_hovered { 1.15 } else { 1.0 };
                let scale = ui.ctx().animate_value_with_time(
                    response.id.with("flag_scale"),
                    target_scale,
                    0.1,
                );

                // Render flag
                if ui.is_rect_visible(rect) {
                    let flag_size = base_flag_size * scale;
                    let flag_rect =
                        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(flag_size));

                    // Dim the flag if not selected
                    let tint = if self.language == Language::Chinese {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(100)
                    };

                    ui.painter().image(
                        icon.id(),
                        flag_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }
            }

            ui.add_space(8.0);

            // German Flag
            if let Some(icon) = self.icon_manager.de_flag() {
                let button_size = egui::Vec2::splat(34.0);
                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                if response.clicked() {
                    self.language = Language::German;
                }
                let response = response.on_hover_text("Deutsch");

                let is_hovered = response.hovered();
                let target_scale = if is_hovered { 1.15 } else { 1.0 };
                let scale = ui.ctx().animate_value_with_time(
                    response.id.with("flag_scale"),
                    target_scale,
                    0.1,
                );

                if ui.is_rect_visible(rect) {
                    let flag_size = base_flag_size * scale;
                    let flag_rect =
                        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(flag_size));

                    let tint = if self.language == Language::German {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(100)
                    };

                    ui.painter().image(
                        icon.id(),
                        flag_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }
            }

            ui.add_space(8.0);

            // Brazilian Flag
            if let Some(icon) = self.icon_manager.br_flag() {
                let button_size = egui::Vec2::splat(34.0);
                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                if response.clicked() {
                    self.language = Language::Portuguese;
                }
                let response = response.on_hover_text("Português");

                let is_hovered = response.hovered();
                let target_scale = if is_hovered { 1.15 } else { 1.0 };
                let scale = ui.ctx().animate_value_with_time(
                    response.id.with("flag_scale"),
                    target_scale,
                    0.1,
                );

                if ui.is_rect_visible(rect) {
                    let flag_size = base_flag_size * scale;
                    let flag_rect =
                        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(flag_size));

                    let tint = if self.language == Language::Portuguese {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(100)
                    };

                    ui.painter().image(
                        icon.id(),
                        flag_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }
            }

            ui.add_space(8.0);

            // Yemen Flag
            if let Some(icon) = self.icon_manager.ar_flag() {
                let button_size = egui::Vec2::splat(34.0);
                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                if response.clicked() {
                    self.language = Language::Arabic;
                }
                let response = response.on_hover_text("العربية");

                let is_hovered = response.hovered();
                let target_scale = if is_hovered { 1.15 } else { 1.0 };
                let scale = ui.ctx().animate_value_with_time(
                    response.id.with("flag_scale"),
                    target_scale,
                    0.1,
                );

                if ui.is_rect_visible(rect) {
                    let flag_size = base_flag_size * scale;
                    let flag_rect =
                        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(flag_size));

                    let tint = if self.language == Language::Arabic {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(100)
                    };

                    ui.painter().image(
                        icon.id(),
                        flag_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                // Determine contact info
                #[cfg(feature = "branding")]
                let (show_tg, tg_contact, show_wc, wc_contact, show_dc, dc_contact) = (
                    crate::branding::SHOW_TELEGRAM,
                    crate::branding::TELEGRAM_CONTACT,
                    crate::branding::SHOW_WECHAT,
                    crate::branding::WECHAT_CONTACT,
                    crate::branding::SHOW_DISCORD,
                    crate::branding::DISCORD_CONTACT,
                );

                #[cfg(not(feature = "branding"))]
                let (show_tg, tg_contact, show_wc, wc_contact, show_dc, dc_contact) = (
                    contact::SHOW_TELEGRAM,
                    contact::TELEGRAM_CONTACT,
                    contact::SHOW_WECHAT,
                    contact::WECHAT_CONTACT,
                    contact::SHOW_DISCORD,
                    contact::DISCORD_CONTACT,
                );

                // Telegram
                if show_tg && let Some(icon) = self.icon_manager.telegram_icon() {
                    Self::render_contact_icon(
                        ui,
                        icon,
                        tg_contact,
                        translate(TextKey::CopyTelegram, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::TelegramLink, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_tg && (show_wc || show_dc) {
                    ui.add_space(4.0);
                }

                // Wechat
                if show_wc && let Some(icon) = self.icon_manager.wechat_icon() {
                    Self::render_contact_icon(
                        ui,
                        icon,
                        wc_contact,
                        translate(TextKey::CopyWeChat, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::WeChatID, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_wc && show_dc {
                    ui.add_space(4.0);
                }

                // Discord
                if show_dc && let Some(icon) = self.icon_manager.discord_icon() {
                    Self::render_contact_icon(
                        ui,
                        icon,
                        dc_contact,
                        translate(TextKey::CopyDiscord, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::DiscordID, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_tg || show_wc || show_dc {
                    ui.label(translate(TextKey::Contact, &self.language));
                }
            });
        });
        ui.add_space(4.0);
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
            language: &self.language,
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

        ui::operation::render_operation_selection(ui, &mut operation_callback, &self.language);
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
            &self.language,
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
        let language = &self.language;

        let mut option_callback = |option: FlashingOption| {
            *selected_option = Some(option.clone());

            if option.is_dna_read() {
                *app_state = AppState::Flashing;

                // Set these flags for the initial DNA read
                *dna_read_start_time = Some(Instant::now());
                *dna_read_in_progress = true;

                flashing_manager.execute_dna_read(&option, language);
            } else if let Some(firmware) = selected_firmware {
                *app_state = AppState::Flashing;
                flashing_manager.execute_flash(firmware, &option, language);
            }
        };

        // Use the appropriate render function based on operation type
        if selected_firmware.is_some() {
            ui::options::render_flash_options(ui, &mut option_callback, &self.language);
        } else {
            ui::options::render_dna_read_options(ui, &mut option_callback, &self.language);
        }
    }

    fn render_flashing(&mut self, ui: &mut egui::Ui) {
        // Render the progress UI
        ui::status::render_flashing_progress(ui, &self.flashing_manager, &self.language);
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
                &self.language,
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

                        self.flashing_manager
                            .execute_dna_read(option, &self.language);
                    } else if let Some(firmware) = &self.selected_firmware {
                        // Re-run flashing with the same firmware and option
                        self.state = AppState::Flashing;
                        self.flashing_manager
                            .execute_flash(firmware, option, &self.language);
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
            ui::log_view::render_log_view(ui, &self.logger, &self.language);
        } else {
            // Add some padding at the bottom for consistent UI
            ui.add_space(BOTTOM_PADDING);
        }
    }
}
