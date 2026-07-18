use super::components::{
    SPACING_SMALL, render_duration_if_meaningful, render_error, render_success,
};
use crate::device_programmer::{CompletionStatus, FlashAssessment, OperationSnapshot};
use crate::ui::common::palette;
use crate::utils::localization::{TextKey, format_translation, translate};
use eframe::egui::{RichText, Ui};

pub(super) fn render(
    ui: &mut Ui,
    snapshot: &OperationSnapshot,
    lang: &crate::utils::localization::Language,
) {
    let duration_secs = snapshot.duration.unwrap_or_default().as_secs();

    match &snapshot.assessment {
        FlashAssessment::ConnectionUnstable {
            normal_writes,
            total_sectors,
        } => {
            let message = connection_error_message(*normal_writes, *total_sectors, lang);
            render_error(
                ui,
                translate(TextKey::FlashingFailedConnection, lang),
                &message,
                lang,
            );
        }
        FlashAssessment::Success => render_success_with_duration(ui, duration_secs, lang),
        FlashAssessment::SuccessWithLimitedSamples { .. } => {
            render_success(ui, lang);
            ui.add_space(SPACING_SMALL);
            ui.label(RichText::new(translate(TextKey::NoteFewerSectors, lang)).italics());
            render_duration_if_meaningful(ui, duration_secs, lang);
        }
        FlashAssessment::Indeterminate => render_error(
            ui,
            translate(TextKey::FlashingResultUnknown, lang),
            translate(TextKey::FlashingResultUnknownMsg, lang),
            lang,
        ),
        FlashAssessment::UnexpectedDnaResult | FlashAssessment::NotApplicable => render_error(
            ui,
            "UNEXPECTED STATE",
            translate(TextKey::UnexpectedStateMsg, lang),
            lang,
        ),
        FlashAssessment::Failed(error) => render_error(
            ui,
            translate(TextKey::FlashingFailed, lang),
            &format!(
                "{}\n\n{error}",
                translate(TextKey::FlashingFailedPrefix, lang)
            ),
            lang,
        ),
        FlashAssessment::Pending => render_pending(ui, &snapshot.status, lang),
    }
}

fn connection_error_message(
    normal_writes: usize,
    total_sectors: usize,
    lang: &crate::utils::localization::Language,
) -> String {
    let normal_writes = normal_writes.to_string();
    let total_sectors = total_sectors.to_string();
    format_translation(
        translate(TextKey::FlashingFailedConnectionMsg, lang),
        &[&normal_writes, &total_sectors],
    )
}

fn render_success_with_duration(
    ui: &mut Ui,
    duration_secs: u64,
    lang: &crate::utils::localization::Language,
) {
    render_success(ui, lang);
    render_duration_if_meaningful(ui, duration_secs, lang);
}

fn render_pending(
    ui: &mut Ui,
    status: &CompletionStatus,
    lang: &crate::utils::localization::Language,
) {
    let CompletionStatus::InProgress(status_message) = status else {
        ui.label(translate(TextKey::FlashStatusUnknownMsg, lang));
        return;
    };

    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(format!(
                "{} {status_message}",
                translate(TextKey::OperationInProgress, lang)
            ))
            .size(18.0)
            .color(palette::INFO),
        );
        ui.add_space(10.0);
        ui.spinner();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::localization::Language;

    #[test]
    fn arabic_connection_error_contains_stats_without_placeholders() {
        let message = connection_error_message(4, 10, &Language::Arabic);

        assert!(message.contains('4'));
        assert!(message.contains("10"));
        assert!(!message.contains("{}"));
        assert!(!message.contains("}{"));
    }
}
