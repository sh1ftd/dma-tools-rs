use super::buttons::render_colored_option_button;
use crate::device_programmer::FlashingOption;
use eframe::egui::{self, RichText, Ui};

// UI styling constants
const PANEL_ROUNDING: u8 = 12;
const PANEL_STROKE_WIDTH: f32 = 1.0;
const PANEL_MARGIN: i8 = 20;
const SECTION_SPACE: f32 = 8.0;
const BUTTON_SPACE: f32 = 12.0;
const HEADING_SIZE: f32 = 18.0;
const SECTION_BOTTOM_SPACE: f32 = 18.0;

// Interface colors
const STROKE_COLOR: egui::Color32 = egui::Color32::from_rgb(60, 60, 70);
const CH347_COLOR: egui::Color32 = egui::Color32::from_rgb(50, 70, 90);
const RS232_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 60, 90);

pub fn render_flash_section(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    render_panel(ui, |ui| {
        render_section_header(ui, "CH347 Options");
        render_ch347_options(ui, on_select);

        ui.add_space(SECTION_BOTTOM_SPACE);

        render_section_header(ui, "RS232 Options");
        render_rs232_options(ui, on_select);
    });
}

pub fn render_dna_section(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    render_panel(ui, |ui| {
        render_colored_option_button(
            ui,
            "CH347 - DNA Read: 35T, 75T, 100T",
            "Read DNA from 35T, 75T, or 100T using CH347 interface",
            CH347_COLOR,
            || FlashingOption::DnaCH347,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            "RS232 - DNA Read: 35T",
            "Read DNA from 35T boards using RS232 interface",
            RS232_COLOR,
            || FlashingOption::DnaRS232_35T,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            "RS232 - DNA Read: 75T",
            "Read DNA from 75T boards using RS232 interface",
            RS232_COLOR,
            || FlashingOption::DnaRS232_75T,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            "RS232 - DNA Read: 100T",
            "Read DNA from 100T boards using RS232 interface",
            RS232_COLOR,
            || FlashingOption::DnaRS232_100T,
            on_select,
        );
    });
}

// Shared UI component functions

fn render_panel(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        egui::Frame::NONE
            .fill(ui.style().visuals.extreme_bg_color)
            .corner_radius(egui::CornerRadius::same(PANEL_ROUNDING))
            .stroke(egui::Stroke::new(PANEL_STROKE_WIDTH, STROKE_COLOR))
            .inner_margin(egui::Margin::same(PANEL_MARGIN))
            .show(ui, add_contents);
    });
}

fn render_section_header(ui: &mut Ui, title: &str) {
    ui.vertical_centered(|ui| {
        ui.heading(RichText::new(title).size(HEADING_SIZE));
    });
    ui.add_space(SECTION_SPACE);
    ui.separator();
    ui.add_space(BUTTON_SPACE);
}

fn render_ch347_options(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    render_colored_option_button(
        ui,
        "CH347 - 35T",
        "For 35T boards using CH347 interface",
        CH347_COLOR,
        || FlashingOption::CH347_35T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        "CH347 - 75T",
        "For 75T boards using CH347 interface",
        CH347_COLOR,
        || FlashingOption::CH347_75T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        "CH347 - Stark100T",
        "For Stark100T boards using CH347 interface",
        CH347_COLOR,
        || FlashingOption::CH347_100T,
        on_select,
    );
}

fn render_rs232_options(ui: &mut Ui, on_select: &mut dyn FnMut(FlashingOption)) {
    render_colored_option_button(
        ui,
        "RS232 - 35T",
        "For 35T boards using RS232 interface",
        RS232_COLOR,
        || FlashingOption::RS232_35T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        "RS232 - 75T",
        "For 75T boards using RS232 interface",
        RS232_COLOR,
        || FlashingOption::RS232_75T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        "RS232 - 100T",
        "For 100T boards using RS232 interface",
        RS232_COLOR,
        || FlashingOption::RS232_100T,
        on_select,
    );
}
