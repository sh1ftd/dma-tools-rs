mod buttons;
mod components;
mod types;

use components::{render_operation_buttons, render_operation_header};
pub use types::OperationType;

pub fn render_operation_selection(
    ui: &mut eframe::egui::Ui,
    on_select: &mut dyn FnMut(OperationType),
    lang: &crate::app::Language,
) {
    render_operation_header(ui, lang);
    render_operation_buttons(ui, on_select, lang);
}
