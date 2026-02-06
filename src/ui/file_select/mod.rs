mod check;
mod components;
mod firmware;

use crate::utils::file_checker::CheckStatus;
use eframe::egui::Ui;

/// Context for file check rendering containing all required parameters
pub struct FileCheckRenderContext<'a> {
    pub ui: &'a mut Ui,
    pub check_status: &'a CheckStatus,
    pub on_continue: &'a mut dyn FnMut(bool),
    pub on_rescan: &'a mut dyn FnMut(),
    pub language: &'a crate::app::Language,
}

// Re-export the main entry points
pub use check::render_file_check;
pub use firmware::render_firmware_selection;
