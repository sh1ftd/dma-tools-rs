use super::panels::{render_dna_section, render_flash_section};
use crate::device_programmer::FlashingOption;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::Ui;

pub fn render_flash_options(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SelectFlashingOption, lang));
        ui.add_space(12.0);
        render_flash_section(ui, on_select, lang);
    });
}

pub fn render_dna_read_options(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SelectDnaReadOption, lang));
        ui.add_space(12.0);
        render_dna_section(ui, on_select, lang);
    });
}
