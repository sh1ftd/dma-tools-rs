use crate::device_programmer::{
    CompletionStatus, FlashingOption, OperationSnapshot, OperationStage,
};
use crate::ui::common::palette;
use crate::utils::localization::{TextKey, format_translation, translate};
use eframe::egui::{self, RichText, Ui};

// UI configuration constants
const SPINNER_SIZE: f32 = 48.0;
const HEADING_SIZE: f32 = 20.0;
const TECHNICAL_INFO_SIZE: f32 = 16.0;
const STANDARD_SPACING: f32 = 8.0;
const MEDIUM_SPACING: f32 = 12.0;
const LARGE_SPACING: f32 = 20.0;
const EXTRA_LARGE_SPACING: f32 = 25.0;

pub fn render_flashing_progress(
    ui: &mut Ui,
    snapshot: &OperationSnapshot,
    lang: &crate::utils::localization::Language,
) {
    if let Some(option) = snapshot.option.as_ref() {
        let is_dna_read = option.is_dna_read();
        let operation_name = if is_dna_read { "DNA Read" } else { "Flashing" };

        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "{} - {}",
                if is_dna_read {
                    translate(TextKey::ReadingDeviceDna, lang)
                } else {
                    translate(TextKey::FlashingFirmware, lang)
                },
                option.get_display_name()
            ));

            ui.add_space(MEDIUM_SPACING);

            let status_text = get_user_friendly_status(snapshot, lang);

            ui.label(
                RichText::new(status_text)
                    .size(22.0)
                    .strong()
                    .color(ui.visuals().strong_text_color()),
            );

            ui.add_space(LARGE_SPACING);
            ui.add(egui::Spinner::new().size(SPINNER_SIZE));
            ui.add_space(EXTRA_LARGE_SPACING);

            render_operation_info_frame(ui, is_dna_read, lang);
            ui.add_space(EXTRA_LARGE_SPACING);
            render_technical_info_frame(ui, option, operation_name, lang);
        });
    } else {
        ui.heading("Operation");
        ui.label(translate(TextKey::Initializing, lang));
    }
}

fn render_operation_info_frame(
    ui: &mut Ui,
    is_dna_read: bool,
    lang: &crate::utils::localization::Language,
) {
    egui::Frame::NONE
        .fill(palette::SURFACE_RECESSED)
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0_f32, palette::STROKE_SUBTLE))
        .inner_margin(egui::Margin::same(LARGE_SPACING as i8))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                if is_dna_read {
                    render_dna_read_info(ui, lang);
                } else {
                    render_flashing_info(ui, lang);
                }
            });
        });
}

fn render_dna_read_info(ui: &mut Ui, lang: &crate::utils::localization::Language) {
    ui.add(egui::Label::new(
        RichText::new(translate(TextKey::ReadingDeviceDna, lang))
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label(
        RichText::new(translate(TextKey::PleaseWaitDna, lang))
            .size(15.0)
            .color(palette::TEXT_MUTED),
    );
    ui.add_space(STANDARD_SPACING);
    ui.label(
        RichText::new(translate(TextKey::DnaTakesSeconds, lang))
            .size(15.0)
            .color(palette::TEXT_MUTED),
    );
}

fn render_flashing_info(ui: &mut Ui, lang: &crate::utils::localization::Language) {
    ui.add(egui::Label::new(
        RichText::new(translate(TextKey::FlashingFirmware, lang))
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label(
        RichText::new(translate(TextKey::PleaseWaitFlash, lang))
            .size(15.0)
            .color(palette::TEXT_MUTED),
    );
    ui.add_space(STANDARD_SPACING);
    ui.label(
        RichText::new(translate(TextKey::FlashTakesMinutes, lang))
            .size(15.0)
            .color(palette::TEXT_MUTED),
    );
    ui.add_space(STANDARD_SPACING);
    ui.label(
        RichText::new(translate(TextKey::FlashFailImmediate, lang))
            .size(15.0)
            .color(palette::TEXT_MUTED),
    );
}

fn render_technical_info_frame(
    ui: &mut Ui,
    option: &FlashingOption,
    _operation_name: &str,
    lang: &crate::utils::localization::Language,
) {
    egui::Frame::NONE
        .fill(palette::SURFACE)
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0_f32, palette::STROKE))
        .inner_margin(egui::Margin::same(15))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    RichText::new(translate(TextKey::TechnicalInfo, lang))
                        .size(TECHNICAL_INFO_SIZE),
                ));
                ui.add_space(STANDARD_SPACING);
                ui.label(format!(
                    "{} {}",
                    translate(TextKey::InterfaceLabel, lang),
                    option.get_driver_type()
                ));

                let op_type_str = if option.is_dna_read() {
                    translate(TextKey::ReadingDeviceDna, lang)
                } else {
                    translate(TextKey::FlashingFirmware, lang)
                };
                ui.label(format!(
                    "{} {}",
                    translate(TextKey::OperationTypeLabel, lang),
                    op_type_str
                ));

                let device_type = get_device_type(option);
                ui.label(format!(
                    "{} {}",
                    translate(TextKey::TargetDeviceLabel, lang),
                    device_type
                ));
            });
        });
}

