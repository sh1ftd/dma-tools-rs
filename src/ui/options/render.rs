use super::panels::{render_dna_section, render_flash_section};
use crate::device_programmer::FlashingOption;
use eframe::egui::Ui;

pub fn render_flash_options(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    ui.vertical_centered(|ui| {
        ui.heading("Select Flashing Option");
        ui.add_space(12.0);
        render_flash_section(ui, on_select);
    });
}

pub fn render_dna_read_options(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    ui.vertical_centered(|ui| {
        ui.heading("Select DNA Read Option");
        ui.add_space(12.0);
        render_dna_section(ui, on_select);
    });
}
