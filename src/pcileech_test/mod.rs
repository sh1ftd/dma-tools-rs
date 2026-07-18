mod controller;
mod parser;
mod runner;

pub use controller::{PcileechTestController, PcileechTestSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PcileechTestState {
    #[default]
    Idle,
    Running,
    Success(String),
    Failed(String),
}
