use super::components::{render_clear_button, render_log_entries, render_toggle_button};
use crate::utils::logger::Logger;
use eframe::egui::Ui;

/// Renders the collapsible operation log. Collapsed by default, showing only
/// a toggle button; the caller is responsible for growing the window to fit
/// once `expanded` becomes true (see `WindowSizeType::OperationResult`).
pub fn render_log_view(
    ui: &mut Ui,
    logger: &Logger,
    lang: &crate::utils::localization::Language,
    expanded: &mut bool,
) {
    render_toggle_button(ui, lang, expanded);

    if *expanded {
        render_log_entries(ui, logger); // Log entries in scrollable area
        render_clear_button(ui, logger, lang);
    }
}
