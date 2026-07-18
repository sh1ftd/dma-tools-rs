pub mod arabic;
pub mod chinese;
pub mod english;
pub mod german;
pub mod portuguese;
pub mod reshaper;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Language {
    English,
    Chinese,
    German,
    Portuguese,
    Arabic,
}

#[allow(non_camel_case_types)]
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
    Drivers,
    TestPcileech,
    SelectFirmware,
    ScanningFirmware,

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
    DriversDesc,
    TestPcileechDesc,
    // Drivers
    DriversMenuTitle,
    DataPortDrivers,
    JtagDrivers,
    InstallFtdiDriver,
    OpenZadig,
    InstallCh347Driver,
    RequiresAdmin,
    // Test PCILeech
    TestPcileechTitle,
    TestingConnection,
    TestSuccess,
    TestFailed,
    ConnectionError,
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

/// Interpolates localized placeholders after language-specific shaping.
///
/// Most translations use `{}`. Arabic source strings use `}{` where bidi
/// shaping requires the braces to be authored in visual order. Parsing both
/// forms here keeps call sites independent from that representation detail.
/// The template is scanned only once so braces inside inserted values are not
/// mistaken for later placeholders.
pub fn format_translation(template: &str, values: &[&str]) -> String {
    let bytes = template.as_bytes();
    let mut rendered = String::with_capacity(
        template.len() + values.iter().map(|value| value.len()).sum::<usize>(),
    );
    let mut cursor = 0;
    let mut scan = 0;
    let mut value_index = 0;

    while scan + 1 < bytes.len() && value_index < values.len() {
        let is_placeholder = matches!((bytes[scan], bytes[scan + 1]), (b'{', b'}') | (b'}', b'{'));

        if is_placeholder {
            rendered.push_str(&template[cursor..scan]);
            rendered.push_str(values[value_index]);
            value_index += 1;
            scan += 2;
            cursor = scan;
        } else {
            scan += 1;
        }
    }

    rendered.push_str(&template[cursor..]);

    // Do not discard diagnostics if a future translation accidentally omits a
    // placeholder. A readable fallback is preferable to losing the value.
    for value in &values[value_index..] {
        if !rendered.is_empty() {
            rendered.push_str(": ");
        }
        rendered.push_str(value);
    }

    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_standard_and_rtl_placeholders() {
        assert_eq!(
            format_translation("first {} second {}", &["one", "two"]),
            "first one second two"
        );
        assert_eq!(
            format_translation("first }{ second }{", &["one", "two"]),
            "first one second two"
        );
    }

    #[test]
    fn does_not_reparse_braces_inside_inserted_values() {
        assert_eq!(
            format_translation("first {} second {}", &["a{}b", "error"]),
            "first a{}b second error"
        );
    }

    #[test]
    fn preserves_values_when_a_translation_omits_placeholders() {
        assert_eq!(
            format_translation("diagnostic", &["path", "access denied"]),
            "diagnostic: path: access denied"
        );
    }

    #[test]
    fn formats_actual_arabic_status_templates() {
        let dna_message = format_translation(
            translate(TextKey::DnaReadFailedStatus, &Language::Arabic),
            &["access denied"],
        );
        assert!(dna_message.contains("access denied"));
        assert!(!dna_message.contains("{}"));
        assert!(!dna_message.contains("}{"));

        let connection_message = format_translation(
            translate(TextKey::FlashingFailedConnectionMsg, &Language::Arabic),
            &["4", "10"],
        );
        assert!(connection_message.contains('4'));
        assert!(connection_message.contains("10"));
        assert!(!connection_message.contains("{}"));
        assert!(!connection_message.contains("}{"));
    }
}
