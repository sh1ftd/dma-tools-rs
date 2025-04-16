use eframe::egui::{Color32, Context, Visuals};
use std::sync::Once;

pub enum WindowSizeType {
    FileCheck,
    MissingFiles,
    OperationSelection,
    FileSelection,
    FlashOptionSelection,
    ReadOptionSelection,
    OperationResult,
}

static SETUP: Once = Once::new();

pub const WINDOW_WIDTH: f32 = 600.0;
pub const WINDOW_HEIGHT_INITIAL: f32 = 200.0;

pub const WINDOW_HEIGHT_FILE_CHECK: f32 = 200.0;
pub const WINDOW_HEIGHT_MISSING_FILES: f32 = 475.0;

pub const WINDOW_HEIGHT_OPERATION_SELECT: f32 = 300.0;

pub const WINDOW_HEIGHT_FLASH_FILE_SELECT: f32 = 240.0;

pub const WINDOW_HEIGHT_FLASH_OPTION_SELECT: f32 = 580.0;
pub const WINDOW_HEIGHT_READ_OPTION_SELECT: f32 = 320.0;

pub const WINDOW_HEIGHT_OPERATION_RESULT: f32 = 675.0;

pub struct WindowManager {
    previous_height: Option<f32>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            previous_height: None,
        }
    }

    pub fn setup_style(&self, ctx: &Context) {
        // Set up styles once
        SETUP.call_once(|| {
            // Apply a dark theme
            let mut visuals = Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(30, 30, 35);
            visuals.window_fill = Color32::from_rgb(30, 30, 35);
            visuals.window_stroke.width = 1.0;
            visuals.window_stroke.color = Color32::from_gray(60);
            visuals.window_shadow.extrusion = 0.0;
            visuals.window_rounding = eframe::epaint::Rounding::same(10.0);
            visuals.window_fill = Color32::from_rgb(30, 30, 35);
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(45, 45, 50);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(50, 50, 55);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 80);
            visuals.widgets.active.bg_fill = Color32::from_rgb(80, 80, 90);
            visuals.widgets.noninteractive.bg_stroke.width = 1.0;
            visuals.widgets.noninteractive.bg_stroke.color = Color32::from_gray(60);

            // Apply the visual styles (colors only)
            ctx.set_visuals(visuals);
        });
    }

    pub fn resize_window(&mut self, frame: &mut eframe::Frame, new_height: f32) {
        // Get current window size and position
        let current_size = frame.info().window_info.size;
        let current_pos = frame.info().window_info.position.unwrap_or_default();

        // Only proceed if height actually changed
        if self.previous_height != Some(new_height) {
            // Calculate how much to shift the Y position to keep the window centered
            let height_diff = new_height - current_size.y;
            let new_y = current_pos.y - (height_diff / 2.0);

            // Set new position
            frame.set_window_pos([current_pos.x, new_y].into());

            // Set new size
            frame.set_window_size([WINDOW_WIDTH, new_height].into());

            // Store the new height for the next comparison
            self.previous_height = Some(new_height);
        }
    }

    /// Get the window size for a given window type
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

    /// Set the window to a specific size type
    pub fn set_window_size(&mut self, frame: &mut eframe::Frame, size_type: WindowSizeType) {
        let target_height = self.get_height_for_type(size_type);

        // Use the resizing function that preserves centering
        self.resize_window(frame, target_height);
    }
}
