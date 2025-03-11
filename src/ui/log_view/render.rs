use super::components::{render_clear_button, render_log_entries};
use crate::utils::logger::Logger;
use eframe::egui::Ui;

pub fn render_log_view(ui: &mut Ui, logger: &Logger) {
    ui.heading("Operation Log");

    render_log_entries(ui, logger); // Log entries in scrollable area

    render_clear_button(ui, logger);
}
