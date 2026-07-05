use crate::ui::common;
use crate::ui::common::palette;
use crate::utils::firmware_discovery::FirmwareManager;
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{
    self, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2,
};
use std::path::PathBuf;

const BORDER_COLOR: Color32 = palette::STROKE;
const BORDER_WIDTH: f32 = 1.0;
const CORNER_RADIUS: u8 = 12;
const PADDING: i8 = 6;
const SCROLL_HEIGHT: f32 = 80.0;

// Text sizes
const HEADING_SIZE: f32 = 18.0;
const NORMAL_SIZE: f32 = 16.0;
const SECONDARY_SIZE: f32 = 14.5;
const SECONDARY_COLOR: Color32 = palette::TEXT_MUTED;

pub fn render_firmware_selection(
    ui: &mut Ui,
    firmware_manager: &mut FirmwareManager,
    on_select: &mut dyn FnMut(Option<PathBuf>),
    on_back: &mut dyn FnMut(),
    is_scanning: bool,
    lang: &crate::utils::localization::Language,
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
        render_firmware_status(
            ui,
            translate(TextKey::ScanningFirmware, lang),
            on_back,
            lang,
        );
    } else if files.is_empty() {
        render_firmware_status(ui, translate(TextKey::NoFirmwareFound, lang), on_back, lang);
    } else {
        render_firmware_list(
            ui,
            &files,
            firmware_manager,
            on_select,
            on_back,
            is_scanning,
            lang,
        );
    }
}

fn render_firmware_status(
    ui: &mut Ui,
    status_message: &str,
    on_back: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
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
        ui.label(RichText::new(translate(TextKey::PlaceFirmwareHere, lang)).size(NORMAL_SIZE));

        ui.add_space(18.0);
        ui.label(
            RichText::new(translate(TextKey::AutoScanning, lang))
                .size(SECONDARY_SIZE)
                .color(SECONDARY_COLOR)
                .italics(),
        );

        ui.add_space(20.0);

        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.add_space(available_width / 2.0 - 100.0);
            if common::secondary_icon_button(
                ui,
                Some(egui_phosphor::regular::HOUSE),
                translate(TextKey::MainMenu, lang),
                Vec2::new(200.0, 32.0),
            )
            .clicked()
            {
                on_back();
            }
        });
    });
}

fn render_firmware_list(
    ui: &mut Ui,
    files: &[(usize, PathBuf, bool)],
    firmware_manager: &mut FirmwareManager,
    on_select: &mut dyn FnMut(Option<PathBuf>),
    on_back: &mut dyn FnMut(),
    is_scanning: bool,
    lang: &crate::utils::localization::Language,
) {
    ui.vertical_centered(|ui| {
        render_status_bar(ui, is_scanning, lang);
        render_file_list(ui, files, firmware_manager);

        let mut cleanup_enabled = firmware_manager.get_cleanup_enabled();
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut cleanup_enabled,
                    translate(TextKey::PerformCleanup, lang),
                )
                .changed()
            {
                firmware_manager.set_cleanup_enabled(cleanup_enabled);
            }
            ui.label(
                RichText::new(translate(TextKey::CleanupDescription, lang))
                    .size(SECONDARY_SIZE)
                    .color(SECONDARY_COLOR),
            );
        });

        ui.add_space(8.0);

        render_continue_button(ui, firmware_manager, on_select, on_back, lang);
    });
}

fn render_status_bar(ui: &mut Ui, is_scanning: bool, lang: &crate::utils::localization::Language) {
    ui.horizontal(|ui| {
        ui.label(translate(TextKey::SelectFirmware, lang));

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if is_scanning {
                ui.spinner();
                ui.label(
                    RichText::new(translate(TextKey::ScanningFirmware, lang))
                        .size(SECONDARY_SIZE)
                        .color(SECONDARY_COLOR)
                        .italics(),
                );
            } else {
                ui.label(
                    RichText::new(translate(TextKey::AutoRefreshing, lang))
                        .size(SECONDARY_SIZE)
                        .color(SECONDARY_COLOR)
                        .italics(),
                );
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
    on_back: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let spacing = 12.0;
        let button_width = (available_width - spacing) / 2.0;

        if common::secondary_icon_button(
            ui,
            Some(egui_phosphor::regular::HOUSE),
            translate(TextKey::MainMenu, lang),
            Vec2::new(button_width, 32.0),
        )
        .clicked()
        {
            on_back();
        }

        ui.add_space(spacing);

        if let Some(selected) = firmware_manager.get_selected_firmware() {
            if common::primary_icon_button(
                ui,
                Some(egui_phosphor::regular::ARROW_RIGHT),
                translate(TextKey::Continue, lang),
                Vec2::new(button_width, 32.0),
            )
            .clicked()
            {
                on_select(Some(selected.clone()));
            }
        } else {
            common::disabled_primary_icon_button(
                ui,
                Some(egui_phosphor::regular::ARROW_RIGHT),
                translate(TextKey::Continue, lang),
                Vec2::new(button_width, 32.0),
            );
        }
    });

    if firmware_manager.get_selected_firmware().is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(translate(TextKey::SelectFirmwareToContinue, lang))
                    .size(SECONDARY_SIZE)
                    .color(SECONDARY_COLOR),
            );
        });
    }
}
