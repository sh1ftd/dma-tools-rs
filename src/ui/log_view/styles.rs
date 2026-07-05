use crate::ui::common::palette;
use crate::utils::logger::LogLevel;
use eframe::egui::{FontId, RichText};

pub fn get_log_text_style(text: String, level: &LogLevel, font_size: f32) -> RichText {
    let mut rich_text = RichText::new(text).font(FontId::proportional(font_size));

    rich_text = match level {
        LogLevel::Info => rich_text.color(palette::TEXT_MUTED),
        LogLevel::Success => rich_text.color(palette::SUCCESS),
        LogLevel::Warning => rich_text.color(palette::WARNING),
        LogLevel::Error => rich_text.color(palette::ERROR),
        LogLevel::Command => rich_text.color(palette::INFO).strong(),
        LogLevel::Output => rich_text.color(palette::TEXT),
    };

    rich_text
}
