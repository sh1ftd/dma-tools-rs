// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod device_programmer;
mod ui;
mod utils;

use crate::utils::cleanup::perform_startup_cleanup;
use crate::utils::logger::Logger;
use crate::utils::window::{WINDOW_HEIGHT_INITIAL, WINDOW_WIDTH};
use eframe::egui;

const APP_TITLE: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), eframe::Error> {
    let logger = Logger::new("DMA-Tools");

    #[cfg(debug_assertions)]
    {
        logger.set_debug_mode(true);
        logger.info("Debug build detected - debug mode enabled");
    }

    // Perform cleanup operations at startup
    perform_startup_cleanup(&logger);

    let window_title = format!("{} v{}", APP_TITLE, VERSION);

    // Try with default renderer first (Glow)
    let result = run_app(&window_title, eframe::Renderer::default());

    // If default renderer failed, try with WGPU
    if let Err(err) = result {
        eprintln!(
            "Default renderer failed: {}. Falling back to WGPU renderer...",
            err
        );
        return run_app(&window_title, eframe::Renderer::Wgpu);
    }

    result
}

fn run_app(window_title: &str, renderer: eframe::Renderer) -> Result<(), eframe::Error> {
    let mut options = create_window_options();
    options.renderer = renderer;

    // Print which renderer we're using
    match renderer {
        eframe::Renderer::Glow => println!("Using Glow renderer"),
        eframe::Renderer::Wgpu => println!("Using WGPU renderer"),
    }

    eframe::run_native(
        window_title,
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
