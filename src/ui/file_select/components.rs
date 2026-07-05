use crate::ui::common::palette;
use eframe::egui::{RichText, Ui};

pub fn render_missing_file(ui: &mut Ui, file: &str, font_size: f32) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("•")
                .size(font_size)
                .strong()
                .color(palette::WARNING),
        );
        ui.label(RichText::new(file).size(font_size).monospace());
    });
}
