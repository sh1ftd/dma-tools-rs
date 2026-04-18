#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaInfo {
    pub dna_value: String,     // Hex value (e.g., 0x...)
    pub dna_raw_value: String, // Binary value (e.g., 0011...)
    pub device_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashingOption {
    CH347_35T,
    CH347_75T,
    CH347_100T,
    RS232_35T,
    RS232_75T,
    RS232_100T,
    DnaCH347,
    DnaRS232_35T,
    DnaRS232_75T,
    DnaRS232_100T,
}

// Constants for OpenOCD executable paths
pub const OPENOCD_CH347_PATH: &str = "OpenOCD/openocd-347.exe";
pub const OPENOCD_RS232_PATH: &str = "OpenOCD/openocd.exe";

impl FlashingOption {
    pub fn is_dna_read(&self) -> bool {
        matches!(
            self,
            FlashingOption::DnaCH347
                | FlashingOption::DnaRS232_35T
                | FlashingOption::DnaRS232_75T
                | FlashingOption::DnaRS232_100T
        )
    }

    pub fn is_flash_operation(&self) -> bool {
        matches!(
            self,
            FlashingOption::CH347_35T
                | FlashingOption::CH347_75T
                | FlashingOption::CH347_100T
                | FlashingOption::RS232_35T
                | FlashingOption::RS232_75T
                | FlashingOption::RS232_100T
        )
    }

    pub fn get_command_args(&self) -> (&'static str, &'static str) {
        match self {
            FlashingOption::CH347_35T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a35T.cfg"),
            FlashingOption::CH347_75T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a75T.cfg"),
            FlashingOption::CH347_100T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a100T.cfg"),
            FlashingOption::RS232_35T => (OPENOCD_RS232_PATH, "OpenOCD/flash/xc7a35T_rs232.cfg"),
            FlashingOption::RS232_75T => (OPENOCD_RS232_PATH, "OpenOCD/flash/xc7a75T_rs232.cfg"),
            FlashingOption::RS232_100T => (OPENOCD_RS232_PATH, "OpenOCD/flash/xc7a100T_rs232.cfg"),
            FlashingOption::DnaCH347 => (OPENOCD_CH347_PATH, "OpenOCD/DNA/init_347.cfg"),
            FlashingOption::DnaRS232_35T => (OPENOCD_RS232_PATH, "OpenOCD/DNA/init_232_35t.cfg"),
            FlashingOption::DnaRS232_75T => (OPENOCD_RS232_PATH, "OpenOCD/DNA/init_232_75t.cfg"),
            FlashingOption::DnaRS232_100T => (OPENOCD_RS232_PATH, "OpenOCD/DNA/init_232_100t.cfg"),
        }
    }

