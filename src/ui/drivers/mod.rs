use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, RichText};
use std::process::Command;

pub fn render_drivers_screen(
    ui: &mut egui::Ui,
    on_back: &mut dyn FnMut(),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::DriversMenuTitle, lang));
        ui.add_space(30.0);

        ui.label(RichText::new(translate(TextKey::DataPortDrivers, lang)).strong().size(18.0));
        ui.add_space(10.0);        let req_admin = translate(TextKey::RequiresAdmin, lang);

        // Render FTDI installation button
        let ftdi_btn = ui.add(
            egui::Button::new(RichText::new(translate(TextKey::InstallFtdiDriver, lang)).size(16.0))
                .min_size(egui::vec2(250.0, 40.0))
        ).on_hover_text(req_admin);

        if ftdi_btn.clicked() {
            // Execute pnputil via PowerShell to elevate privileges
            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "Start-Process pnputil -ArgumentList '/add-driver tools\\FTDIBUS3\\ftdibus3.Inf /install' -Verb RunAs"
                ])
                .spawn();
        }

        ui.add_space(30.0);

        ui.label(RichText::new(translate(TextKey::JtagDrivers, lang)).strong().size(18.0));
        ui.add_space(10.0);

        if ui.add(
            egui::Button::new(RichText::new(translate(TextKey::OpenZadig, lang)).size(16.0))
                .min_size(egui::vec2(250.0, 40.0))
        ).clicked() {
            let _ = Command::new("tools\\zadig-2.9.exe").spawn();
        }

        ui.add_space(10.0);

        if ui.add(
            egui::Button::new(RichText::new(translate(TextKey::InstallCh347Driver, lang)).size(16.0))
                .min_size(egui::vec2(250.0, 40.0))
        ).clicked() {
            let _ = Command::new("tools\\CH341PAR_USB_DRIVER.EXE").spawn();
        }

    });

    ui.add_space(40.0);
    ui.separator();
    ui.add_space(15.0);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let button_width = 200.0;

        ui.add_space((available_width - button_width) / 2.0);

        if ui
            .add(
                egui::Button::new(translate(TextKey::MainMenu, lang))
                    .min_size(egui::vec2(button_width, 32.0)),
            )
            .clicked()
        {
            on_back();
        }
    });

    ui.add_space(15.0);
}
