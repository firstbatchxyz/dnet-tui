mod app;
mod config;
mod menu;
mod settings;
mod shard;
mod topology;

use app::{App, AppState};
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

impl App {
    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;

        // Create a ticker for animation updates (60 FPS for smooth animation)
        let mut interval = tokio::time::interval(Duration::from_millis(16));

        while self.running {
            // Check if we need to load topology
            if self.state.is_loading_topology() {
                self.state.load_topology(&self.config.api_url()).await;
            }

            terminal.draw(|frame| self.draw(frame))?;

            // Handle events with timeout to allow animation updates
            tokio::select! {
                _ = interval.tick() => {
                    // Just trigger a redraw for animation
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
            AppState::TopologyView(state) => self.draw_topology(frame, &state),
            AppState::ShardView(device) => self.draw_shard_interaction(frame, &device),
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        let event = self.event_stream.next().fuse().await;
        match event {
            Some(Ok(evt)) => match evt {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match &self.state {
            AppState::Menu => self.handle_menu_input(key),
            AppState::Settings => self.handle_settings_input(key),
            AppState::TopologyView(_) => self.handle_topology_input(key),
            AppState::ShardView(_) => self.handle_shard_interaction_input(key),
        }
    }
}
