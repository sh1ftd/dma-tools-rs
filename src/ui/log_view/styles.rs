use crate::utils::logger::LogLevel;
use eframe::egui::{Color32, FontId, RichText};

pub fn get_log_text_style(text: String, level: &LogLevel, font_size: f32) -> RichText {
    let mut rich_text = RichText::new(text).font(FontId::proportional(font_size));

    rich_text = match level {
        LogLevel::Info => rich_text.color(Color32::LIGHT_GRAY),
        LogLevel::Success => rich_text.color(Color32::GREEN),
        LogLevel::Warning => rich_text.color(Color32::YELLOW),
        LogLevel::Error => rich_text.color(Color32::RED),
        LogLevel::Command => rich_text.color(Color32::LIGHT_BLUE).strong(),
        LogLevel::Output => rich_text.color(Color32::WHITE),
    };

    rich_text
}
