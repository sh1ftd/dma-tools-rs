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
const BUTTON_ICON_SIZE: f32 = 17.0;
const BUTTON_ICON_GAP: f32 = 6.0;
const BUTTON_HORIZONTAL_PADDING: f32 = 20.0;
const SECONDARY_FILL: Color32 = palette::SURFACE_ELEVATED;
const SECONDARY_HOVER_FILL: Color32 = Color32::from_rgb(62, 65, 74);
const SECONDARY_STROKE: Color32 = palette::STROKE;
const PRIMARY_FILL: Color32 = palette::PRIMARY;
const PRIMARY_HOVER_FILL: Color32 = palette::PRIMARY_HOVER;
const DISABLED_FILL: Color32 = palette::BACKGROUND;
const DISABLED_TEXT: Color32 = palette::TEXT_SUBTLE;

struct ButtonStyle {
    fill: Color32,
    hover_fill: Color32,
    text_color: Color32,
    enabled: bool,
}

pub fn estimated_text_width(text: &str, font_size: f32) -> f32 {
    let character_count = text.chars().count().max(1) as f32;
    let width_factor = if text.chars().any(|ch| ch as u32 >= 0x2E80) {
        0.92
    } else {
        0.58
    };
    character_count * font_size * width_factor
}

pub fn fitted_font_id(text: &str, max_size: f32, min_size: f32, available_width: f32) -> FontId {
    let estimated_width = estimated_text_width(text, max_size);
    let font_size = if estimated_width > available_width {
        (max_size * available_width / estimated_width).clamp(min_size, max_size)
    } else {
        max_size
    };

    FontId::proportional(font_size)
}

pub fn secondary_icon_button(
    ui: &mut Ui,
    icon: Option<&str>,
    text: &str,
    size: Vec2,
) -> egui::Response {
    styled_button(
        ui,
        icon,
        text,
        size,
        ButtonStyle {
            fill: SECONDARY_FILL,
            hover_fill: SECONDARY_HOVER_FILL,
            text_color: palette::TEXT,
            enabled: true,
        },
    )
}

pub fn primary_icon_button(
    ui: &mut Ui,
    icon: Option<&str>,
    text: &str,
    size: Vec2,
) -> egui::Response {
    styled_button(
        ui,
        icon,
        text,
        size,
        ButtonStyle {
            fill: PRIMARY_FILL,
            hover_fill: PRIMARY_HOVER_FILL,
            text_color: palette::TEXT,
            enabled: true,
        },
    )
}

pub fn disabled_primary_icon_button(
    ui: &mut Ui,
    icon: Option<&str>,
    text: &str,
    size: Vec2,
) -> egui::Response {
    styled_button(
        ui,
        icon,
        text,
        size,
        ButtonStyle {
            fill: DISABLED_FILL,
            hover_fill: DISABLED_FILL,
            text_color: DISABLED_TEXT,
            enabled: false,
        },
    )
}

fn styled_button(
    ui: &mut Ui,
    icon: Option<&str>,
    text: &str,
    size: Vec2,
    style: ButtonStyle,
) -> egui::Response {
    let response = ui.add_enabled(
        style.enabled,
        egui::Button::new("")
            .min_size(size)
            .fill(style.fill)
            .stroke(egui::Stroke::new(1.0, SECONDARY_STROKE))
            .corner_radius(egui::CornerRadius::same(BUTTON_RADIUS)),
    );

    let button_fill = if style.enabled && response.hovered() {
        style.hover_fill
    } else {
        style.fill
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
    paint_button_label(ui, &response, icon, text, style.text_color);

    response
}

fn paint_button_label(
    ui: &Ui,
    response: &egui::Response,
    icon: Option<&str>,
    text: &str,
    text_color: Color32,
) {
    let icon_width = if icon.is_some() {
        BUTTON_ICON_SIZE
    } else {
        0.0
    };
    let gap_width = if icon.is_some() { BUTTON_ICON_GAP } else { 0.0 };
    let text_available_width =
        response.rect.width() - BUTTON_HORIZONTAL_PADDING * 2.0 - icon_width - gap_width;
    let font_id = fitted_font_id(text, BUTTON_TEXT_SIZE, 12.5, text_available_width);

    if let Some(icon) = icon {
        let text_width = estimated_text_width(text, font_id.size);
        let row_width = icon_width + gap_width + text_width;
        let start_x = response.rect.center().x - row_width / 2.0;
        let center_y = response.rect.center().y;

        ui.painter().text(
            egui::pos2(start_x + icon_width / 2.0, center_y),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(BUTTON_ICON_SIZE),
            text_color,
        );
        ui.painter().text(
            egui::pos2(start_x + icon_width + gap_width, center_y),
            Align2::LEFT_CENTER,
            text,
            font_id,
            text_color,
        );
    } else {
        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            text,
            font_id,
            text_color,
        );
    }
}
