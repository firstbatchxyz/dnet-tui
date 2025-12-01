/// The top-level application module.
mod app;
pub use app::{App, AppView};

/// Views for each "screen".
pub mod views;
pub use views::*;

/// Common utilities.
pub mod common;

mod config;
pub use config::Config;
mod utils;

/// Reusable widgets.
pub mod widgets;
pub use widgets::*;