fn get_device_type(option: &FlashingOption) -> &'static str {
    match option {
        FlashingOption::CH347_35T => "Artix-7 35T (CH347)",
        FlashingOption::CH347_75T => "Artix-7 75T (CH347)",
        FlashingOption::CH347_100T => "Artix-7 100T (CH347)",
        FlashingOption::RS232_35T => "Artix-7 35T (RS232)",
        FlashingOption::RS232_75T => "Artix-7 75T (RS232)",
        FlashingOption::RS232_100T => "Artix-7 100T (RS232)",
        FlashingOption::DnaCH347 => "CH347",
        FlashingOption::DnaRS232_35T => "Artix-7 35T (RS232)",
        FlashingOption::DnaRS232_75T => "Artix-7 75T (RS232)",
        FlashingOption::DnaRS232_100T => "Artix-7 100T (RS232)",
    }
}

// Add this function to extract a user-friendly status
fn get_user_friendly_status(
    snapshot: &OperationSnapshot,
    lang: &crate::utils::localization::Language,
) -> String {
    if snapshot
        .option
        .as_ref()
        .is_some_and(|opt| opt.is_dna_read())
    {
        dna_status_text(&snapshot.status, lang)
    } else {
        match snapshot.stage {
            OperationStage::Starting => translate(TextKey::StartingOperation, lang).to_string(),
            OperationStage::InitializingJtag => translate(TextKey::InitJtag, lang).to_string(),
            OperationStage::LoadingBitstream => {
                translate(TextKey::LoadingBitstream, lang).to_string()
            }
            OperationStage::ResettingFpga => translate(TextKey::ResettingFpga, lang).to_string(),
            OperationStage::ProbingFlash => translate(TextKey::ProbingFlash, lang).to_string(),
            OperationStage::WritingImage => translate(TextKey::WritingImage, lang).to_string(),
            OperationStage::WritingSector(sector) => {
                format!("{} {}...", translate(TextKey::WritingSector, lang), sector)
            }
            OperationStage::Verifying => translate(TextKey::Verifying, lang).to_string(),
        }
    }
}

fn dna_status_text(
    status: &CompletionStatus,
    lang: &crate::utils::localization::Language,
) -> String {
    match status {
        CompletionStatus::NotCompleted => translate(TextKey::DnaWaitingStart, lang).to_string(),
        CompletionStatus::InProgress(_) => translate(TextKey::DnaRetrieving, lang).to_string(),
        CompletionStatus::DnaReadCompleted(_) => {
            translate(TextKey::DnaReadSuccessStatus, lang).to_string()
        }
        CompletionStatus::Completed => translate(TextKey::DnaOperationCompleted, lang).to_string(),
        CompletionStatus::Failed(error) => {
            format_translation(translate(TextKey::DnaReadFailedStatus, lang), &[error])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_programmer::DnaInfo;
    use crate::utils::localization::Language;

    #[test]
    fn dna_stage_messages_are_non_empty() {
        let statuses = [
            CompletionStatus::NotCompleted,
            CompletionStatus::InProgress("working".into()),
            CompletionStatus::Completed,
            CompletionStatus::Failed("err".into()),
            CompletionStatus::DnaReadCompleted(DnaInfo {
                dna_value: "0x1".into(),
                dna_raw_value: "1".into(),
                device_type: "T".into(),
            }),
        ];

        for status in &statuses {
            assert!(!dna_status_text(status, &Language::English).is_empty());
        }
    }

    #[test]
    fn failed_dna_stage_contains_error() {
        let message = dna_status_text(&CompletionStatus::Failed("oops".into()), &Language::English);
        assert!(message.contains("oops"));
    }

    #[test]
    fn failed_arabic_dna_stage_contains_error_without_placeholders() {
        let message = dna_status_text(
            &CompletionStatus::Failed("access denied".into()),
            &Language::Arabic,
        );

        assert!(message.contains("access denied"));
        assert!(!message.contains("{}"));
        assert!(!message.contains("}{"));
    }
}
