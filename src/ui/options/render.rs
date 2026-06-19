use super::panels::{render_dna_section, render_flash_section};
use crate::device_programmer::FlashingOption;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, RichText, Ui, Vec2};

const MAIN_MENU_BUTTON_WIDTH: f32 = 200.0;
const MAIN_MENU_BUTTON_HEIGHT: f32 = 30.0;
const MAIN_MENU_TEXT_SIZE: f32 = 16.0;

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
    on_back: &mut dyn FnMut(),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SelectDnaReadOption, lang));
        ui.add_space(12.0);
        render_dna_section(ui, on_select, lang);
        ui.add_space(16.0);
        render_main_menu_button(ui, on_back, lang);
    });
}

fn render_main_menu_button(ui: &mut Ui, on_back: &mut dyn FnMut(), lang: &crate::app::Language) {
    if ui
        .add(
            egui::Button::new(
                RichText::new(translate(TextKey::MainMenu, lang)).size(MAIN_MENU_TEXT_SIZE),
            )
            .min_size(Vec2::new(MAIN_MENU_BUTTON_WIDTH, MAIN_MENU_BUTTON_HEIGHT)),
        )
        .clicked()
    {
        on_back();
    }
}
