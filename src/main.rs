// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
#[cfg(feature = "branding")]
mod branding;
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

    // Use branded title when branding feature is enabled
    #[cfg(feature = "branding")]
    let window_title = branding::get_branded_title(APP_TITLE, VERSION);

    #[cfg(not(feature = "branding"))]
    let window_title = format!("{APP_TITLE} v{VERSION}");

    // Try with default renderer first (Glow)
    let result = run_app(&window_title, eframe::Renderer::default());

    // If default renderer failed, try with WGPU
    if let Err(err) = result {
        eprintln!("Default renderer failed: {err}. Falling back to WGPU renderer...");
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
            Ok(Box::new(app::FirmwareToolApp::new(cc)))
        }),
    )
}

fn create_window_options() -> eframe::NativeOptions {
    let window_width = WINDOW_WIDTH;
    let window_height = WINDOW_HEIGHT_INITIAL;

    #[cfg(feature = "branding")]
    let icon_data = branding::get_window_icon();

    #[cfg(not(feature = "branding"))]
    let icon_data: Option<egui::IconData> = None;

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([window_width, window_height])
        .with_min_inner_size([window_width, window_height])
        .with_max_inner_size([window_width, window_height])
        .with_resizable(false)
        .with_decorations(true)
        .with_maximized(false)
        .with_fullscreen(false)
        .with_icon(icon_data.map(std::sync::Arc::new).unwrap_or_default());

    // Center the window on the screen
    if let Some(monitor_size) = get_primary_monitor_size() {
        let x = (monitor_size.x - window_width) / 2.0;
        let y = (monitor_size.y - window_height) / 2.0;
        viewport = viewport.with_position([x, y]);
    }

    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

fn get_primary_monitor_size() -> Option<egui::Vec2> {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    // SAFETY: GetSystemMetrics is a read-only Windows API call that returns screen dimensions
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN) as f32;
        let height = GetSystemMetrics(SM_CYSCREEN) as f32;

        if width > 0.0 && height > 0.0 {
            Some(egui::Vec2::new(width, height))
        } else {
            None
        }
    }
}

fn setup_window() {
    utils::win_utils::setup_window_controls();
}
