use eframe::egui::{self, RichText, Ui};

pub fn render_missing_file(ui: &mut Ui, file: &str, font_size: f32) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("•")
                .size(font_size)
                .strong()
                .color(egui::Color32::from_rgb(255, 120, 0)),
        );
        ui.label(RichText::new(file).size(font_size).monospace());
    });
}
