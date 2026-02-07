use super::types::ResultAction;
use crate::assets::IconManager;
use crate::device_programmer::{CompletionStatus, DnaInfo, FlashingManager};
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, RichText, Ui};
use std::time::Duration;

// UI Constants
const SPACING_SMALL: f32 = 6.0;
const SPACING_MEDIUM: f32 = 12.0;
const SPACING_LARGE: f32 = 18.0;
const SPACING_XLARGE: f32 = 24.0;
const SPACING_XXLARGE: f32 = 30.0;

const ICON_SIZE: f32 = 60.0;
const BUTTON_HEIGHT: f32 = 32.0;
const TITLE_FONT_SIZE: f32 = 24.0;
const SUBTITLE_FONT_SIZE: f32 = 16.0;
const DNA_VALUE_FONT_SIZE: f32 = 22.0;
const FALLBACK_ICON_SIZE: f32 = 50.0;

const FRAME_ROUNDING: u8 = 12;
const FRAME_STROKE_WIDTH: f32 = 1.0;
const FRAME_MARGIN: i8 = 20;
const FRAME_OUTER_MARGIN: i8 = 10;

// Color Constants
const SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);

pub fn render_result_screen(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
    lang: &crate::app::Language,
) {
    let is_dna_read = manager
        .get_current_option()
        .is_some_and(|option| option.is_dna_read());

    if is_dna_read {
        render_dna_result(ui, manager, on_action, icon_manager, lang);
    } else {
        render_flashing_result(ui, manager, on_action, icon_manager, lang);
    }
}

fn render_dna_result(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
    lang: &crate::app::Language,
) {
    match manager.get_status() {
        CompletionStatus::DnaReadCompleted(dna_info) => {
            render_dna_success(ui, &dna_info, icon_manager, lang);
        }
        CompletionStatus::Completed => {
            render_error(
                ui,
                translate(TextKey::DnaReadUnexpected, lang),
                translate(TextKey::DnaReadUnexpectedMsg, lang),
                icon_manager,
                lang,
            );
        }
        CompletionStatus::Failed(error) => {
            render_error(
                ui,
                translate(TextKey::DnaReadFailed, lang),
                &format!(
                    "{}\n\n{error}",
                    translate(TextKey::DnaReadFailedPrefix, lang)
                ),
                icon_manager,
                lang,
            );
        }
        CompletionStatus::InProgress(status_msg) => {
            ui.vertical_centered(|ui| {
                ui.label(format!(
                    "{} {status_msg}",
                    translate(TextKey::OperationInProgress, lang)
                ));
                ui.spinner();
            });
        }
        CompletionStatus::NotCompleted => {
            ui.label(translate(TextKey::DnaStatusUnknownMsg, lang));
        }
    }

    render_dna_action_buttons(ui, on_action, lang);
}

fn render_dna_success(
    ui: &mut Ui,
    dna_info: &DnaInfo,
    icon_manager: &IconManager,
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);

        render_icon(
            ui,
            icon_manager.checkmark_icon().cloned(),
            '✓',
            SUCCESS_COLOR,
        );

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

                let dna_text = RichText::new(&dna_info.dna_value)
                    .monospace()
                    .strong()
                    .size(DNA_VALUE_FONT_SIZE);

                let response = ui.selectable_label(false, dna_text);

                if response.clicked() {
                    // Get the Verilog format
                    let verilog_hex =
                        crate::device_programmer::dna::DnaReader::convert_dna_to_verilog_hex(
                            &dna_info.dna_raw_value,
                        );

                    // Create a multi-line string with all formats
                    let copy_text = format!(
                        "DNA RAW: {}\nDNA HEX: {}\nVERILOG: {}",
                        dna_info.dna_raw_value, dna_info.dna_value, verilog_hex
                    );
                    ui.ctx().copy_text(copy_text);
                }

                response.on_hover_text(translate(TextKey::ClickToCopyTooltip, lang));

                ui.add_space(SPACING_SMALL);
                ui.label(translate(TextKey::ClickToCopy, lang));
            });
        });
    });
}

