use super::types::OperationType;
use crate::ui::common;
use crate::ui::common::palette;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Align2, Ui};

const BUTTON_TEXT_SIZE: f32 = 17.5;
const BUTTON_WIDTH: f32 = 250.0;
const BUTTON_HEIGHT: f32 = 50.0;
const BUTTON_ROUNDING: u8 = 8;
const BUTTON_ICON_SIZE: f32 = 19.0;
const BUTTON_ICON_GAP: f32 = 8.0;
const BUTTON_STROKE_COLOR: egui::Color32 = palette::STROKE;
const BUTTON_TEXT_COLOR: egui::Color32 = palette::TEXT;
const DRIVERS_SHADOW_COLOR: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
const HOVER_BRIGHTNESS: u8 = 18;

mod colors {
    use eframe::egui::Color32;

    pub const FLASH_FIRMWARE: Color32 = super::palette::PRIMARY;
    pub const READ_DNA: Color32 = Color32::from_rgb(62, 118, 88);
    pub const DRIVERS: Color32 = Color32::from_rgb(150, 100, 24);
    pub const TEST_PCILEECH: Color32 = Color32::from_rgb(110, 74, 138);
}

pub fn create_operation_button(
    ui: &mut Ui,
    operation_type: OperationType,
    lang: &crate::utils::localization::Language,
) -> egui::Response {
    let (text, icon, color, enable_shadow) = match operation_type {
        OperationType::FlashFirmware => (
            translate(TextKey::FlashFirmware, lang),
            egui_phosphor::regular::LIGHTNING,
            colors::FLASH_FIRMWARE,
            false,
        ),
        OperationType::ReadDNA => (
            translate(TextKey::ReadDna, lang),
            egui_phosphor::regular::EYE,
            colors::READ_DNA,
            false,
        ),
        OperationType::Drivers => (
            translate(TextKey::Drivers, lang),
            egui_phosphor::regular::WRENCH,
            colors::DRIVERS,
            true,
        ),
        OperationType::TestPcileech => (
            translate(TextKey::TestPcileech, lang),
            egui_phosphor::regular::CPU,
            colors::TEST_PCILEECH,
            false,
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

    paint_operation_label(ui, &response, icon, text, enable_shadow);

    response
}

fn paint_operation_label(
    ui: &Ui,
    response: &egui::Response,
    icon: &str,
    text: &str,
    use_shadow: bool,
) {
    let text_available_width = response.rect.width() - BUTTON_ICON_SIZE - BUTTON_ICON_GAP - 28.0;
    let font_id = common::fitted_font_id(text, BUTTON_TEXT_SIZE, 13.0, text_available_width);
    let text_width = common::estimated_text_width(text, font_id.size);
    let row_width = BUTTON_ICON_SIZE + BUTTON_ICON_GAP + text_width;
    let start_x = response.rect.center().x - row_width / 2.0;
    let center_y = response.rect.center().y;

    if use_shadow {
        ui.painter().text(
            egui::pos2(start_x + BUTTON_ICON_SIZE / 2.0 + 0.8, center_y + 0.8),
            Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(BUTTON_ICON_SIZE),
            DRIVERS_SHADOW_COLOR.gamma_multiply(0.27),
        );
        ui.painter().text(
            egui::pos2(
                start_x + BUTTON_ICON_SIZE + BUTTON_ICON_GAP + 0.8,
                center_y + 0.8,
            ),
            Align2::LEFT_CENTER,
            text,
            font_id.clone(),
            DRIVERS_SHADOW_COLOR.gamma_multiply(0.27),
        );
    }

    ui.painter().text(
        egui::pos2(start_x + BUTTON_ICON_SIZE / 2.0, center_y),
        Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(BUTTON_ICON_SIZE),
        BUTTON_TEXT_COLOR,
    );
    ui.painter().text(
        egui::pos2(start_x + BUTTON_ICON_SIZE + BUTTON_ICON_GAP, center_y),
        Align2::LEFT_CENTER,
        text,
        font_id.clone(),
        BUTTON_TEXT_COLOR,
    );
}

fn brighten_color(color: egui::Color32, amount: u8) -> egui::Color32 {
    egui::Color32::from_rgb(
        color.r().saturating_add(amount),
        color.g().saturating_add(amount),
        color.b().saturating_add(amount),
    )
}
