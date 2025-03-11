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
            FlashingOption::CH347_35T => ("OpenOCD/openocd-347.exe", "OpenOCD/flash/xc7a35T.cfg"),
            FlashingOption::CH347_75T => ("OpenOCD/openocd-347.exe", "OpenOCD/flash/xc7a75T.cfg"),
            FlashingOption::CH347_100T => ("OpenOCD/openocd-347.exe", "OpenOCD/flash/xc7a100T.cfg"),
            FlashingOption::RS232_35T => ("OpenOCD/openocd.exe", "OpenOCD/flash/xc7a35T_rs232.cfg"),
            FlashingOption::RS232_75T => ("OpenOCD/openocd.exe", "OpenOCD/flash/xc7a75T_rs232.cfg"),
            FlashingOption::DnaCH347 => ("OpenOCD/openocd-347.exe", "OpenOCD/DNA/init_347.cfg"),
            FlashingOption::DnaRS232_35T => ("OpenOCD/openocd.exe", "OpenOCD/DNA/init_232_35t.cfg"),
            FlashingOption::DnaRS232_75T => ("OpenOCD/openocd.exe", "OpenOCD/DNA/init_232_75t.cfg"),
        }
    }

    pub fn get_display_name(&self) -> &'static str {
        match self {
            FlashingOption::CH347_35T => "CH347 - 35T",
            FlashingOption::CH347_75T => "CH347 - 75T",
            FlashingOption::CH347_100T => "CH347 - Stark100T",
            FlashingOption::RS232_35T => "RS232 - 35T",
            FlashingOption::RS232_75T => "RS232 - 75T",
            FlashingOption::DnaCH347 => "CH347 - 35T, 75T, 100T DNA Read",
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
            _ => "FTDI Driver",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionStatus {
    NotCompleted,
    Completed,
    Failed(String),
}