fn render_flashing_result(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
    lang: &crate::app::Language,
) {
    let status = manager.get_status();
    let duration = manager.get_duration().unwrap_or(Duration::from_secs(0));
    let duration_secs = duration.as_secs();

    // Analyze sector write times from logs
    let mut total_sectors = 0;
    let mut normal_writes = 0;

    for entry in manager.logger().get_entries() {
        if let Some(time_ms) = extract_sector_time(&entry.message) {
            total_sectors += 1;
            if time_ms >= 10 {
                normal_writes += 1;
            }
        }
    }

    // Check the logs for success message
    let operation_success = manager.logger().get_entries().iter().any(|entry| {
        entry
            .message
            .contains("Firmware flash completed successfully")
    });

    // Analysis of sector write times
    let has_enough_sectors = total_sectors >= 10; // Minimum sectors before checking normal writes
    let has_few_normal_writes = has_enough_sectors && normal_writes < 5;
    let has_proper_sector_times = has_enough_sectors && normal_writes >= 5;

    // Show appropriate screen based on analysis
    match status {
        CompletionStatus::Completed => {
            if has_enough_sectors && has_few_normal_writes {
                let msg = translate(TextKey::FlashingFailedConnectionMsg, lang)
                    .replacen("{}", &normal_writes.to_string(), 1)
                    .replacen("{}", &total_sectors.to_string(), 1);

                render_error(
                    ui,
                    translate(TextKey::FlashingFailedConnection, lang),
                    &msg,
                    icon_manager,
                    lang,
                );
            } else if has_proper_sector_times || operation_success {
                render_success(ui, icon_manager, lang);

                if duration_secs > 1 {
                    render_duration(ui, duration_secs, lang);
                }
            } else if total_sectors == 0 {
                render_error(
                    ui,
                    translate(TextKey::FlashingResultUnknown, lang),
                    translate(TextKey::FlashingResultUnknownMsg, lang),
                    icon_manager,
                    lang,
                );
            } else if total_sectors < 10 {
                render_success(ui, icon_manager, lang);

                ui.add_space(SPACING_SMALL);
                ui.label(RichText::new(translate(TextKey::NoteFewerSectors, lang)).italics());

                if duration_secs > 1 {
                    render_duration(ui, duration_secs, lang);
                }
            } else {
                render_success(ui, icon_manager, lang);

                ui.add_space(SPACING_SMALL);
                ui.label(RichText::new(translate(TextKey::NoteVerifySuccess, lang)).italics());

                if duration_secs > 1 {
                    render_duration(ui, duration_secs, lang);
                }
            }
        }
        CompletionStatus::DnaReadCompleted(_) => {
            render_error(
                ui,
                "UNEXPECTED STATE",
                translate(TextKey::UnexpectedStateMsg, lang),
                icon_manager,
                lang,
            );
        }
        CompletionStatus::Failed(error) => {
            render_error(
                ui,
                translate(TextKey::FlashingFailed, lang),
                &format!(
                    "{}\n\n{error}",
                    translate(TextKey::FlashingFailedPrefix, lang)
                ),
                icon_manager,
                lang,
            );
        }
        CompletionStatus::InProgress(status_msg) => {
            // Display the current operation progress
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {status_msg}",
                        translate(TextKey::OperationInProgress, lang)
                    ))
                    .size(18.0)
                    .color(egui::Color32::from_rgb(50, 150, 255)),
                ); // Use a blue color for progress

                ui.add_space(10.0);
                ui.spinner(); // Show a spinner
            });
        }
        CompletionStatus::NotCompleted => {
            ui.label(translate(TextKey::FlashStatusUnknownMsg, lang));
        }
    }

    render_action_buttons(ui, on_action, lang);
}

