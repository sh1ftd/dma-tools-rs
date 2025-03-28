use crate::device_programmer::{FlashingManager, FlashingOption};
use eframe::egui::{self, RichText, Ui};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// UI configuration constants
const SECTOR_STUCK_THRESHOLD: Duration = Duration::from_secs(1);
const SPINNER_SIZE: f32 = 48.0;
const HEADING_SIZE: f32 = 20.0;
const TECHNICAL_INFO_SIZE: f32 = 16.0;
const STANDARD_SPACING: f32 = 8.0;
const MEDIUM_SPACING: f32 = 12.0;
const LARGE_SPACING: f32 = 20.0;
const EXTRA_LARGE_SPACING: f32 = 25.0;

// Tracks each sector's first appearance time to detect stalled operations
static SECTOR_TIMESTAMPS: LazyLock<Mutex<HashMap<u32, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn render_flashing_progress(ui: &mut Ui, manager: &FlashingManager) {
    let now = Instant::now();

    let (_stage_message, _current_sector, _is_writing) = determine_current_stage(manager, now);

    if let Some(option) = manager.get_current_option() {
        let is_dna_read = option.is_dna_read();
        let operation_name = if is_dna_read { "DNA Read" } else { "Flashing" };

        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "{} - {}",
                operation_name,
                option.get_display_name()
            ));

            ui.add_space(MEDIUM_SPACING);

            // Display the status more prominently
            let status_text = get_user_friendly_status(manager);

            ui.label(
                RichText::new(status_text)
                    .size(22.0)
                    .strong()
                    .color(ui.visuals().strong_text_color()),
            );

            ui.add_space(LARGE_SPACING);
            ui.add(egui::Spinner::new().size(SPINNER_SIZE));
            ui.add_space(EXTRA_LARGE_SPACING);

            render_operation_info_frame(ui, is_dna_read);
            ui.add_space(EXTRA_LARGE_SPACING);
            render_technical_info_frame(ui, option, operation_name);
        });
    } else {
        ui.heading("Operation");
        ui.label("Initializing...");
    }
}

fn determine_current_stage(manager: &FlashingManager, now: Instant) -> (String, Option<u32>, bool) {
    let logger = manager.logger();
    let entries = logger.get_entries();

    let mut current_stage = "Starting operation...";
    let mut current_sector = None;
    let mut is_writing = false;
    let mut is_finalizing = false;

    // Find the most recent log entry that indicates our current progress
    for entry in entries.iter().rev() {
        let msg = &entry.message;

        if msg.contains("sector") && msg.contains("took") {
            if let Some(sector) = extract_sector_from_log(msg) {
                let mut sector_times = SECTOR_TIMESTAMPS.lock().unwrap();
                sector_times.entry(sector).or_insert(now);

                // Check if we've been stuck on this sector
                if let Some(first_seen) = sector_times.get(&sector) {
                    if now.duration_since(*first_seen) > SECTOR_STUCK_THRESHOLD {
                        is_finalizing = true;
                    }
                }

                current_sector = Some(sector);
                is_writing = true;
                break;
            }
        } else if msg.contains("Writing the image to the flash memory") {
            current_stage = "Writing image to flash memory...";
            is_writing = true;
            break;
        } else if msg.contains("Probing the flash memory") {
            current_stage = "Probing flash memory...";
            break;
        } else if msg.contains("Resetting and halting the FPGA") {
            current_stage = "Resetting and halting FPGA...";
            break;
        } else if msg.contains("Loading the bitstream") {
            current_stage = "Loading bitstream...";
            break;
        } else if msg.contains("Initializing the JTAG interface") {
            current_stage = "Initializing JTAG interface...";
            break;
        }
    }

    // Create the appropriate message based on current state
    let stage_message = if is_writing && current_sector.is_some() {
        if is_finalizing {
            "Testing and verifying...".to_string()
        } else {
            format!("Writing sector {}...", current_sector.unwrap())
        }
    } else {
        current_stage.to_string()
    };

    (stage_message, current_sector, is_writing)
}

fn extract_sector_from_log(message: &str) -> Option<u32> {
    message
        .split("sector")
        .nth(1)
        .and_then(|s| s.split("took").next())
        .and_then(|sector_str| sector_str.trim().parse::<u32>().ok())
}

fn render_operation_info_frame(ui: &mut Ui, is_dna_read: bool) {
    egui::Frame::none()
        .fill(ui.style().visuals.extreme_bg_color)
        .rounding(egui::Rounding::same(12.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
        .inner_margin(egui::Margin::same(LARGE_SPACING))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                if is_dna_read {
                    render_dna_read_info(ui);
                } else {
                    render_flashing_info(ui);
                }
            });
        });
}

fn render_dna_read_info(ui: &mut Ui) {
    ui.add(egui::Label::new(
        RichText::new("Reading Device DNA")
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label("Please wait while we retrieve the unique ID from your device.");
    ui.add_space(STANDARD_SPACING);
    ui.label("This typically takes a few seconds to complete.");
}

fn render_flashing_info(ui: &mut Ui) {
    ui.add(egui::Label::new(
        RichText::new("Flashing Firmware")
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label("Please wait while the firmware is being written to your device.");
    ui.add_space(STANDARD_SPACING);
    ui.label("This typically takes 1-2 minutes to complete.");
    ui.add_space(STANDARD_SPACING);
    ui.label("If the process completes immediately, it likely failed.");
}

fn render_technical_info_frame(ui: &mut Ui, option: &FlashingOption, operation_name: &str) {
    egui::Frame::none()
        .fill(ui.style().visuals.faint_bg_color)
        .rounding(egui::Rounding::same(12.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)))
        .inner_margin(egui::Margin::same(15.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    RichText::new("Technical Information").size(TECHNICAL_INFO_SIZE),
                ));
                ui.add_space(STANDARD_SPACING);
                ui.label(format!("Interface: {}", option.get_driver_type()));
                ui.label(format!("Operation Type: {}", operation_name));

                let device_type = get_device_type(option);
                ui.label(format!("Target Device: {}", device_type));
            });
        });
}

fn get_device_type(option: &FlashingOption) -> &'static str {
    let display_name = option.get_display_name();

    if display_name.contains("CH347 - 35T, 75T, 100T DNA Read") {
        "CH347"
    } else if display_name.contains("RS232 - 35T") {
        "Artix-7 35T (RS232)"
    } else if display_name.contains("RS232 - 75T") {
        "Artix-7 75T (RS232)"
    } else if display_name.contains("CH347 - 35T") {
        "Artix-7 35T (CH347)"
    } else if display_name.contains("CH347 - 75T") {
        "Artix-7 75T (CH347)"
    } else if display_name.contains("CH347 - Stark100T") {
        "Artix-7 100T (CH347)"
    } else {
        "Unknown Device"
    }
}

// Add this function to extract a user-friendly status
fn get_user_friendly_status(manager: &FlashingManager) -> String {
    // Check if it's a DNA read operation
    if manager
        .get_current_option()
        .is_some_and(|opt| opt.is_dna_read())
    {
        crate::device_programmer::dna::DnaReader::get_dna_read_stage(&manager.get_status())
    } else {
        // The compiler error is here - we need to use the right function with the right parameters
        let now = std::time::Instant::now();
        let (stage_message, _, _) = determine_current_stage(manager, now);
        stage_message
    }
}
