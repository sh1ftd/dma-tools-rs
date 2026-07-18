use super::super::types::ResultAction;
use crate::ui::common::{self, palette};
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, RichText, Ui};

pub(super) const SPACING_SMALL: f32 = 6.0;
pub(super) const SPACING_MEDIUM: f32 = 12.0;
pub(super) const SPACING_LARGE: f32 = 18.0;
pub(super) const SPACING_XLARGE: f32 = 24.0;
const SPACING_XXLARGE: f32 = 30.0;

const ICON_SIZE: f32 = 60.0;
const BUTTON_HEIGHT: f32 = 32.0;
pub(super) const TITLE_FONT_SIZE: f32 = 24.0;
pub(super) const SUBTITLE_FONT_SIZE: f32 = 16.0;
pub(super) const DNA_VALUE_FONT_SIZE: f32 = 22.0;

const FRAME_ROUNDING: u8 = 12;
const FRAME_STROKE_WIDTH: f32 = 1.0;
const FRAME_MARGIN: i8 = 20;
const FRAME_OUTER_MARGIN: i8 = 10;

pub(super) const SUCCESS_COLOR: egui::Color32 = palette::SUCCESS;
const ERROR_COLOR: egui::Color32 = palette::ERROR;

pub(super) fn render_duration_if_meaningful(
    ui: &mut Ui,
    duration_secs: u64,
    lang: &crate::utils::localization::Language,
) {
    if duration_secs <= 1 {
        return;
    }

    ui.add_space(SPACING_SMALL);
    ui.label(
        RichText::new(format!(
            "{}: {}:{:02}",
            translate(TextKey::OperationTook, lang),
            duration_secs / 60,
            duration_secs % 60
        ))
        .size(14.0)
        .color(palette::TEXT_MUTED),
    );
}

pub(super) fn render_icon(ui: &mut Ui, icon: &str, color: egui::Color32) {
    ui.add(egui::Label::new(
        RichText::new(icon).size(ICON_SIZE).color(color),
    ));
}

pub(super) fn render_framed_content(
    ui: &mut Ui,
    border_color: egui::Color32,
    add_contents: impl FnOnce(&mut Ui),
) {
    egui::Frame::NONE
        .fill(palette::SURFACE_RECESSED)
        .corner_radius(egui::CornerRadius::same(FRAME_ROUNDING))
        .stroke(egui::Stroke::new(FRAME_STROKE_WIDTH, border_color))
        .inner_margin(egui::Margin::same(FRAME_MARGIN))
        .outer_margin(egui::Margin::same(FRAME_OUTER_MARGIN))
        .show(ui, add_contents);
}

pub(super) fn render_error(
    ui: &mut Ui,
    title: &str,
    message: &str,
    lang: &crate::utils::localization::Language,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);
        render_icon(ui, egui_phosphor::regular::X_CIRCLE, ERROR_COLOR);
        ui.add_space(SPACING_MEDIUM);
        ui.colored_label(
            ERROR_COLOR,
            RichText::new(title).size(TITLE_FONT_SIZE).strong(),
        );
        ui.add_space(SPACING_XXLARGE);

        render_framed_content(ui, ERROR_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(translate(TextKey::ErrorDetails, lang)).size(SUBTITLE_FONT_SIZE),
                );
                ui.add_space(SPACING_MEDIUM);
                for line in message.split('\n') {
                    if line.trim().is_empty() {
                        ui.add_space(SPACING_SMALL);
                    } else {
                        ui.label(line.trim());
                    }
                }
            });
        });
    });
}

pub(super) fn render_success(ui: &mut Ui, lang: &crate::utils::localization::Language) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);
        render_icon(ui, egui_phosphor::regular::CHECK_CIRCLE, SUCCESS_COLOR);
        ui.add_space(SPACING_MEDIUM);
        ui.colored_label(
            SUCCESS_COLOR,
            RichText::new(translate(TextKey::FlashingSuccess, lang))
                .size(TITLE_FONT_SIZE)
                .strong(),
        );
        ui.add_space(SPACING_XXLARGE);

        render_framed_content(ui, SUCCESS_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(translate(TextKey::NextSteps, lang)).size(SUBTITLE_FONT_SIZE),
                );
                ui.add_space(SPACING_MEDIUM);
                for line in translate(TextKey::NextStepsList, lang).split('\n') {
                    ui.label(line);
                }
            });
        });

        ui.add_space(SPACING_XLARGE);
    });
}

pub(super) fn render_action_buttons(
    ui: &mut Ui,
    on_action: &mut dyn FnMut(ResultAction),
    lang: &crate::utils::localization::Language,
    safe_to_restart: bool,
) {
    ui.add_space(SPACING_MEDIUM);
    ui.separator();
    ui.add_space(SPACING_MEDIUM);

    ui.add_enabled_ui(safe_to_restart, |ui| {
        ui.horizontal(|ui| {
            let spacing = SPACING_MEDIUM;
            let button_width = (ui.available_width() - spacing) / 2.0;

            if common::secondary_icon_button(
                ui,
                Some(egui_phosphor::regular::HOUSE),
                translate(TextKey::MainMenu, lang),
                egui::vec2(button_width, BUTTON_HEIGHT),
            )
            .clicked()
            {
                on_action(ResultAction::MainMenu);
            }
            ui.add_space(spacing);

            if common::primary_icon_button(
                ui,
                Some(egui_phosphor::regular::ARROWS_CLOCKWISE),
                translate(TextKey::TryAgainButton, lang),
                egui::vec2(button_width, BUTTON_HEIGHT),
            )
            .clicked()
            {
                on_action(ResultAction::TryAgain);
            }
        });
    });
}