fn extract_sector_time(message: &str) -> Option<u64> {
    if !message.contains("Info :") || !message.contains("sector") || !message.contains("took") {
        return None;
    }

    message
        .split("took")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn render_duration(ui: &mut Ui, duration_secs: u64, lang: &crate::app::Language) {
    ui.add_space(SPACING_SMALL);
    ui.label(RichText::new(format!(
        "{}: {}:{:02}",
        translate(TextKey::OperationTook, lang),
        duration_secs / 60,
        duration_secs % 60
    )));
}

fn render_icon(
    ui: &mut Ui,
    icon_option: Option<egui::TextureHandle>,
    fallback_char: char,
    color: egui::Color32,
) {
    if let Some(icon) = icon_option {
        let icon_size = egui::vec2(ICON_SIZE, ICON_SIZE);
        ui.add(
            egui::Image::new(&icon)
                .fit_to_exact_size(icon_size)
                .tint(color),
        );
    } else {
        ui.add(egui::Label::new(
            RichText::new(fallback_char.to_string())
                .size(FALLBACK_ICON_SIZE)
                .color(color),
        ));
    }
}

fn render_framed_content(
    ui: &mut Ui,
    border_color: egui::Color32,
    add_contents: impl FnOnce(&mut Ui),
) {
    egui::Frame::NONE
        .fill(ui.style().visuals.extreme_bg_color)
        .corner_radius(egui::CornerRadius::same(FRAME_ROUNDING))
        .stroke(egui::Stroke::new(FRAME_STROKE_WIDTH, border_color))
        .inner_margin(egui::Margin::same(FRAME_MARGIN))
        .outer_margin(egui::Margin::same(FRAME_OUTER_MARGIN))
        .show(ui, add_contents);
}

fn render_error(
    ui: &mut Ui,
    title: &str,
    message: &str,
    icon_manager: &IconManager,
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);

        // Add error icon
        render_icon(ui, icon_manager.x_icon().cloned(), 'X', ERROR_COLOR);

        ui.add_space(SPACING_MEDIUM);

        // Error title in red
        ui.colored_label(
            ERROR_COLOR,
            RichText::new(title).size(TITLE_FONT_SIZE).strong(),
        );

        ui.add_space(SPACING_XXLARGE);

        // Error message in a bordered frame
        render_framed_content(ui, ERROR_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(translate(TextKey::ErrorDetails, lang)).size(SUBTITLE_FONT_SIZE),
                );
                ui.add_space(SPACING_MEDIUM);

                // Split message by newlines and display each line
                for line in message.split('\n') {
                    if !line.trim().is_empty() {
                        ui.label(line.trim());
                    } else {
                        ui.add_space(SPACING_SMALL);
                    }
                }
            });
        });
    });
}

fn render_success(ui: &mut Ui, icon_manager: &IconManager, lang: &crate::app::Language) {
    ui.vertical_centered(|ui| {
        ui.add_space(SPACING_LARGE);

        // Add success icon
        render_icon(
            ui,
            icon_manager.checkmark_icon().cloned(),
            '✓',
            SUCCESS_COLOR,
        );

        ui.add_space(SPACING_MEDIUM);

        ui.colored_label(
            SUCCESS_COLOR,
            RichText::new(translate(TextKey::FlashingSuccess, lang))
                .size(TITLE_FONT_SIZE)
                .strong(),
        );

        ui.add_space(SPACING_XXLARGE);

        // Success message in a bordered frame
        render_framed_content(ui, SUCCESS_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(translate(TextKey::NextSteps, lang)).size(SUBTITLE_FONT_SIZE),
                );
                ui.add_space(SPACING_MEDIUM);
                let next_steps = translate(TextKey::NextStepsList, lang);
                // Split by newlines to display each line
                for line in next_steps.split('\n') {
                    ui.label(line);
                }
            });
        });

        ui.add_space(SPACING_XLARGE);
    });
}

fn render_dna_action_buttons(
    ui: &mut Ui,
    on_action: &mut dyn FnMut(ResultAction),
    lang: &crate::app::Language,
) {
    render_action_buttons_with_layout(ui, on_action, true, lang);
}

fn render_action_buttons(
    ui: &mut Ui,
    on_action: &mut dyn FnMut(ResultAction),
    lang: &crate::app::Language,
) {
    render_action_buttons_with_layout(ui, on_action, true, lang);
}

fn render_action_buttons_with_layout(
    ui: &mut Ui,
    on_action: &mut dyn FnMut(ResultAction),
    include_main_menu: bool,
    lang: &crate::app::Language,
) {
    ui.add_space(SPACING_MEDIUM);
    ui.separator();
    ui.add_space(SPACING_MEDIUM);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();

        let button_count = if include_main_menu { 3 } else { 2 };
        let spacing = SPACING_MEDIUM * (button_count - 1) as f32;
        let button_width = (available_width - spacing) / button_count as f32;

        if ui
            .add(
                egui::Button::new(translate(TextKey::Exit, lang))
                    .min_size(egui::vec2(button_width, BUTTON_HEIGHT)),
            )
            .clicked()
        {
            on_action(ResultAction::Exit);
        }

        ui.add_space(SPACING_MEDIUM);

        if include_main_menu {
            if ui
                .add(
                    egui::Button::new(translate(TextKey::MainMenu, lang))
                        .min_size(egui::vec2(button_width, BUTTON_HEIGHT)),
                )
                .clicked()
            {
                on_action(ResultAction::MainMenu);
            }
            ui.add_space(SPACING_MEDIUM);
        }

        if ui
            .add(
                egui::Button::new(translate(TextKey::TryAgainButton, lang))
                    .min_size(egui::vec2(button_width, BUTTON_HEIGHT)),
            )
            .clicked()
        {
            on_action(ResultAction::TryAgain);
        }
    });

    if !include_main_menu {
        ui.add_space(SPACING_MEDIUM);
    }
}
