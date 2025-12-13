use crate::device_programmer::FlashingOption;
use eframe::egui::{self, Align2, Ui};

// Styling constants
const BUTTON_HEIGHT: f32 = 54.0;
const BUTTON_ROUNDING: u8 = 12;
const BUTTON_STROKE_WIDTH: f32 = 1.0;
const BUTTON_STROKE_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 80, 90);
const BUTTON_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const BUTTON_FONT_SIZE: f32 = 18.0;

// Animation constants
const COLOR_BRIGHTNESS_INCREASE: u8 = 20;
const HIGHLIGHT_BAR_BASE_WIDTH: f32 = 4.0;
const PULSE_MIN: f32 = 0.85;
const PULSE_RANGE: f32 = 0.15;
const PULSE_SPEED: f32 = 3.0;
const HIGHLIGHT_BASE_COLOR: (u8, u8, u8) = (100, 180, 200);
const HIGHLIGHT_COLOR_RANGE: f32 = 55.0;
const HIGHLIGHT_COLOR_SPEED: f32 = 4.0;

/// Renders an option button with interactive hover effects
pub fn render_colored_option_button(
    ui: &mut Ui,
    label: &str,
    tooltip: &str,
    color: egui::Color32,
    option_fn: impl FnOnce() -> FlashingOption,
    on_select: &mut dyn FnMut(FlashingOption),
) {
    let hover_color = brighten_color(color, COLOR_BRIGHTNESS_INCREASE);

    let button = egui::Button::new("")
        .min_size(egui::vec2(ui.available_width(), BUTTON_HEIGHT))
        .fill(color)
        .stroke(egui::Stroke::new(BUTTON_STROKE_WIDTH, BUTTON_STROKE_COLOR))
        .corner_radius(egui::CornerRadius::same(BUTTON_ROUNDING));

    let response = ui.add(button);

    if response.hovered() {
        draw_hover_effects(ui, &response, hover_color, tooltip);
    }

    draw_centered_text(ui, &response, label);

    if response.clicked() {
        on_select(option_fn());
    }
}

/// Handles hover state rendering including background, highlight bar and tooltip
fn draw_hover_effects(
    ui: &mut Ui,
    response: &egui::Response,
    hover_color: egui::Color32,
    tooltip: &str,
) {
    ui.painter().rect_filled(
        response.rect,
        egui::CornerRadius::same(BUTTON_ROUNDING),
        hover_color,
    );

    let time = ui.ctx().input(|i| i.time) as f32;

    draw_animated_highlight_bar(ui, response, time);

    response.clone().on_hover_text(tooltip);
}

/// Draws an animated vertical highlight on the button's left edge
fn draw_animated_highlight_bar(ui: &Ui, response: &egui::Response, time: f32) {
    let pulse = PULSE_MIN + (PULSE_RANGE * (time * PULSE_SPEED).sin().abs());
    let bar_width = HIGHLIGHT_BAR_BASE_WIDTH * pulse;

    let bar_rect = egui::Rect::from_min_size(
        response.rect.min,
        egui::vec2(bar_width, response.rect.height()),
    );

    // Round only the left corners to match button shape
    let left_only_rounding = egui::CornerRadius {
        nw: BUTTON_ROUNDING,
        ne: 0,
        sw: BUTTON_ROUNDING,
        se: 0,
    };

    let highlight_color = egui::Color32::from_rgb(
        HIGHLIGHT_BASE_COLOR.0,
        HIGHLIGHT_BASE_COLOR.1,
        (HIGHLIGHT_BASE_COLOR.2 as f32
            + HIGHLIGHT_COLOR_RANGE * (time * HIGHLIGHT_COLOR_SPEED).sin()) as u8,
    );

    ui.painter()
        .rect_filled(bar_rect, left_only_rounding, highlight_color);
}

/// Draws the label text centered on the button
fn draw_centered_text(ui: &Ui, response: &egui::Response, label: &str) {
    ui.painter().text(
        response.rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(BUTTON_FONT_SIZE),
        BUTTON_TEXT_COLOR,
    );
}

/// Creates a brighter version of a color for hover effects
fn brighten_color(color: egui::Color32, amount: u8) -> egui::Color32 {
    egui::Color32::from_rgb(
        color.r().saturating_add(amount),
        color.g().saturating_add(amount),
        color.b().saturating_add(amount),
    )
}
