/// The top-level application module.
mod app;
pub use app::{App, AppView};

/// Views for each "screen".
pub mod views;
pub use views::*;

mod common;
mod config;
mod utils;
