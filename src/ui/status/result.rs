use super::types::ResultAction;
use crate::assets::IconManager;
use crate::device_programmer::{CompletionStatus, DnaInfo, FlashingManager};
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

const FRAME_ROUNDING: f32 = 12.0;
const FRAME_STROKE_WIDTH: f32 = 1.0;
const FRAME_MARGIN: f32 = 20.0;
const FRAME_OUTER_MARGIN: f32 = 10.0;

// Color Constants
const SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);

pub fn render_result_screen(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
) {
    let is_dna_read = manager
        .get_current_option()
        .is_some_and(|option| option.is_dna_read());

    if is_dna_read {
        render_dna_result(ui, manager, on_action, icon_manager);
    } else {
        render_flashing_result(ui, manager, on_action, icon_manager);
    }
}

fn render_dna_result(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
) {
    match manager.get_status() {
        CompletionStatus::DnaReadCompleted(dna_info) => {
            render_dna_success(ui, &dna_info, icon_manager);
        }
        CompletionStatus::Completed => {
            render_error(
                ui,
                "DNA READ STATUS UNEXPECTED",
                "The operation completed, but the DNA value could not be confirmed.\n\
                 This might indicate an issue with the DNA extraction process.\n\
                 Please check the log output for details.",
                icon_manager,
            );
        }
        CompletionStatus::Failed(error) => {
            render_error(
                ui,
                "DNA READ FAILED",
                &format!("Failed to read DNA from the device:\n\n{}", error),
                icon_manager,
            );
        }
        CompletionStatus::InProgress(status_msg) => {
            ui.vertical_centered(|ui| {
                ui.label(format!("Operation in progress: {}", status_msg));
                ui.spinner();
            });
        }
        CompletionStatus::NotCompleted => {
            ui.label("DNA read operation status is unknown.");
            ui.label("Please check the log for details.");
        }
    }

    render_dna_action_buttons(ui, on_action);
}

fn render_dna_success(ui: &mut Ui, dna_info: &DnaInfo, icon_manager: &IconManager) {
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
            RichText::new("DNA READ SUCCESSFUL!")
                .size(TITLE_FONT_SIZE)
                .strong(),
        );

        ui.add_space(SPACING_XLARGE);

        render_framed_content(ui, SUCCESS_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Device DNA").size(SUBTITLE_FONT_SIZE));
                ui.add_space(SPACING_MEDIUM);

                // Display the HEX value
                let response = ui.add(egui::SelectableLabel::new(
                    false,
                    RichText::new(&dna_info.dna_value)
                        .monospace()
                        .strong()
                        .size(DNA_VALUE_FONT_SIZE),
                ));

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
                    ui.output_mut(|output| output.copied_text = copy_text);
                }

                if response.hovered() {
                    egui::show_tooltip(ui.ctx(), response.id, |ui| {
                        ui.label("Click to copy RAW, HEX, and Verilog DNA values");
                    });
                }

                ui.add_space(SPACING_SMALL);
                ui.label("Click to copy");
            });
        });
    });
}

fn render_flashing_result(
    ui: &mut Ui,
    manager: &FlashingManager,
    on_action: &mut dyn FnMut(ResultAction),
    icon_manager: &IconManager,
) {
    let status = manager.get_status();
    let duration = manager.get_duration().unwrap_or(Duration::from_secs(0));
    let duration_secs = duration.as_secs();

    // Analyze sector write times from logs
    let mut total_sectors = 0;
    let mut quick_writes = 0;

    for entry in manager.logger().get_entries() {
        if let Some(time_ms) = extract_sector_time(&entry.message) {
            total_sectors += 1;
            if time_ms < 50 {
                quick_writes += 1;
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
    let has_sectors = total_sectors > 0;
    let has_quick_writes = has_sectors && quick_writes > total_sectors / 2;
    let has_proper_sector_times = has_sectors && quick_writes < total_sectors / 2;

    // Show appropriate screen based on analysis
    match status {
        CompletionStatus::Completed => {
            if has_quick_writes {
                render_error(
                    ui,
                    "FLASHING FAILED - CONNECTION ISSUE",
                    &format!(
                        "Multiple sector writes completed too quickly (50ms or less): {} out of {} sectors.\n\n\
                        This indicates a hardware connection issue. The device is accessible but \
                        data is not being properly transferred.\n\n\
                        Try:\n\
                        1. Use a different USB port\n\
                        2. Check cable connections\n\
                        3. Ensure the device is powered correctly\n\
                        4. Try a different USB cable",
                        quick_writes, total_sectors
                    ),
                    icon_manager,
                );
            } else if has_proper_sector_times || operation_success {
                render_success(ui, icon_manager);

                if duration_secs > 1 {
                    render_duration(ui, duration_secs);
                }
            } else if total_sectors == 0 {
                render_error(
                    ui,
                    "FLASHING RESULT UNKNOWN",
                    "Flashing process completed but no sector write information was found in logs.\n\n\
                    1. You selected the correct board type\n\
                    2. The appropriate USB driver is installed and in JTAG port.\n\
                    3. Try a different USB cable and/or port\n\
                    4. Make sure the device is properly seated in the PCIE slot.",
                    icon_manager,
                );
            } else {
                render_success(ui, icon_manager);

                ui.add_space(SPACING_SMALL);
                ui.label(
                    RichText::new(
                        "Note: Unable to verify complete success, but no errors were detected. Please verify manually or try again.",
                    )
                    .italics(),
                );

                if duration_secs > 1 {
                    render_duration(ui, duration_secs);
                }
            }
        }
        CompletionStatus::DnaReadCompleted(_) => {
            render_error(
                ui,
                "UNEXPECTED STATE",
                "This state should not be reached. Please report this bug.",
                icon_manager,
            );
        }
        CompletionStatus::Failed(error) => {
            render_error(
                ui,
                "FLASHING FAILED",
                &format!("Failed to flash firmware to the device:\n\n{}", error),
                icon_manager,
            );
        }
        CompletionStatus::InProgress(status_msg) => {
            // Display the current operation progress
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("Operation in progress: {}", status_msg))
                        .size(18.0)
                        .color(egui::Color32::from_rgb(50, 150, 255)),
                ); // Use a blue color for progress

                ui.add_space(10.0);
                ui.spinner(); // Show a spinner
            });
        }
        CompletionStatus::NotCompleted => {
            ui.label("Flash operation status is unknown.");
            ui.label("Please check the log for details or try again.");
        }
    }

    render_action_buttons(ui, on_action);
}

