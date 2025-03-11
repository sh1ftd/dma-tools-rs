use super::types::OperationType;
use eframe::egui::{self, RichText, Ui, Widget};

// Button styling constants
const BUTTON_TEXT_SIZE: f32 = 18.0;
const BUTTON_WIDTH: f32 = 250.0;
const BUTTON_HEIGHT: f32 = 50.0;

// Button color definitions
mod colors {
    use eframe::egui::Color32;

    pub const FLASH_FIRMWARE: Color32 = Color32::from_rgb(70, 100, 150);
    pub const READ_DNA: Color32 = Color32::from_rgb(50, 120, 50);
}

/// Trait for creating styled buttons in a UI
pub trait ButtonStyled {
    /// Creates a styled button with the given ID and button configuration
    fn button_styled(&mut self, id: impl Into<String>, button: egui::Button) -> egui::Response;
}

impl ButtonStyled for Ui {
    fn button_styled(&mut self, _id: impl Into<String>, button: egui::Button) -> egui::Response {
        button.ui(self)
    }
}

/// Creates a button for the specified operation type with appropriate styling
pub fn create_operation_button(ui: &mut Ui, operation_type: OperationType) -> egui::Response {
    let (text, color) = match operation_type {
        OperationType::FlashFirmware => ("Flash Firmware", colors::FLASH_FIRMWARE),
        OperationType::ReadDNA => ("Read Device DNA", colors::READ_DNA),
    };

    let button_size = egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT);

    ui.button_styled(
        text,
        egui::Button::new(RichText::new(text).size(BUTTON_TEXT_SIZE))
            .fill(color)
            .min_size(button_size),
    )
}