    pub fn get_display_name(&self) -> &'static str {
        match self {
            FlashingOption::CH347_35T => "CH347 - 35T",
            FlashingOption::CH347_75T => "CH347 - 75T",
            FlashingOption::CH347_100T => "CH347 - 100T",

            FlashingOption::RS232_35T => "RS232 - 35T",
            FlashingOption::RS232_75T => "RS232 - 75T",
            FlashingOption::RS232_100T => "RS232 - 100T",

            FlashingOption::DnaCH347 => "CH347 - 35T, 75T, 100T DNA Read",

            FlashingOption::DnaRS232_35T => "RS232 - 35T DNA Read",
            FlashingOption::DnaRS232_75T => "RS232 - 75T DNA Read",
            FlashingOption::DnaRS232_100T => "RS232 - 100T DNA Read",
        }
    }

    pub fn get_driver_type(&self) -> &'static str {
        match self {
            FlashingOption::CH347_35T
            | FlashingOption::CH347_75T
            | FlashingOption::CH347_100T
            | FlashingOption::DnaCH347 => "CH347 USB Driver",
            FlashingOption::RS232_35T
            | FlashingOption::RS232_75T
            | FlashingOption::RS232_100T
            | FlashingOption::DnaRS232_35T
            | FlashingOption::DnaRS232_75T
            | FlashingOption::DnaRS232_100T => "FTDI Driver",
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompletionStatus {
    NotCompleted,
    InProgress(String),
    Completed,                 // Flashing
    DnaReadCompleted(DnaInfo), // DNA read
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FlashingOption classification ──

    #[test]
    fn dna_variants_classified_correctly() {
        assert!(FlashingOption::DnaCH347.is_dna_read());
        assert!(FlashingOption::DnaRS232_35T.is_dna_read());
        assert!(FlashingOption::DnaRS232_75T.is_dna_read());
        assert!(FlashingOption::DnaRS232_100T.is_dna_read());
    }

    #[test]
    fn flash_variants_classified_correctly() {
        assert!(FlashingOption::CH347_35T.is_flash_operation());
        assert!(FlashingOption::CH347_75T.is_flash_operation());
        assert!(FlashingOption::CH347_100T.is_flash_operation());
        assert!(FlashingOption::RS232_35T.is_flash_operation());
        assert!(FlashingOption::RS232_75T.is_flash_operation());
        assert!(FlashingOption::RS232_100T.is_flash_operation());
    }

    #[test]
    fn dna_and_flash_are_mutually_exclusive() {
        let all = [
            FlashingOption::CH347_35T,
            FlashingOption::CH347_75T,
            FlashingOption::CH347_100T,
            FlashingOption::RS232_35T,
            FlashingOption::RS232_75T,
            FlashingOption::RS232_100T,
            FlashingOption::DnaCH347,
            FlashingOption::DnaRS232_35T,
            FlashingOption::DnaRS232_75T,
            FlashingOption::DnaRS232_100T,
        ];
        for opt in &all {
            assert_ne!(
                opt.is_dna_read(),
                opt.is_flash_operation(),
                "{opt:?} should be exactly one of dna_read or flash_operation"
            );
        }
    }

    // ── Command args ──

    #[test]
    fn ch347_variants_use_ch347_binary() {
        for opt in &[
            FlashingOption::CH347_35T,
            FlashingOption::CH347_75T,
            FlashingOption::CH347_100T,
            FlashingOption::DnaCH347,
        ] {
            let (exe, _) = opt.get_command_args();
            assert_eq!(exe, OPENOCD_CH347_PATH, "{opt:?} should use CH347 binary");
        }
    }

    #[test]
    fn rs232_variants_use_standard_binary() {
        for opt in &[
            FlashingOption::RS232_35T,
            FlashingOption::RS232_75T,
            FlashingOption::RS232_100T,
            FlashingOption::DnaRS232_35T,
            FlashingOption::DnaRS232_75T,
            FlashingOption::DnaRS232_100T,
        ] {
            let (exe, _) = opt.get_command_args();
            assert_eq!(exe, OPENOCD_RS232_PATH, "{opt:?} should use RS232 binary");
        }
    }

    #[test]
    fn flash_configs_in_flash_directory() {
        for opt in &[
            FlashingOption::CH347_35T,
            FlashingOption::RS232_75T,
            FlashingOption::RS232_100T,
        ] {
            let (_, cfg) = opt.get_command_args();
            assert!(
                cfg.contains("/flash/"),
                "{opt:?} config should be in flash/"
            );
        }
    }

    #[test]
    fn dna_configs_in_dna_directory() {
        for opt in &[
            FlashingOption::DnaCH347,
            FlashingOption::DnaRS232_35T,
            FlashingOption::DnaRS232_75T,
        ] {
            let (_, cfg) = opt.get_command_args();
            assert!(cfg.contains("/DNA/"), "{opt:?} config should be in DNA/");
        }
    }

    // ── Driver types ──

    #[test]
    fn driver_types_match_interface() {
        assert_eq!(
            FlashingOption::CH347_35T.get_driver_type(),
            "CH347 USB Driver"
        );
        assert_eq!(
            FlashingOption::DnaCH347.get_driver_type(),
            "CH347 USB Driver"
        );
        assert_eq!(FlashingOption::RS232_75T.get_driver_type(), "FTDI Driver");
        assert_eq!(
            FlashingOption::DnaRS232_100T.get_driver_type(),
            "FTDI Driver"
        );
    }

    // ── Display names ──

    #[test]
    fn display_names_are_non_empty() {
        let all = [
            FlashingOption::CH347_35T,
            FlashingOption::CH347_75T,
            FlashingOption::CH347_100T,
            FlashingOption::RS232_35T,
            FlashingOption::RS232_75T,
            FlashingOption::RS232_100T,
            FlashingOption::DnaCH347,
            FlashingOption::DnaRS232_35T,
            FlashingOption::DnaRS232_75T,
            FlashingOption::DnaRS232_100T,
        ];
        for opt in &all {
            assert!(!opt.get_display_name().is_empty(), "{opt:?}");
        }
    }

    // ── CompletionStatus ──

    #[test]
    fn terminal_statuses_detected() {
        let terminal = [
            CompletionStatus::Completed,
            CompletionStatus::Failed("err".into()),
            CompletionStatus::DnaReadCompleted(DnaInfo {
                dna_value: "0x1".into(),
                dna_raw_value: "1".into(),
                device_type: "CH347".into(),
            }),
        ];
        for s in &terminal {
            assert!(
                matches!(
                    s,
                    CompletionStatus::Completed
                        | CompletionStatus::DnaReadCompleted(_)
                        | CompletionStatus::Failed(_)
                ),
                "{s:?} should be terminal"
            );
        }
    }

    #[test]
    fn non_terminal_statuses_detected() {
        let non_terminal = [
            CompletionStatus::NotCompleted,
            CompletionStatus::InProgress("working".into()),
        ];
        for s in &non_terminal {
            assert!(
                !matches!(
                    s,
                    CompletionStatus::Completed
                        | CompletionStatus::DnaReadCompleted(_)
                        | CompletionStatus::Failed(_)
                ),
                "{s:?} should NOT be terminal"
            );
        }
    }

    #[test]
    fn dna_completed_preserves_info() {
        let info = DnaInfo {
            dna_value: "0x00641CC26AE96854".into(),
            dna_raw_value: "00110010".into(),
            device_type: "CH347".into(),
        };
        let status = CompletionStatus::DnaReadCompleted(info);
        if let CompletionStatus::DnaReadCompleted(inner) = status {
            assert_eq!(inner.dna_value, "0x00641CC26AE96854");
            assert_eq!(inner.device_type, "CH347");
        } else {
            panic!("Expected DnaReadCompleted");
        }
    }

    #[test]
    fn failed_preserves_message() {
        let status = CompletionStatus::Failed("cable disconnected".into());
        if let CompletionStatus::Failed(msg) = status {
            assert_eq!(msg, "cable disconnected");
        } else {
            panic!("Expected Failed");
        }
    }
}
