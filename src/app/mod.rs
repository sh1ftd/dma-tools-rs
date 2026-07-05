use crate::APP_TITLE;

#[cfg(feature = "branding")]
use crate::branding::BrandingManager;

use crate::device_programmer::{FlashingManager, FlashingOption};
use crate::utils::file_checker::FileChecker;
use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::localization::Language;
use crate::utils::logger::Logger;
use crate::utils::window::WindowManager;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod footer;
mod lifecycle;
mod screens;
mod state;

use self::state::AppState;

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
    dna_read_start_time: Option<Instant>,
    dna_read_in_progress: bool,
    waiting_message_logged: bool,
    #[cfg(feature = "branding")]
    branding_manager: BrandingManager,
    contact_copy_notification: Option<(String, Instant)>,
    icon_manager: crate::assets::IconManager,
    language: Language,
    /// Current auto-retry attempt (0 = first try, 1 = first retry, etc.)
    auto_retry_attempt: u32,
    /// Timestamp when the last retry cooldown started (None = not in cooldown)
    retry_cooldown_start: Option<Instant>,
    pcileech_test_state: Option<Arc<Mutex<crate::ui::pcileech_test::types::PcileechTestState>>>,
}

impl FirmwareToolApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let logger = Logger::new("AppLogger");
        logger.info(format!("{APP_TITLE} Tool started"));

        let window_manager = WindowManager::new();
        window_manager.setup_fonts(&cc.egui_ctx);
        window_manager.setup_style(&cc.egui_ctx);
        let mut icon_manager = crate::assets::IconManager::new();
        icon_manager.ensure_loaded(&cc.egui_ctx);

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
            dna_read_start_time: None,
            dna_read_in_progress: false,
            waiting_message_logged: false,
            #[cfg(feature = "branding")]
            branding_manager: BrandingManager::new(),
            contact_copy_notification: None,
            icon_manager,
            language: Language::English,
            auto_retry_attempt: 0,
            retry_cooldown_start: None,
            pcileech_test_state: None,
        }
    }
}

impl eframe::App for FirmwareToolApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force dark mode if system is overriding it
        if !ctx.global_style().visuals.dark_mode {
            self.window_manager.setup_style(ctx);
        }

        self.setup_ui_and_animation(ctx);
        self.update_window_size(ctx);
        self.handle_state_specific_logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_main_ui(ui);
    }
}
