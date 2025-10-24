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

use color_eyre::Result;
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new()?.run(terminal).await;
    ratatui::restore();
    result
}

/// 60 FPS = 1000ms / 60 = 16.67ms per frame
const FPS_RATE: Duration = Duration::from_millis(1000 / 60);

impl App {
    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.is_running = true;

        // check if a model is already loaded on startup
        // TODO: this should check for topology instead, which may enable / disable view topology button too
        // TODO: maybe this should be done ONCE as the user opens the menu for the first time?
        self.is_model_loaded = chat::is_model_loaded(&self.config.api_url()).await;

        // create a ticker for animation updates
        let mut interval = tokio::time::interval(FPS_RATE);

        while self.is_running {
            // draw first (to disguise async stuff in ticks)
            terminal.draw(|frame| self.draw(frame))?;

            // process ticks
            match self.state.clone() {
                AppState::Topology(topology_state) => {
                    self.tick_topology(&topology_state).await;
                }
                AppState::Model(model_state) => {
                    self.tick_model(&model_state).await;
                }
                AppState::Developer(developer_state) => {
                    self.tick_developer(&developer_state).await;
                }
                AppState::Chat(chat_state) => {
                    self.tick_chat(&chat_state).await;
                }
                _ => {
                    // No async operations for Menu and Settings
                }
            }

            // handle events with timeout to allow animation updates
            tokio::select! {
                _ = interval.tick() => {
                    // will trigger a redraw for animation by looping
                    continue;
                }
                result = self.handle_crossterm_events() => {
                    result?;
                }
            }
        }
        Ok(())
    }

    /// Renders the user interface.
    fn draw(&mut self, frame: &mut Frame) {
        match self.state.clone() {
            AppState::Menu => self.draw_menu(frame),
            AppState::Settings => self.draw_settings(frame),
            AppState::Topology(state) => self.draw_topology(frame, &state),
            AppState::Model(state) => self.draw_model(frame, &state),
            AppState::Developer(state) => self.draw_developer(frame, &state),
            AppState::Chat(state) => self.draw_chat(frame, &state),
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        let event = self.event_stream.next().fuse().await;
        match event {
            Some(Ok(evt)) => match evt {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(_) => {} // TODO: do we want mouse events?
                Event::Resize(_, _) => {}
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match &self.state.clone() {
            AppState::Menu => self.handle_menu_input(key),
            AppState::Settings => self.handle_settings_input(key),
            AppState::Topology(state) => self.handle_topology_input(key, state),
            AppState::Model(state) => self.handle_model_input(key, state),
            AppState::Developer(state) => self.handle_developer_input(key, state),
            AppState::Chat(state) => self.handle_chat_input(key, state),
        }
    }
}
