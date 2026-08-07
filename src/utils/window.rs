use crate::ui::common::palette;
use eframe::egui::{self, Color32, Context, FontFamily, FontId, TextStyle, Visuals};

pub enum WindowSizeType {
    FileCheck,
    MissingFiles,
    OperationSelection,
    FileSelection,
    FlashOptionSelection,
    ReadOptionSelection,
    FlashingProgress { log_expanded: bool },
    OperationResult { log_expanded: bool },
    Drivers,
    PcileechTest,
}

pub const WINDOW_WIDTH: f32 = 600.0;
pub const WINDOW_HEIGHT_INITIAL: f32 = 250.0;

pub const WINDOW_HEIGHT_FILE_CHECK: f32 = 330.0;
pub const WINDOW_HEIGHT_MISSING_FILES: f32 = 600.0;

pub const WINDOW_HEIGHT_OPERATION_SELECT: f32 = 670.0;

pub const WINDOW_HEIGHT_FLASH_FILE_SELECT: f32 = 360.0;

pub const WINDOW_HEIGHT_FLASH_OPTION_SELECT: f32 = 820.0;
pub const WINDOW_HEIGHT_READ_OPTION_SELECT: f32 = 530.0;

// The progress screen (spinner + technical info) is tall on its own, so it
// keeps the old combined-with-log height even with the log collapsed.
pub const WINDOW_HEIGHT_FLASHING_PROGRESS: f32 = 725.0;
// Result screens are shorter on average (icon + message + action buttons),
// so this can sit lower than the progress screen once the log is collapsed.
pub const WINDOW_HEIGHT_OPERATION_RESULT: f32 = 625.0;
// Extra room needed to fit the toggle button, scrollable entries, and clear
// button once the operation log is expanded.
pub const LOG_EXPANDED_EXTRA_HEIGHT: f32 = 220.0;

pub const WINDOW_HEIGHT_DRIVERS: f32 = 470.0;
pub const WINDOW_HEIGHT_PCILEECH_TEST: f32 = 575.0;

