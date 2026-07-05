use crate::device_programmer::FlashingOption;
use crate::ui::common;
use crate::ui::common::palette;
use eframe::egui::{self, Align2, Ui};

const BUTTON_HEIGHT: f32 = 62.0;
const BUTTON_ROUNDING: u8 = 8;
const BUTTON_STROKE_WIDTH: f32 = 1.0;
const BUTTON_FILL: egui::Color32 = palette::SURFACE;
const BUTTON_HOVER_FILL: egui::Color32 = palette::SURFACE_ELEVATED;
const BUTTON_STROKE_COLOR: egui::Color32 = palette::STROKE;
const BUTTON_TEXT_COLOR: egui::Color32 = palette::TEXT;
const BUTTON_DESCRIPTION_COLOR: egui::Color32 = palette::TEXT_MUTED;
const BUTTON_FONT_SIZE: f32 = 17.5;
const BUTTON_DESCRIPTION_SIZE: f32 = 13.5;
const ACCENT_WIDTH: f32 = 4.0;
const TEXT_LEFT_PADDING: f32 = 18.0;
const LABEL_Y_OFFSET: f32 = -10.0;
const DESCRIPTION_Y_OFFSET: f32 = 12.0;

pub fn render_colored_option_button(
    ui: &mut Ui,
    label: &str,
    description: &str,
    accent_color: egui::Color32,
    option_fn: impl FnOnce() -> FlashingOption,
    on_select: &mut dyn FnMut(FlashingOption),
) {
    let button = egui::Button::new("")
        .min_size(egui::vec2(ui.available_width(), BUTTON_HEIGHT))
        .fill(if ui.visuals().dark_mode {
            BUTTON_FILL
        } else {
            ui.visuals().widgets.inactive.bg_fill
        })
        .stroke(egui::Stroke::new(BUTTON_STROKE_WIDTH, BUTTON_STROKE_COLOR))
        .corner_radius(egui::CornerRadius::same(BUTTON_ROUNDING));

    let response = ui.add(button);

    if response.hovered() {
        draw_hover_fill(ui, &response);
    }

    draw_accent(ui, &response, accent_color);
    draw_option_text(ui, &response, label, description);
    response.clone().on_hover_text(description);

    if response.clicked() {
        on_select(option_fn());
    }
}

fn draw_hover_fill(ui: &mut Ui, response: &egui::Response) {
    ui.painter().rect_filled(
        response.rect,
        egui::CornerRadius::same(BUTTON_ROUNDING),
        BUTTON_HOVER_FILL,
    );
}

fn draw_accent(ui: &Ui, response: &egui::Response, accent_color: egui::Color32) {
    let bar_rect = egui::Rect::from_min_size(
        response.rect.min,
        egui::vec2(ACCENT_WIDTH, response.rect.height()),
    );

    let left_only_rounding = egui::CornerRadius {
        nw: BUTTON_ROUNDING,
        ne: 0,
        sw: BUTTON_ROUNDING,
        se: 0,
    };

    ui.painter()
        .rect_filled(bar_rect, left_only_rounding, accent_color);
}

fn draw_option_text(ui: &Ui, response: &egui::Response, label: &str, description: &str) {
    let text_x = response.rect.left() + TEXT_LEFT_PADDING;
    let center_y = response.rect.center().y;
    let text_width = response.rect.width() - TEXT_LEFT_PADDING - 14.0;

    ui.painter().text(
        egui::pos2(text_x, center_y + LABEL_Y_OFFSET),
        Align2::LEFT_CENTER,
        label,
        common::fitted_font_id(label, BUTTON_FONT_SIZE, 13.0, text_width),
        BUTTON_TEXT_COLOR,
    );

    ui.painter().text(
        egui::pos2(text_x, center_y + DESCRIPTION_Y_OFFSET),
        Align2::LEFT_CENTER,
        description,
        common::fitted_font_id(description, BUTTON_DESCRIPTION_SIZE, 11.5, text_width),
        BUTTON_DESCRIPTION_COLOR,
    );
}
