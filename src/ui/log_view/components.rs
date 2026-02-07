use super::styles::get_log_text_style;
use crate::utils::logger::Logger;
use eframe::egui::{ScrollArea, Ui};

const LOG_FONT_SIZE: f32 = 14.0;
const LOG_AREA_MAX_HEIGHT: f32 = 200.0;

pub fn render_log_entries(ui: &mut Ui, logger: &Logger) {
    ScrollArea::vertical()
        .max_height(LOG_AREA_MAX_HEIGHT)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            let entries = logger.get_entries();

            for entry in entries {
                let timestamp = logger.format_timestamp(entry.timestamp);
                let text = format!("[{}] {}", timestamp, entry.message);
                let rich_text = get_log_text_style(text, &entry.level, LOG_FONT_SIZE);
                ui.label(rich_text);
            }
        });
}

pub fn render_clear_button(ui: &mut Ui, logger: &Logger, lang: &crate::app::Language) {
    ui.horizontal(|ui| {
        if ui
            .button(crate::utils::localization::translate(
                crate::utils::localization::TextKey::ClearLog,
                lang,
            ))
            .clicked()
        {
            logger.clear();
        }
    });
}
