use super::types::OperationType;
use crate::ui::common;
use crate::ui::common::palette;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Align2, Ui};

const BUTTON_TEXT_SIZE: f32 = 17.5;
const BUTTON_WIDTH: f32 = 250.0;
const BUTTON_HEIGHT: f32 = 50.0;
const BUTTON_ROUNDING: u8 = 8;
const BUTTON_STROKE_COLOR: egui::Color32 = palette::STROKE;
const BUTTON_TEXT_COLOR: egui::Color32 = palette::TEXT;
const HOVER_BRIGHTNESS: u8 = 18;

mod colors {
    use eframe::egui::Color32;

    pub const FLASH_FIRMWARE: Color32 = super::palette::PRIMARY;
    pub const READ_DNA: Color32 = Color32::from_rgb(62, 118, 88);
    pub const DRIVERS: Color32 = super::palette::WARNING;
    pub const TEST_PCILEECH: Color32 = Color32::from_rgb(110, 74, 138);
}

pub fn create_operation_button(
    ui: &mut Ui,
    operation_type: OperationType,
    lang: &crate::utils::localization::Language,
) -> egui::Response {
    let (text, color) = match operation_type {
        OperationType::FlashFirmware => (
            translate(TextKey::FlashFirmware, lang),
            colors::FLASH_FIRMWARE,
        ),
        OperationType::ReadDNA => (translate(TextKey::ReadDna, lang), colors::READ_DNA),
        OperationType::Drivers => (translate(TextKey::Drivers, lang), colors::DRIVERS),
        OperationType::TestPcileech => (
            translate(TextKey::TestPcileech, lang),
            colors::TEST_PCILEECH,
        ),
    };

    let button_size = egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT);
    let response = ui.add(
        egui::Button::new("")
            .fill(color)
            .stroke(egui::Stroke::new(1.0, BUTTON_STROKE_COLOR))
            .corner_radius(egui::CornerRadius::same(BUTTON_ROUNDING))
            .min_size(button_size),
    );

    if response.hovered() {
        ui.painter().rect_filled(
            response.rect,
            egui::CornerRadius::same(BUTTON_ROUNDING),
            brighten_color(color, HOVER_BRIGHTNESS),
        );
        ui.painter().rect_stroke(
            response.rect,
            egui::CornerRadius::same(BUTTON_ROUNDING),
            egui::Stroke::new(1.0, BUTTON_STROKE_COLOR),
            egui::StrokeKind::Inside,
        );
    }

    ui.painter().text(
        response.rect.center(),
        Align2::CENTER_CENTER,
        text,
        common::fitted_font_id(text, BUTTON_TEXT_SIZE, 13.0, response.rect.width() - 24.0),
        BUTTON_TEXT_COLOR,
    );

    response
}

fn brighten_color(color: egui::Color32, amount: u8) -> egui::Color32 {
    egui::Color32::from_rgb(
        color.r().saturating_add(amount),
        color.g().saturating_add(amount),
        color.b().saturating_add(amount),
    )
}
