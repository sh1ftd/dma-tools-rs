use super::components::{render_clear_button, render_log_entries};
use crate::utils::logger::Logger;
use eframe::egui::Ui;

pub fn render_log_view(ui: &mut Ui, logger: &Logger, lang: &crate::app::Language) {
    ui.heading(crate::utils::localization::translate(
        crate::utils::localization::TextKey::OperationLog,
        lang,
    ));

    render_log_entries(ui, logger); // Log entries in scrollable area

    render_clear_button(ui, logger, lang);
}
