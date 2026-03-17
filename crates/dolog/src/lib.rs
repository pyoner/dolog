mod cli;
pub mod trigger;

pub use cli::{Cli, run};
pub use trigger::{AppError, TriggerManager};
