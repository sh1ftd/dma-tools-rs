use crate::ui::common;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, RichText};
use std::process::Command;

const DRIVER_BUTTON_WIDTH: f32 = 250.0;
const DRIVER_BUTTON_HEIGHT: f32 = 40.0;
const MAIN_MENU_BUTTON_WIDTH: f32 = 200.0;
const MAIN_MENU_BUTTON_HEIGHT: f32 = 32.0;

pub fn render_drivers_screen(
    ui: &mut egui::Ui,
    on_back: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::DriversMenuTitle, lang));
        ui.add_space(30.0);

        ui.label(RichText::new(translate(TextKey::DataPortDrivers, lang)).strong().size(18.0));
        ui.add_space(10.0);

        let req_admin = translate(TextKey::RequiresAdmin, lang);

        // Render FTDI installation button
        let ftdi_btn = common::primary_icon_button(
            ui,
            Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
            translate(TextKey::InstallFtdiDriver, lang),
            egui::vec2(DRIVER_BUTTON_WIDTH, DRIVER_BUTTON_HEIGHT),
        )
        .on_hover_text(req_admin);

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

        if common::secondary_icon_button(
            ui,
            Some(egui_phosphor::regular::WRENCH),
            translate(TextKey::OpenZadig, lang),
            egui::vec2(DRIVER_BUTTON_WIDTH, DRIVER_BUTTON_HEIGHT),
        )
        .clicked()
        {
            let _ = Command::new("tools\\zadig-2.9.exe").spawn();
        }

        ui.add_space(10.0);

        if common::primary_icon_button(
            ui,
            Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
            translate(TextKey::InstallCh347Driver, lang),
            egui::vec2(DRIVER_BUTTON_WIDTH, DRIVER_BUTTON_HEIGHT),
        )
        .clicked()
        {
            let _ = Command::new("tools\\CH341PAR_USB_DRIVER.EXE").spawn();
        }
    });

    ui.add_space(40.0);
    ui.separator();
    ui.add_space(15.0);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let button_width = MAIN_MENU_BUTTON_WIDTH;

        ui.add_space((available_width - button_width) / 2.0);

        if common::secondary_icon_button(
            ui,
            Some(egui_phosphor::regular::HOUSE),
            translate(TextKey::MainMenu, lang),
            egui::vec2(button_width, MAIN_MENU_BUTTON_HEIGHT),
        )
        .clicked()
        {
            on_back();
        }
    });

    ui.add_space(15.0);
}
