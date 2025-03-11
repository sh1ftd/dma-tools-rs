// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod device_programmer;
mod ui;
mod utils;

use crate::utils::window::{WINDOW_HEIGHT_INITIAL, WINDOW_WIDTH};
use eframe::egui;

const APP_TITLE: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), eframe::Error> {
    let window_title = format!("{} v{}", APP_TITLE, VERSION);
    let options = create_window_options();

    eframe::run_native(
        &window_title,
        options,
        Box::new(|cc| {
            setup_window();
            Box::new(app::FirmwareToolApp::new(cc))
        }),
    )
}

fn create_window_options() -> eframe::NativeOptions {
    let window_width = WINDOW_WIDTH;
    let window_height = WINDOW_HEIGHT_INITIAL;

    eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(window_width, window_height)),
        min_window_size: Some(egui::vec2(window_width, window_height)),
        max_window_size: Some(egui::vec2(window_width, window_height)),
        resizable: false,
        centered: true,
        decorated: true,
        maximized: false,
        fullscreen: false,
        icon_data: None,
        ..Default::default()
    }
}

fn setup_window() {
    utils::win_utils::setup_window_controls();
}
