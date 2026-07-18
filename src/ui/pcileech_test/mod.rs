use crate::pcileech_test::PcileechTestState;
use crate::ui::common::{self, palette};
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Color32, RichText};

const STATUS_CARD_WIDTH: f32 = 440.0;
const STATUS_CARD_RADIUS: u8 = 10;
const STATUS_CARD_FILL: Color32 = palette::SURFACE_RECESSED;
const STATUS_CARD_INNER_MARGIN_X: i8 = 18;
const STATUS_CARD_INNER_MARGIN_Y: i8 = 16;
const STATUS_CARD_STROKE_WIDTH: f32 = 1.0;
const STATUS_ICON_SIZE: f32 = 38.0;
const STATUS_TITLE_SIZE: f32 = 18.0;
const STATUS_TEXT_SIZE: f32 = 14.0;
const SUCCESS_COLOR: Color32 = palette::SUCCESS;
const ERROR_COLOR: Color32 = palette::ERROR;
const BODY_TEXT_COLOR: Color32 = palette::TEXT;
const MUTED_TEXT_COLOR: Color32 = palette::TEXT_MUTED;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcileechAction {
    Back,
    Retry,
}

pub fn render_pcileech_test(
    ui: &mut egui::Ui,
    test_state: &PcileechTestState,
    lang: &crate::utils::localization::Language,
) -> Option<PcileechAction> {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::TestPcileechTitle, lang));
        ui.add_space(30.0);

        match test_state {
            PcileechTestState::Idle | PcileechTestState::Running => {
                render_running_state(ui, lang);
                ui.ctx().request_repaint();
            }
            PcileechTestState::Success(line) => render_success_state(ui, line, lang),
            PcileechTestState::Failed(message) => render_error_state(ui, message, lang),
        }
    });

    ui.add_space(40.0);
    ui.separator();
    ui.add_space(15.0);

    let mut action = None;
    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let spacing = 12.0;
        let button_width = (available_width - spacing) / 2.0;

        if common::secondary_icon_button(
            ui,
            Some(egui_phosphor::regular::HOUSE),
            translate(TextKey::MainMenu, lang),
            egui::vec2(button_width, 32.0),
        )
        .clicked()
        {
            action = Some(PcileechAction::Back);
        }

        ui.add_space(spacing);

        if common::primary_icon_button(
            ui,
            Some(egui_phosphor::regular::ARROWS_CLOCKWISE),
            translate(TextKey::TryAgainButton, lang),
            egui::vec2(button_width, 32.0),
        )
        .clicked()
        {
            action = Some(PcileechAction::Retry);
        }
    });

    ui.add_space(15.0);
    action
}

fn render_running_state(ui: &mut egui::Ui, lang: &crate::utils::localization::Language) {
    ui.add_space(10.0);
    render_status_card(ui, palette::INFO, |ui| {
        ui.vertical_centered(|ui| {
            ui.spinner();
            ui.add_space(12.0);
            ui.label(
                RichText::new(translate(TextKey::TestingConnection, lang))
                    .size(STATUS_TEXT_SIZE + 1.0)
                    .color(BODY_TEXT_COLOR),
            );
        });
    });
}

fn render_success_state(
    ui: &mut egui::Ui,
    line: &str,
    lang: &crate::utils::localization::Language,
) {
    ui.add_space(10.0);
    render_status_card(ui, SUCCESS_COLOR, |ui| {
        ui.vertical_centered(|ui| {
            render_status_icon(ui, egui_phosphor::regular::CHECK_CIRCLE, SUCCESS_COLOR);
            ui.add_space(8.0);
            ui.label(
                RichText::new(translate(TextKey::TestSuccess, lang))
                    .strong()
                    .size(STATUS_TITLE_SIZE)
                    .color(SUCCESS_COLOR),
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new(line)
                    .size(STATUS_TEXT_SIZE)
                    .color(BODY_TEXT_COLOR),
            );
        });
    });
}

fn render_error_state(
    ui: &mut egui::Ui,
    message: &str,
    lang: &crate::utils::localization::Language,
) {
    ui.add_space(10.0);
    render_status_card(ui, ERROR_COLOR, |ui| {
        ui.vertical_centered(|ui| {
            render_status_icon(ui, egui_phosphor::regular::X_CIRCLE, ERROR_COLOR);
            ui.add_space(8.0);
            ui.label(
                RichText::new(translate(TextKey::TestFailed, lang))
                    .strong()
                    .size(STATUS_TITLE_SIZE)
                    .color(ERROR_COLOR),
            );
        });

        ui.add_space(14.0);
        render_error_message(ui, message);
        ui.add_space(14.0);
        render_connection_fixes(ui, lang);
    });
}

fn render_status_icon(ui: &mut egui::Ui, icon: &str, color: Color32) {
    ui.label(RichText::new(icon).color(color).size(STATUS_ICON_SIZE));
}

fn render_status_card(
    ui: &mut egui::Ui,
    border_color: Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let card_width = ui.available_width().min(STATUS_CARD_WIDTH);
    let content_width = (card_width - f32::from(STATUS_CARD_INNER_MARGIN_X) * 2.0).max(0.0);

    egui::Frame::NONE
        .fill(STATUS_CARD_FILL)
        .corner_radius(egui::CornerRadius::same(STATUS_CARD_RADIUS))
        .stroke(egui::Stroke::new(STATUS_CARD_STROKE_WIDTH, border_color))
        .inner_margin(egui::Margin::symmetric(
            STATUS_CARD_INNER_MARGIN_X,
            STATUS_CARD_INNER_MARGIN_Y,
        ))
        .show(ui, |ui| {
            ui.set_min_width(content_width);
            add_contents(ui);
        });
}

fn render_error_message(ui: &mut egui::Ui, message: &str) {
    let message_width = (ui.available_width() - 24.0).max(0.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(17, 18, 21))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(message_width);
            ui.label(
                RichText::new(message)
                    .size(STATUS_TEXT_SIZE)
                    .color(Color32::from_rgb(255, 145, 150)),
            );
        });
}

fn render_connection_fixes(ui: &mut egui::Ui, lang: &crate::utils::localization::Language) {
    ui.vertical_centered(|ui| {
        for (index, line) in translate(TextKey::ConnectionError, lang)
            .lines()
            .enumerate()
        {
            let mut text = RichText::new(line)
                .size(STATUS_TEXT_SIZE)
                .color(MUTED_TEXT_COLOR);

            if index == 0 {
                text = text.strong().color(BODY_TEXT_COLOR);
            }

            ui.label(text);
        }
    });
}
