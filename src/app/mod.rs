use crate::APP_TITLE;

#[cfg(feature = "branding")]
use crate::branding::BrandingManager;

use crate::pcileech_test::PcileechTestController;
use crate::utils::localization::Language;
use crate::utils::logger::Logger;
use crate::utils::window::WindowManager;
use eframe::egui;
use std::time::Instant;

mod flows;
mod footer;
mod lifecycle;
mod screens;
mod state;

use self::state::AppState;
use flows::{FileCheckFlow, FirmwareScanFlow, OperationFlow};

pub struct FirmwareToolApp {
    window_manager: WindowManager,
    state: AppState,
    file_check: FileCheckFlow,
    firmware_scan: FirmwareScanFlow,
    operation: OperationFlow,
    logger: Logger,
    previous_log_state: bool,
    #[cfg(feature = "branding")]
    branding_manager: BrandingManager,
    contact_copy_notification: Option<(String, Instant)>,
    icon_manager: crate::assets::IconManager,
    language: Language,
    pcileech_test: PcileechTestController,
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
            file_check: FileCheckFlow::new(),
            firmware_scan: FirmwareScanFlow::new(),
            operation: OperationFlow::new(logger.clone()),
            logger,
            previous_log_state: false,
            #[cfg(feature = "branding")]
            branding_manager: BrandingManager::new(),
            contact_copy_notification: None,
            icon_manager,
            language: Language::English,
            pcileech_test: PcileechTestController::new(),
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
