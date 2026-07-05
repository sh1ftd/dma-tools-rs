use super::FirmwareToolApp;
use crate::utils::file_checker::CheckStatus;
use crate::utils::window::WindowSizeType;

#[derive(PartialEq, Eq)]
pub(super) enum AppState {
    FileCheck,
    OperationSelection,
    FirmwareSelection,
    FlashingOptions,
    Flashing,
    Result,
    Drivers,
    PcileechTest,
}

impl FirmwareToolApp {
    fn is_dna_read_operation(&self) -> bool {
        self.selected_option
            .as_ref()
            .is_some_and(|option| option.is_dna_read())
    }

    fn is_flash_operation(&self) -> bool {
        self.selected_option
            .as_ref()
            .is_some_and(|option| option.is_flash_operation())
    }

    pub(super) fn get_window_size_type(&self) -> WindowSizeType {
        match self.state {
            AppState::FileCheck => {
                if let CheckStatus::Complete(result) = self.file_checker.get_status()
                    && result.error_count > 0
                {
                    return WindowSizeType::MissingFiles;
                }
                WindowSizeType::FileCheck
            }
            AppState::OperationSelection => WindowSizeType::OperationSelection,
            AppState::FirmwareSelection => WindowSizeType::FileSelection,
            AppState::FlashingOptions => {
                match (self.is_dna_read_operation(), self.is_flash_operation()) {
                    (true, _) => WindowSizeType::ReadOptionSelection,
                    (_, true) => WindowSizeType::FlashOptionSelection,
                    _ => WindowSizeType::FlashOptionSelection, // fallback but should never happen
                }
            }
            AppState::Flashing | AppState::Result => WindowSizeType::OperationResult,
            AppState::Drivers => WindowSizeType::Drivers,
            AppState::PcileechTest => WindowSizeType::PcileechTest,
        }
    }

    pub(super) fn should_show_log(&self) -> bool {
        matches!(self.state, AppState::Flashing | AppState::Result)
    }
}
