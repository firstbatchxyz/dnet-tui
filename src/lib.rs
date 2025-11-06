/// The top-level application module.
mod app;
pub use app::{App, AppState};

/// Views for each "screen".
pub mod views;
pub use views::*;

mod common;
mod config;
mod constants;
mod utils;
