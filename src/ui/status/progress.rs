use crate::device_programmer::{FlashingManager, FlashingOption, dna::DnaReader};
use crate::utils::localization::{translate, TextKey};
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

pub fn render_flashing_progress(
    ui: &mut Ui,
    manager: &FlashingManager,
    lang: &crate::app::Language,
) {
    let now = Instant::now();

    let (_stage_message, _current_sector, _is_writing) = determine_current_stage(manager, now, lang);

    if let Some(option) = manager.get_current_option() {
        let is_dna_read = option.is_dna_read();
        let operation_name = if is_dna_read { "DNA Read" } else { "Flashing" };

        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "{} - {}",
                if is_dna_read { translate(TextKey::ReadingDeviceDna, lang) } else { translate(TextKey::FlashingFirmware, lang) },
                option.get_display_name()
            ));

            ui.add_space(MEDIUM_SPACING);

            let status_text = get_user_friendly_status(manager, lang);

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

fn determine_current_stage(
    manager: &FlashingManager,
    now: Instant,
    lang: &crate::app::Language,
) -> (String, Option<u32>, bool) {
    let logger = manager.logger();
    let entries = logger.get_entries();

    let mut current_stage = translate(TextKey::StartingOperation, lang).to_string();
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
                if let Some(first_seen) = sector_times.get(&sector)
                    && now.duration_since(*first_seen) > SECTOR_STUCK_THRESHOLD
                {
                    is_finalizing = true;
                }

                current_sector = Some(sector);
                is_writing = true;
                break;
            }
        } else if msg.contains("Writing the image to the flash memory") {
            current_stage = translate(TextKey::WritingImage, lang).to_string();
            is_writing = true;
            break;
        } else if msg.contains("Probing the flash memory") {
            current_stage = translate(TextKey::ProbingFlash, lang).to_string();
            break;
        } else if msg.contains("Resetting and halting the FPGA") {
            current_stage = translate(TextKey::ResettingFpga, lang).to_string();
            break;
        } else if msg.contains("Loading the bitstream") {
            current_stage = translate(TextKey::LoadingBitstream, lang).to_string();
            break;
        } else if msg.contains("Initializing the JTAG interface") {
            current_stage = translate(TextKey::InitJtag, lang).to_string();
            break;
        }
    }

    // Create the appropriate message based on current state
    let stage_message = if is_writing {
        if let Some(sector) = current_sector {
            if is_finalizing {
                translate(TextKey::Verifying, lang).to_string()
            } else {
                format!("{} {}...", translate(TextKey::WritingSector, lang), sector)
            }
        } else {
            current_stage.to_string()
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

fn render_operation_info_frame(ui: &mut Ui, is_dna_read: bool, lang: &crate::app::Language) {
    egui::Frame::NONE
        .fill(ui.style().visuals.extreme_bg_color)
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
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

fn render_dna_read_info(ui: &mut Ui, lang: &crate::app::Language) {
    ui.add(egui::Label::new(
        RichText::new(translate(TextKey::ReadingDeviceDna, lang))
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label(translate(TextKey::PleaseWaitDna, lang));
    ui.add_space(STANDARD_SPACING);
    ui.label(translate(TextKey::DnaTakesSeconds, lang));
}

fn render_flashing_info(ui: &mut Ui, lang: &crate::app::Language) {
    ui.add(egui::Label::new(
        RichText::new(translate(TextKey::FlashingFirmware, lang))
            .size(HEADING_SIZE)
            .strong(),
    ));
    ui.add_space(LARGE_SPACING);
    ui.label(translate(TextKey::PleaseWaitFlash, lang));
    ui.add_space(STANDARD_SPACING);
    ui.label(translate(TextKey::FlashTakesMinutes, lang));
    ui.add_space(STANDARD_SPACING);
    ui.label(translate(TextKey::FlashFailImmediate, lang));
}

fn render_technical_info_frame(
    ui: &mut Ui,
    option: &FlashingOption,
    _operation_name: &str,
    lang: &crate::app::Language,
) {
    egui::Frame::NONE
        .fill(ui.style().visuals.faint_bg_color)
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)))
        .inner_margin(egui::Margin::same(15))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    RichText::new(translate(TextKey::TechnicalInfo, lang)).size(TECHNICAL_INFO_SIZE),
                ));
                ui.add_space(STANDARD_SPACING);
                ui.label(format!("{} {}", translate(TextKey::InterfaceLabel, lang), option.get_driver_type()));
                
                let op_type_str = if option.is_dna_read() { translate(TextKey::ReadingDeviceDna, lang) } else { translate(TextKey::FlashingFirmware, lang) };
                ui.label(format!("{} {}", translate(TextKey::OperationTypeLabel, lang), op_type_str));

                let device_type = get_device_type(option);
                ui.label(format!("{} {}", translate(TextKey::TargetDeviceLabel, lang), device_type));
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
    } else if display_name.contains("RS232 - 100T") {
        "Artix-7 100T (RS232)"
    } else if display_name.contains("CH347 - 35T") {
        "Artix-7 35T (CH347)"
    } else if display_name.contains("CH347 - 75T") {
        "Artix-7 75T (CH347)"
    } else if display_name.contains("CH347 - 100T") {
        "Artix-7 100T (CH347)"
    } else {
        "Unknown Device"
    }
}

// Add this function to extract a user-friendly status
fn get_user_friendly_status(manager: &FlashingManager, lang: &crate::app::Language) -> String {
    // Check if it's a DNA read operation
    if manager
        .get_current_option()
        .is_some_and(|opt| opt.is_dna_read())
    {
        DnaReader::get_dna_read_stage(&manager.get_status(), lang)
    } else {
        let now = std::time::Instant::now();
        let (stage_message, _, _) = determine_current_stage(manager, now, lang);
        stage_message
    }
}
