#[derive(Debug, Clone)]
pub enum PcileechTestState {
    Running,
    Success(String),
    Failed(String),
}
