use eframe::egui::{self, Align2, Color32, FontId, Ui, Vec2};

pub mod palette {
    use eframe::egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(30, 31, 36);
    pub const SURFACE: Color32 = Color32::from_rgb(38, 40, 45);
    pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(44, 46, 52);
    pub const SURFACE_RECESSED: Color32 = Color32::from_rgb(20, 21, 25);
    pub const STROKE: Color32 = Color32::from_rgb(72, 76, 86);
    pub const STROKE_SUBTLE: Color32 = Color32::from_rgb(56, 60, 68);
    pub const TEXT: Color32 = Color32::from_rgb(238, 240, 244);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(176, 180, 188);
    pub const TEXT_SUBTLE: Color32 = Color32::from_rgb(148, 152, 160);
    pub const PRIMARY: Color32 = Color32::from_rgb(66, 102, 145);
    pub const PRIMARY_HOVER: Color32 = Color32::from_rgb(84, 122, 168);
    pub const SUCCESS: Color32 = Color32::from_rgb(82, 190, 120);
    pub const ERROR: Color32 = Color32::from_rgb(255, 78, 88);
    pub const WARNING: Color32 = Color32::from_rgb(218, 156, 72);
    pub const INFO: Color32 = Color32::from_rgb(90, 145, 210);
}

const BUTTON_RADIUS: u8 = 8;
const BUTTON_TEXT_SIZE: f32 = 16.0;
const SECONDARY_FILL: Color32 = palette::SURFACE_ELEVATED;
const SECONDARY_HOVER_FILL: Color32 = Color32::from_rgb(62, 65, 74);
const SECONDARY_STROKE: Color32 = palette::STROKE;
const PRIMARY_FILL: Color32 = palette::PRIMARY;
const PRIMARY_HOVER_FILL: Color32 = palette::PRIMARY_HOVER;
const DISABLED_FILL: Color32 = palette::BACKGROUND;
const DISABLED_TEXT: Color32 = palette::TEXT_SUBTLE;

pub fn fitted_font_id(text: &str, max_size: f32, min_size: f32, available_width: f32) -> FontId {
    let character_count = text.chars().count().max(1) as f32;
    let width_factor = if text.chars().any(|ch| ch as u32 >= 0x2E80) {
        0.92
    } else {
        0.58
    };
    let estimated_width = character_count * max_size * width_factor;
    let font_size = if estimated_width > available_width {
        (max_size * available_width / estimated_width).clamp(min_size, max_size)
    } else {
        max_size
    };

    FontId::proportional(font_size)
}

pub fn secondary_button(ui: &mut Ui, text: &str, size: Vec2) -> egui::Response {
    styled_button(
        ui,
        text,
        size,
        SECONDARY_FILL,
        SECONDARY_HOVER_FILL,
        palette::TEXT,
        true,
    )
}

pub fn primary_button(ui: &mut Ui, text: &str, size: Vec2) -> egui::Response {
    styled_button(
        ui,
        text,
        size,
        PRIMARY_FILL,
        PRIMARY_HOVER_FILL,
        palette::TEXT,
        true,
    )
}

pub fn disabled_primary_button(ui: &mut Ui, text: &str, size: Vec2) -> egui::Response {
    styled_button(
        ui,
        text,
        size,
        DISABLED_FILL,
        DISABLED_FILL,
        DISABLED_TEXT,
        false,
    )
}

fn styled_button(
    ui: &mut Ui,
    text: &str,
    size: Vec2,
    fill: Color32,
    hover_fill: Color32,
    text_color: Color32,
    enabled: bool,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .min_size(size)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, SECONDARY_STROKE))
            .corner_radius(egui::CornerRadius::same(BUTTON_RADIUS)),
    );

    let button_fill = if enabled && response.hovered() {
        hover_fill
    } else {
        fill
    };

    ui.painter().rect_filled(
        response.rect,
        egui::CornerRadius::same(BUTTON_RADIUS),
        button_fill,
    );
    ui.painter().rect_stroke(
        response.rect,
        egui::CornerRadius::same(BUTTON_RADIUS),
        egui::Stroke::new(1.0, SECONDARY_STROKE),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        response.rect.center(),
        Align2::CENTER_CENTER,
        text,
        fitted_font_id(text, BUTTON_TEXT_SIZE, 12.5, response.rect.width() - 20.0),
        text_color,
    );

    response
}
