use eframe::egui::{Color32, Context, Visuals};

pub enum WindowSizeType {
    FileCheck,
    MissingFiles,
    OperationSelection,
    FileSelection,
    FlashOptionSelection,
    ReadOptionSelection,
    OperationResult,
}

pub const WINDOW_WIDTH: f32 = 600.0;
pub const WINDOW_HEIGHT_INITIAL: f32 = 250.0;

pub const WINDOW_HEIGHT_FILE_CHECK: f32 = 250.0;
pub const WINDOW_HEIGHT_MISSING_FILES: f32 = 600.0;

pub const WINDOW_HEIGHT_OPERATION_SELECT: f32 = 350.0;

pub const WINDOW_HEIGHT_FLASH_FILE_SELECT: f32 = 290.0;

pub const WINDOW_HEIGHT_FLASH_OPTION_SELECT: f32 = 700.0;
pub const WINDOW_HEIGHT_READ_OPTION_SELECT: f32 = 440.0;

pub const WINDOW_HEIGHT_OPERATION_RESULT: f32 = 725.0;

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
                    vec.insert(0, font_name.clone());
                }
                if let Some(vec) = fonts.families.get_mut(&eframe::egui::FontFamily::Monospace) {
                    vec.insert(0, font_name);
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
            visuals.panel_fill = Color32::from_rgb(30, 30, 35);
            visuals.window_fill = Color32::from_rgb(30, 30, 35);
        }

        visuals.window_stroke.width = 1.0;
        visuals.window_stroke.color = Color32::from_gray(60);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(45, 45, 50);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(50, 50, 55);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 80);
        visuals.widgets.active.bg_fill = Color32::from_rgb(80, 80, 90);
        visuals.widgets.noninteractive.bg_stroke.width = 1.0;
        visuals.widgets.noninteractive.bg_stroke.color = Color32::from_gray(60);

        ctx.set_visuals(visuals);
    }

    pub fn resize_window(&mut self, ctx: &Context, new_height: f32) {
        if self.previous_height != Some(new_height) {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::Vec2::new(WINDOW_WIDTH, new_height),
            ));

            // Recenter the window when resizing
            if let Some(screen_size) = get_primary_monitor_size() {
                let x = (screen_size.x - WINDOW_WIDTH) / 2.0;
                let y = (screen_size.y - new_height) / 2.0;
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                    eframe::egui::Pos2::new(x, y),
                ));
            }

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
            WindowSizeType::OperationResult => WINDOW_HEIGHT_OPERATION_RESULT,
        }
    }

    pub fn set_window_size(&mut self, ctx: &Context, size_type: WindowSizeType) {
        let target_height = self.get_height_for_type(size_type);
        self.resize_window(ctx, target_height);
    }
}

fn get_primary_monitor_size() -> Option<eframe::egui::Vec2> {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    // SAFETY: GetSystemMetrics is a read-only Windows API call that returns screen dimensions
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN) as f32;
        let height = GetSystemMetrics(SM_CYSCREEN) as f32;

        if width > 0.0 && height > 0.0 {
            Some(eframe::egui::Vec2::new(width, height))
        } else {
            None
        }
    }
}
