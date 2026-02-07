use super::buttons::render_colored_option_button;
use crate::device_programmer::FlashingOption;
use crate::utils::localization::{TextKey, translate};
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

pub fn render_flash_section(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    render_panel(ui, |ui| {
        render_section_header(ui, translate(TextKey::Ch347Options, lang));
        render_ch347_options(ui, on_select, lang);

        ui.add_space(SECTION_BOTTOM_SPACE);

        render_section_header(ui, translate(TextKey::Rs232Options, lang));
        render_rs232_options(ui, on_select, lang);
    });
}

pub fn render_dna_section(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    render_panel(ui, |ui| {
        render_colored_option_button(
            ui,
            translate(TextKey::Dna_Ch347_Label, lang),
            translate(TextKey::Dna_Ch347_Desc, lang),
            CH347_COLOR,
            || FlashingOption::DnaCH347,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            translate(TextKey::Dna_Rs232_35T_Label, lang),
            translate(TextKey::Dna_Rs232_35T_Desc, lang),
            RS232_COLOR,
            || FlashingOption::DnaRS232_35T,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            translate(TextKey::Dna_Rs232_75T_Label, lang),
            translate(TextKey::Dna_Rs232_75T_Desc, lang),
            RS232_COLOR,
            || FlashingOption::DnaRS232_75T,
            on_select,
        );

        ui.add_space(BUTTON_SPACE);

        render_colored_option_button(
            ui,
            translate(TextKey::Dna_Rs232_100T_Label, lang),
            translate(TextKey::Dna_Rs232_100T_Desc, lang),
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

fn render_ch347_options(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    render_colored_option_button(
        ui,
        translate(TextKey::Ch347_35T_Label, lang),
        translate(TextKey::Ch347_35T_Desc, lang),
        CH347_COLOR,
        || FlashingOption::CH347_35T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        translate(TextKey::Ch347_75T_Label, lang),
        translate(TextKey::Ch347_75T_Desc, lang),
        CH347_COLOR,
        || FlashingOption::CH347_75T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        translate(TextKey::Ch347_100T_Label, lang),
        translate(TextKey::Ch347_100T_Desc, lang),
        CH347_COLOR,
        || FlashingOption::CH347_100T,
        on_select,
    );
}

fn render_rs232_options(
    ui: &mut Ui,
    on_select: &mut dyn FnMut(FlashingOption),
    lang: &crate::app::Language,
) {
    render_colored_option_button(
        ui,
        translate(TextKey::Rs232_35T_Label, lang),
        translate(TextKey::Rs232_35T_Desc, lang),
        RS232_COLOR,
        || FlashingOption::RS232_35T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        translate(TextKey::Rs232_75T_Label, lang),
        translate(TextKey::Rs232_75T_Desc, lang),
        RS232_COLOR,
        || FlashingOption::RS232_75T,
        on_select,
    );

    ui.add_space(BUTTON_SPACE);

    render_colored_option_button(
        ui,
        translate(TextKey::Rs232_100T_Label, lang),
        translate(TextKey::Rs232_100T_Desc, lang),
        RS232_COLOR,
        || FlashingOption::RS232_100T,
        on_select,
    );
}
