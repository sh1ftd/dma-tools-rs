#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    FlashFirmware,
    ReadDNA,
    Drivers,
    TestPcileech,
}