pub struct WindowManager {
    previous_height: Option<f32>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            previous_height: None,
        }
    }

    pub fn setup_fonts(&self, ctx: &Context) {
        let mut fonts = eframe::egui::FontDefinitions::default();

        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
            let font_name = "Segoe UI".to_string();

            fonts.font_data.insert(
                font_name.clone(),
                std::sync::Arc::new(eframe::egui::FontData::from_owned(font_data)),
            );

            if let Some(vec) = fonts
                .families
                .get_mut(&eframe::egui::FontFamily::Proportional)
            {
                vec.insert(0, font_name);
            }
        }

        let font_paths = [
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyh.ttf",
            "C:\\Windows\\Fonts\\simhei.ttf",
        ];

        for path in font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                // Determine name based on path
                let font_name = "Microsoft YaHei".to_string();

                fonts.font_data.insert(
                    font_name.clone(),
                    std::sync::Arc::new(eframe::egui::FontData::from_owned(font_data)),
                );

                // Insert into families
                if let Some(vec) = fonts
                    .families
                    .get_mut(&eframe::egui::FontFamily::Proportional)
                {
                    vec.insert(1, font_name.clone());
                }
                if let Some(vec) = fonts.families.get_mut(&eframe::egui::FontFamily::Monospace) {
                    vec.insert(1, font_name);
                }

                break;
            }
        }

        // Load Arabic font support
        let arabic_font_paths = [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\tahoma.ttf",
        ];

        for path in arabic_font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                let font_name = "Arabic Font".to_string();

                fonts.font_data.insert(
                    font_name.clone(),
                    std::sync::Arc::new(eframe::egui::FontData::from_owned(font_data)),
                );

                if let Some(vec) = fonts
                    .families
                    .get_mut(&eframe::egui::FontFamily::Proportional)
                {
                    vec.insert(1, font_name.clone());
                }
                if let Some(vec) = fonts.families.get_mut(&eframe::egui::FontFamily::Monospace) {
                    vec.insert(1, font_name);
                }

                break;
            }
        }

        fonts.font_data.insert(
            "phosphor".into(),
            std::sync::Arc::new(eframe::egui::FontData::from_static(
                egui_phosphor::Variant::Regular.font_bytes(),
            )),
        );

        if let Some(font_keys) = fonts
            .families
            .get_mut(&eframe::egui::FontFamily::Proportional)
        {
            font_keys.insert(1, "phosphor".into());
        }

        ctx.set_fonts(fonts);
    }

    pub fn setup_style(&self, ctx: &Context) {
        let mut visuals = Visuals::dark();

        #[cfg(feature = "branding")]
        {
            let (r, g, b) = crate::branding::BACKGROUND_COLOR;
            let bg_color = Color32::from_rgb(r, g, b);
            visuals.panel_fill = bg_color;
            visuals.window_fill = bg_color;
        }

        #[cfg(not(feature = "branding"))]
        {
            visuals.panel_fill = palette::BACKGROUND;
            visuals.window_fill = palette::BACKGROUND;
        }

        visuals.window_stroke.width = 1.0;
        visuals.window_stroke.color = palette::STROKE_SUBTLE;
        visuals.window_corner_radius = egui::CornerRadius::same(8);
        visuals.menu_corner_radius = egui::CornerRadius::same(8);
        visuals.extreme_bg_color = palette::SURFACE_RECESSED;
        visuals.faint_bg_color = palette::SURFACE;
        visuals.override_text_color = Some(palette::TEXT);
        visuals.selection.bg_fill = palette::PRIMARY;
        visuals.selection.stroke.color = palette::TEXT;
        visuals.widgets.noninteractive.bg_fill = palette::SURFACE;
        visuals.widgets.inactive.bg_fill = palette::SURFACE_ELEVATED;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(62, 65, 74);
        visuals.widgets.active.bg_fill = palette::PRIMARY;
        visuals.widgets.noninteractive.bg_stroke.width = 1.0;
        visuals.widgets.noninteractive.bg_stroke.color = palette::STROKE_SUBTLE;
        visuals.widgets.inactive.bg_stroke.color = palette::STROKE;
        visuals.widgets.hovered.bg_stroke.color = Color32::from_rgb(96, 102, 114);
        visuals.widgets.active.bg_stroke.color = palette::PRIMARY_HOVER;
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);

        ctx.set_visuals(visuals);

        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(14.0, 6.0);
            style.spacing.interact_size = egui::vec2(40.0, 30.0);
            style.text_styles.insert(
                TextStyle::Heading,
                FontId::new(19.0, FontFamily::Proportional),
            );
            style
                .text_styles
                .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
            style.text_styles.insert(
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            );
        });
    }

    // Only resizes in place. The window is centered once at startup (see
    // main.rs); re-centering on every view change made the window jump
    // around whenever its height changed (e.g. toggling the log).
    pub fn resize_window(&mut self, ctx: &Context, new_height: f32) {
        if self.previous_height != Some(new_height) {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::Vec2::new(WINDOW_WIDTH, new_height),
            ));

            self.previous_height = Some(new_height);
        }
    }

    fn get_height_for_type(&self, size_type: WindowSizeType) -> f32 {
        match size_type {
            WindowSizeType::FileCheck => WINDOW_HEIGHT_FILE_CHECK,
            WindowSizeType::MissingFiles => WINDOW_HEIGHT_MISSING_FILES,
            WindowSizeType::OperationSelection => WINDOW_HEIGHT_OPERATION_SELECT,
            WindowSizeType::FileSelection => WINDOW_HEIGHT_FLASH_FILE_SELECT,
            WindowSizeType::FlashOptionSelection => WINDOW_HEIGHT_FLASH_OPTION_SELECT,
            WindowSizeType::ReadOptionSelection => WINDOW_HEIGHT_READ_OPTION_SELECT,
            WindowSizeType::FlashingProgress { log_expanded } => {
                if log_expanded {
                    WINDOW_HEIGHT_FLASHING_PROGRESS + LOG_EXPANDED_EXTRA_HEIGHT
                } else {
                    WINDOW_HEIGHT_FLASHING_PROGRESS
                }
            }
            WindowSizeType::OperationResult { log_expanded } => {
                if log_expanded {
                    WINDOW_HEIGHT_OPERATION_RESULT + LOG_EXPANDED_EXTRA_HEIGHT
                } else {
                    WINDOW_HEIGHT_OPERATION_RESULT
                }
            }
            WindowSizeType::Drivers => WINDOW_HEIGHT_DRIVERS,
            WindowSizeType::PcileechTest => WINDOW_HEIGHT_PCILEECH_TEST,
        }
    }

    pub fn set_window_size(&mut self, ctx: &Context, size_type: WindowSizeType) {
        let target_height = self.get_height_for_type(size_type);
        self.resize_window(ctx, target_height);
    }
}
