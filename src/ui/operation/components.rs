use super::buttons::create_operation_button;
use super::types::OperationType;
use crate::utils::localization::{translate, TextKey};
use eframe::egui::Ui;

// Spacing constants
const SECTION_SPACING: f32 = 30.0;
const LABEL_SPACING: f32 = 12.0;

/// Renders the operation selection header with a title.
pub fn render_operation_header(ui: &mut Ui, lang: &crate::app::Language) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SelectOperation, lang));
        ui.add_space(SECTION_SPACING);
    });
}

/// Renders operation buttons that trigger the provided callback when clicked
pub fn render_operation_buttons(
    ui: &mut Ui, 
    on_select: &mut dyn FnMut(OperationType),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        render_operation_option(
            ui,
            OperationType::FlashFirmware,
            translate(TextKey::FlashFirmwareDesc, lang),
            on_select,
            lang,
        );

        ui.add_space(SECTION_SPACING);

        render_operation_option(
            ui,
            OperationType::ReadDNA,
            translate(TextKey::ReadDnaDesc, lang),
            on_select,
            lang,
        );
    });
}

/// Helper function to render a single operation option with its button and description.
fn render_operation_option(
    ui: &mut Ui,
    operation_type: OperationType,
    description: &str,
    on_select: &mut dyn FnMut(OperationType),
    lang: &crate::app::Language,
) {
    if create_operation_button(ui, operation_type, lang).clicked() {
        on_select(operation_type);
    }

    ui.add_space(LABEL_SPACING);
    ui.label(description);
}
