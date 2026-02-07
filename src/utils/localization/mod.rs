use crate::app::Language;

pub mod arabic;
pub mod chinese;
pub mod english;
pub mod german;
pub mod portuguese;
pub mod reshaper;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextKey {
    OperationLog,
    Contact,
    CopyTelegram,
    CopyWeChat,
    CopyDiscord,
    TelegramLink,
    WeChatID,
    DiscordID,
    Copied,
    CheckingFiles,
    MissingFiles,
    FileCheckSuccess,
    SystemCheck,
    WelcomeMessage,
    CheckingItem,
    CountdownMessage,
    ExitButton,
    MissingFilesWarning,
    GroupExecutables,
    GroupLibraries,
    GroupBitstreams,
    GroupConfigs,
    GroupOther,
    ContinueAnyway,
    Rescan,
    SelectOperation,
    FlashFirmware,
    ReadDna,
    SelectFirmware,
    ScanningFirmware,
    SelectOption,
    Flash,
    Read,
    Back,
    FlashingInProgress,
    ReadingDnaInProgress,
    Success,
    Failed,
    TryAgain,
    ReturnToMenu,
    DownloadHere,
    UpdateAvailable,
    // Firmware Selection
    NoFirmwareFound,
    PlaceFirmwareHere,
    AutoScanning,
    AutoRefreshing,
    PerformCleanup,
    CleanupDescription,
    Continue,
    SelectFirmwareToContinue,
    FlashFirmwareDesc,
    ReadDnaDesc,
    // Flashing Options
    SelectFlashingOption,
    SelectDnaReadOption,
    Ch347Options,
    Rs232Options,
    // Option Labels & Descriptions
    Ch347_35T_Label,
    Ch347_35T_Desc,
    Ch347_75T_Label,
    Ch347_75T_Desc,
    Ch347_100T_Label,
    Ch347_100T_Desc,
    Rs232_35T_Label,
    Rs232_35T_Desc,
    Rs232_75T_Label,
    Rs232_75T_Desc,
    Rs232_100T_Label,
    Rs232_100T_Desc,
    // DNA Read Labels & Descriptions
    Dna_Ch347_Label,
    Dna_Ch347_Desc,
    Dna_Rs232_35T_Label,
    Dna_Rs232_35T_Desc,
    Dna_Rs232_75T_Label,
    Dna_Rs232_75T_Desc,
    Dna_Rs232_100T_Label,
    Dna_Rs232_100T_Desc,
    // Log View
    ClearLog,

    // Result Extras
    OperationTook,
    NoteFewerSectors,
    NoteVerifySuccess,
    ErrorDetails,

    // Progress
    Initializing,
    StartingOperation,
    WritingImage,
    ProbingFlash,
    ResettingFpga,
    LoadingBitstream,
    InitJtag,
    Verifying,
    WritingSector, // "Writing sector {}..."
    ReadingDeviceDna,
    PleaseWaitDna,
    DnaTakesSeconds,
    FlashingFirmware,
    PleaseWaitFlash,
    FlashTakesMinutes,
    FlashFailImmediate,
    TechnicalInfo,
    InterfaceLabel,
    OperationTypeLabel,
    TargetDeviceLabel,
    // Result
    DnaReadSuccess,
    DnaReadFailed,
    DnaReadUnexpected,
    DeviceDnaHeader,
    ClickToCopy,
    FlashingSuccess,
    FlashingFailed,
    FlashingFailedConnection,
    FlashingResultUnknown,
    NextSteps,
    NextStepsList, // Multiline string for the steps
    Exit,
    MainMenu,
    TryAgainButton, // TryAgain exists?

    // Detailed Result Messages
    DnaReadUnexpectedMsg,
    DnaReadFailedPrefix,
    OperationInProgress,
    DnaStatusUnknownMsg,
    ClickToCopyTooltip,
    FlashingFailedConnectionMsg,
    FlashingResultUnknownMsg,
    UnexpectedStateMsg,
    FlashingFailedPrefix,
    FlashStatusUnknownMsg,

    // DNA Backend & Status
    DnaInvalidOption,
    DnaCommandFailed,
    DnaFileNotFound,
    DnaExtractFailed,
    DnaFileReadError,
    DnaInfoNotFound,
    DnaWaitingStart,
    DnaRetrieving,
    DnaReadSuccessStatus,
    DnaOperationCompleted,
    DnaReadFailedStatus,
}

// Use a static cache to store reshaped Arabic strings so we can return &'static str.
// This is acceptable because the number of labels is small and fixed.
static ARABIC_CACHE: OnceLock<Mutex<HashMap<TextKey, &'static str>>> = OnceLock::new();

pub fn translate(key: TextKey, lang: &Language) -> &'static str {
    let text = match lang {
        Language::English => english::get_text(key),
        Language::Chinese => chinese::get_text(key),
        Language::German => german::get_text(key),
        Language::Portuguese => portuguese::get_text(key),
        Language::Arabic => arabic::get_text(key),
    };

    if *lang == Language::Arabic {
        let cache_mutex = ARABIC_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache_mutex.lock().unwrap();
        if let Some(reshaped) = cache.get(&key) {
            return reshaped;
        }
        let reshaped: &'static str = Box::leak(reshaper::reshape_arabic(text).into_boxed_str());
        cache.insert(key, reshaped);
        reshaped
    } else {
        text
    }
}
