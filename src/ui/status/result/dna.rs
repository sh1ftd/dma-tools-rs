use super::components::{
    DNA_VALUE_FONT_SIZE, SPACING_LARGE, SPACING_MEDIUM, SPACING_SMALL, SPACING_XLARGE,
    SUBTITLE_FONT_SIZE, SUCCESS_COLOR, TITLE_FONT_SIZE, render_error, render_framed_content,
    render_icon,
};
use crate::device_programmer::{CompletionStatus, DnaInfo, OperationSnapshot};
use crate::ui::common::palette;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{RichText, Ui};

pub(super) fn render(
    ui: &mut Ui,
    snapshot: &OperationSnapshot,
    lang: &crate::utils::localization::Language,
) {
    match &snapshot.status {
        CompletionStatus::DnaReadCompleted(dna_info) => render_success(ui, dna_info, lang),
        CompletionStatus::Completed => render_error(
            ui,
            translate(TextKey::DnaReadUnexpected, lang),
            translate(TextKey::DnaReadUnexpectedMsg, lang),
            lang,
        ),
        CompletionStatus::Failed(error) => render_error(
            ui,
            translate(TextKey::DnaReadFailed, lang),
            &format!(
                "{}\n\n{error}",
                translate(TextKey::DnaReadFailedPrefix, lang)
            ),
            lang,
        ),
        CompletionStatus::InProgress(status_message) => {
            ui.vertical_centered(|ui| {
                ui.label(format!(
                    "{} {status_message}",
                    translate(TextKey::OperationInProgress, lang)
                ));
                ui.spinner();
            });
        }
        CompletionStatus::NotCompleted => {
            ui.label(translate(TextKey::DnaStatusUnknownMsg, lang));
        }
    }
}

fn render_success(ui: &mut Ui, dna_info: &DnaInfo, lang: &crate::utils::localization::Language) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);
        render_icon(ui, egui_phosphor::regular::CHECK_CIRCLE, SUCCESS_COLOR);
        ui.add_space(SPACING_MEDIUM);
        ui.colored_label(
            SUCCESS_COLOR,
            RichText::new(translate(TextKey::DnaReadSuccess, lang))
                .size(TITLE_FONT_SIZE)
                .strong(),
        );
        ui.add_space(SPACING_XLARGE);

        render_framed_content(ui, SUCCESS_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(translate(TextKey::DeviceDnaHeader, lang))
                        .size(SUBTITLE_FONT_SIZE),
                );
                ui.add_space(SPACING_MEDIUM);

                let response = ui.selectable_label(
                    false,
                    RichText::new(&dna_info.dna_value)
                        .monospace()
                        .strong()
                        .size(DNA_VALUE_FONT_SIZE),
                );

                if response.clicked() {
                    ui.ctx().copy_text(format!(
                        "DNA RAW: {}\nDNA HEX: {}",
                        dna_info.dna_raw_value, dna_info.dna_value
                    ));
                }

                response.on_hover_text(translate(TextKey::ClickToCopyTooltip, lang));
                ui.add_space(SPACING_SMALL);
                ui.label(
                    RichText::new(translate(TextKey::ClickToCopy, lang))
                        .size(14.0)
                        .color(palette::TEXT_MUTED),
                );
            });
        });
    });
}
