use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::localization::{translate, TextKey};
use eframe::egui::{
    self, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2,
};
use std::path::PathBuf;

// UI constants for consistent styling
const BUTTON_SIZE: Vec2 = Vec2::new(200.0, 30.0);
const PRIMARY_COLOR: Color32 = Color32::from_rgb(70, 130, 180);
const DISABLED_COLOR_FACTOR: f32 = 0.5;
const BORDER_COLOR: Color32 = Color32::from_rgb(150, 150, 150);
const BORDER_WIDTH: f32 = 1.0;
const CORNER_RADIUS: u8 = 12;
const PADDING: i8 = 6;
const SCROLL_HEIGHT: f32 = 80.0;

// Text sizes
const HEADING_SIZE: f32 = 18.0;
const NORMAL_SIZE: f32 = 16.0;

pub fn render_firmware_selection(
    ui: &mut Ui,
    firmware_manager: &mut FirmwareManager,
    on_select: &mut dyn FnMut(Option<PathBuf>),
    is_scanning: bool,
    lang: &crate::app::Language,
) {
    let files: Vec<(usize, PathBuf, bool)> = firmware_manager
        .get_firmware_files()
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let selected = firmware_manager.get_selected_firmware() == Some(path);
            (i, path.clone(), selected)
        })
        .collect();

    if is_scanning && (files.is_empty() || firmware_manager.get_scan_count() <= 1) {
        render_firmware_status(ui, translate(TextKey::ScanningFirmware, lang), lang);
    } else if files.is_empty() {
        render_firmware_status(ui, translate(TextKey::NoFirmwareFound, lang), lang);
    } else {
        render_firmware_list(ui, &files, firmware_manager, on_select, is_scanning, lang);
    }
}

fn render_firmware_status(ui: &mut Ui, status_message: &str, lang: &crate::app::Language) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SelectFirmware, lang));

        // Center the spinner horizontally
        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.add_space(available_width / 2.0 - 10.0);
            Frame::NONE.show(ui, |ui| {
                ui.spinner();
            });
        });

        ui.label(RichText::new(status_message).size(HEADING_SIZE).strong());

        ui.add_space(8.0);
        ui.label(
            RichText::new(translate(TextKey::PlaceFirmwareHere, lang))
                .size(NORMAL_SIZE),
        );

        ui.add_space(18.0);
        ui.label(
            RichText::new(translate(TextKey::AutoScanning, lang))
                .size(NORMAL_SIZE)
                .italics(),
        );
    });
}

fn render_firmware_list(
    ui: &mut Ui,
    files: &[(usize, PathBuf, bool)],
    firmware_manager: &mut FirmwareManager,
    on_select: &mut dyn FnMut(Option<PathBuf>),
    is_scanning: bool,
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        render_status_bar(ui, is_scanning, lang);
        render_file_list(ui, files, firmware_manager);

        let mut cleanup_enabled = firmware_manager.get_cleanup_enabled();
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cleanup_enabled, translate(TextKey::PerformCleanup, lang))
                .changed()
            {
                firmware_manager.set_cleanup_enabled(cleanup_enabled);
            }
            ui.label(translate(TextKey::CleanupDescription, lang));
        });

        ui.add_space(8.0);

        render_continue_button(ui, firmware_manager, on_select, lang);
    });
}

fn render_status_bar(ui: &mut Ui, is_scanning: bool, lang: &crate::app::Language) {
    ui.horizontal(|ui| {
        ui.label(translate(TextKey::SelectFirmware, lang));

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if is_scanning {
                ui.spinner();
                ui.label(RichText::new(translate(TextKey::ScanningFirmware, lang)).italics());
            } else {
                ui.small(RichText::new(translate(TextKey::AutoRefreshing, lang)).italics());
            }
        });
    });

    ui.add_space(4.0);
}

fn render_file_list(
    ui: &mut Ui,
    files: &[(usize, PathBuf, bool)],
    firmware_manager: &mut FirmwareManager,
) {
    let file_list_frame = Frame::dark_canvas(ui.style())
        .stroke(Stroke::new(BORDER_WIDTH, BORDER_COLOR))
        .corner_radius(CornerRadius::same(CORNER_RADIUS))
        .inner_margin(Margin::same(PADDING));

    file_list_frame.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .max_height(SCROLL_HEIGHT)
            .show(ui, |ui| {
                for (i, file, selected) in files {
                    let file_name = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");

                    let text = format!("{}. {}", i + 1, file_name);

                    let response =
                        ui.selectable_label(*selected, RichText::new(text).size(NORMAL_SIZE));

                    if response.clicked() {
                        firmware_manager.select_firmware(*i);
                    }

                    response.on_hover_text(file.to_string_lossy());
                }
            });
    });
}

fn render_continue_button(
    ui: &mut Ui,
    firmware_manager: &FirmwareManager,
    on_select: &mut dyn FnMut(Option<PathBuf>),
    lang: &crate::app::Language,
) {
    ui.add_space(16.0);

    if let Some(selected) = firmware_manager.get_selected_firmware() {
        let button = egui::Button::new(RichText::new(translate(TextKey::Continue, lang)).size(HEADING_SIZE))
            .min_size(BUTTON_SIZE)
            .fill(PRIMARY_COLOR);

        if ui.add(button).clicked() {
            on_select(Some(selected.clone()));
        }
    } else {
        ui.add_enabled(
            false,
            egui::Button::new(RichText::new(translate(TextKey::Continue, lang)).size(HEADING_SIZE))
                .min_size(BUTTON_SIZE)
                .fill(PRIMARY_COLOR.gamma_multiply(DISABLED_COLOR_FACTOR)),
        );
        ui.label(translate(TextKey::SelectFirmwareToContinue, lang));
    }
}
