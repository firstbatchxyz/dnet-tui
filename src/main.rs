/// The top-level application module.
mod app;
pub use app::{App, AppState};

/// Views for each "screen".
mod views;
use views::*;

mod common;
mod config;
mod constants;
mod utils;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new()?;
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
