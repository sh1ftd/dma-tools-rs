use super::buttons::create_operation_button;
use super::types::OperationType;
use eframe::egui::Ui;

// Spacing constants
const SECTION_SPACING: f32 = 30.0;
const LABEL_SPACING: f32 = 12.0;

/// Renders the operation selection header with a title.
pub fn render_operation_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("Select Operation");
        ui.add_space(SECTION_SPACING);
    });
}

/// Renders operation buttons that trigger the provided callback when clicked
pub fn render_operation_buttons(ui: &mut Ui, on_select: &mut dyn FnMut(OperationType)) {
    ui.vertical_centered(|ui| {
        render_operation_option(
            ui,
            OperationType::FlashFirmware,
            "Upload firmware to your device",
            on_select,
        );

        ui.add_space(SECTION_SPACING);

        render_operation_option(
            ui,
            OperationType::ReadDNA,
            "Retrieve the unique ID from your device",
            on_select,
        );
    });
}

/// Helper function to render a single operation option with its button and description.
fn render_operation_option(
    ui: &mut Ui,
    operation_type: OperationType,
    description: &str,
    on_select: &mut dyn FnMut(OperationType),
) {
    if create_operation_button(ui, operation_type).clicked() {
        on_select(operation_type);
    }

    ui.add_space(LABEL_SPACING);
    ui.label(description);
}