fn extract_sector_time(message: &str) -> Option<u64> {
    // Look for lines like "[ERROR] Info : sector 25 took 1 ms"
    if !message.contains("Info :") || !message.contains("sector") || !message.contains("took") {
        //  Filter out non-relevant messages
        return None; // Early exit if the message does not contain the required parts
    }

    message
        .split("took")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn render_duration(ui: &mut Ui, duration_secs: u64) {
    ui.add_space(SPACING_SMALL);
    ui.label(RichText::new(format!(
        "Operation took: {}:{:02}",
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
    egui::Frame::none()
        .fill(ui.style().visuals.extreme_bg_color)
        .rounding(egui::Rounding::same(FRAME_ROUNDING))
        .stroke(egui::Stroke::new(FRAME_STROKE_WIDTH, border_color))
        .inner_margin(egui::Margin::same(FRAME_MARGIN))
        .outer_margin(egui::Margin::same(FRAME_OUTER_MARGIN))
        .show(ui, add_contents);
}

fn render_error(ui: &mut Ui, title: &str, message: &str, icon_manager: &IconManager) {
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
                ui.label(RichText::new("Error Details").size(SUBTITLE_FONT_SIZE));
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

fn render_success(ui: &mut Ui, icon_manager: &IconManager) {
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
            RichText::new("FLASHING SUCCESSFUL!")
                .size(TITLE_FONT_SIZE)
                .strong(),
        );

        ui.add_space(SPACING_XXLARGE);

        // Success message in a bordered frame
        render_framed_content(ui, SUCCESS_COLOR, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Next Steps").size(SUBTITLE_FONT_SIZE));
                ui.add_space(SPACING_MEDIUM);
                ui.label("1. Reboot both computers");
                ui.label("2. Follow the next steps in the guide");
                ui.label("   - Install firmware driver on host computer");
                ui.label("   - Swap cable to DATA port");
                ui.label("   - Activate using provided software and activation code");
                ui.label("   - DNA locked firmware builds do not require activation");
            });
        });

        ui.add_space(SPACING_XLARGE);
    });
}

fn render_dna_action_buttons(ui: &mut Ui, on_action: &mut dyn FnMut(ResultAction)) {
    render_action_buttons_with_layout(ui, on_action, true);
}

fn render_action_buttons(ui: &mut Ui, on_action: &mut dyn FnMut(ResultAction)) {
    render_action_buttons_with_layout(ui, on_action, true);
}

fn render_action_buttons_with_layout(
    ui: &mut Ui,
    on_action: &mut dyn FnMut(ResultAction),
    include_main_menu: bool,
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
            .add(egui::Button::new("Exit").min_size(egui::vec2(button_width, BUTTON_HEIGHT)))
            .clicked()
        {
            on_action(ResultAction::Exit);
        }

        ui.add_space(SPACING_MEDIUM);

        if include_main_menu {
            if ui
                .add(
                    egui::Button::new("Main Menu")
                        .min_size(egui::vec2(button_width, BUTTON_HEIGHT)),
                )
                .clicked()
            {
                on_action(ResultAction::MainMenu);
            }
            ui.add_space(SPACING_MEDIUM);
        }

        if ui
            .add(egui::Button::new("Try Again").min_size(egui::vec2(button_width, BUTTON_HEIGHT)))
            .clicked()
        {
            on_action(ResultAction::TryAgain);
        }
    });

    if !include_main_menu {
        ui.add_space(SPACING_MEDIUM);
    }
}
