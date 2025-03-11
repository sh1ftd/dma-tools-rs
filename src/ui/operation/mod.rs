mod buttons;
mod components;
mod types;

use components::{render_operation_buttons, render_operation_header};
pub use types::OperationType;

pub fn render_operation_selection(
    ui: &mut eframe::egui::Ui,
    on_select: &mut dyn FnMut(OperationType),
) {
    render_operation_header(ui);
    render_operation_buttons(ui, on_select);
}
