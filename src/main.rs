mod app;
/// Configurations & settings.
mod config;
/// Menu interface.
mod menu;
/// Loading models & prepaing.
mod model;
mod settings;
mod shard;
mod topology;

use app::{App, AppState, LoadModelState, UnloadModelState};
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
            terminal.draw(|frame| self.draw(frame))?;

            // Check if we need to load topology
            if self.state.is_loading_topology() {
                self.state.load_topology(&self.config.api_url()).await;
            }

            // Check if we need to prepare topology for model loading
            if let AppState::LoadModel(LoadModelState::PreparingTopology(model)) =
                &self.state.clone()
            {
                match LoadModelState::prepare_topology(&self.config.api_url(), model).await {
                    Ok(_topology) => {
                        // Move to loading model state and trigger load
                        let model_name = model.clone();
                        self.state = AppState::LoadModel(LoadModelState::LoadingModel(model_name.clone()));

                        // Load the model - just pass the model name
                        match LoadModelState::load_model(&self.config.api_url(), Some(&model_name)).await {
                            Ok(response) => {
                                self.state = AppState::LoadModel(LoadModelState::Success(response));
                            }
                            Err(err) => {
                                self.state =
                                    AppState::LoadModel(LoadModelState::Error(err.to_string()));
                            }
                        }
                    }
                    Err(err) => {
                        self.state = AppState::LoadModel(LoadModelState::Error(err.to_string()));
                    }
                }
            }

            // Check if we need to unload model
            if matches!(
                &self.state,
                AppState::UnloadModel(UnloadModelState::Unloading)
            ) {
                match UnloadModelState::unload_model(&self.config.api_url()).await {
                    Ok(_) => {
                        self.state = AppState::UnloadModel(UnloadModelState::Success);
                    }
                    Err(err) => {
                        self.state =
                            AppState::UnloadModel(UnloadModelState::Error(err.to_string()));
                    }
                }
            }

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
            AppState::LoadModel(state) => self.draw_load_model(frame, &state),
            AppState::UnloadModel(state) => self.draw_unload_model(frame, &state),
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
        match &self.state.clone() {
            AppState::Menu => self.handle_menu_input(key),
            AppState::Settings => self.handle_settings_input(key),
            AppState::TopologyView(_) => self.handle_topology_input(key),
            AppState::ShardView(_) => self.handle_shard_interaction_input(key),
            AppState::LoadModel(state) => self.handle_load_model_input(key, state),
            AppState::UnloadModel(state) => self.handle_unload_model_input(key, state),
        }
    }
}
