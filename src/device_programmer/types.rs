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
    DnaCH347,
    DnaRS232_35T,
    DnaRS232_75T,
}

// Constants for OpenOCD executable paths
pub const OPENOCD_CH347_PATH: &str = "OpenOCD/openocd-347.exe";
pub const OPENOCD_RS232_PATH: &str = "OpenOCD/openocd.exe";

impl FlashingOption {
    pub fn is_dna_read(&self) -> bool {
        matches!(
            self,
            FlashingOption::DnaCH347 | FlashingOption::DnaRS232_35T | FlashingOption::DnaRS232_75T
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
        )
    }

    pub fn get_command_args(&self) -> (&'static str, &'static str) {
        match self {
            FlashingOption::CH347_35T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a35T.cfg"),
            FlashingOption::CH347_75T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a75T.cfg"),
            FlashingOption::CH347_100T => (OPENOCD_CH347_PATH, "OpenOCD/flash/xc7a100T.cfg"),
            FlashingOption::RS232_35T => (OPENOCD_RS232_PATH, "OpenOCD/flash/xc7a35T_rs232.cfg"),
            FlashingOption::RS232_75T => (OPENOCD_RS232_PATH, "OpenOCD/flash/xc7a75T_rs232.cfg"),
            FlashingOption::DnaCH347 => (OPENOCD_CH347_PATH, "OpenOCD/DNA/init_347.cfg"),
            FlashingOption::DnaRS232_35T => (OPENOCD_RS232_PATH, "OpenOCD/DNA/init_232_35t.cfg"),
            FlashingOption::DnaRS232_75T => (OPENOCD_RS232_PATH, "OpenOCD/DNA/init_232_75t.cfg"),
        }
    }

    pub fn get_display_name(&self) -> &'static str {
        match self {
            FlashingOption::CH347_35T => "CH347 - 35T",
            FlashingOption::CH347_75T => "CH347 - 75T",
            FlashingOption::CH347_100T => "CH347 - Stark100T",
            FlashingOption::DnaCH347 => "CH347 - 35T, 75T, 100T DNA Read",

            FlashingOption::RS232_35T => "RS232 - 35T",
            FlashingOption::RS232_75T => "RS232 - 75T",
            FlashingOption::DnaRS232_35T => "RS232 - 35T DNA Read",
            FlashingOption::DnaRS232_75T => "RS232 - 75T DNA Read",
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
            | FlashingOption::DnaRS232_35T
            | FlashingOption::DnaRS232_75T => "FTDI Driver",
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
