mod app;
/// Chat interface.
mod chat;
/// Configurations & settings.
mod config;
/// Developer tools and manual assignment.
mod developer;
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

        // Check if a model is already loaded on startup
        self.model_loaded = chat::is_model_loaded(&self.config.api_url()).await;

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
                                self.model_loaded = true;  // Set model loaded flag
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
                        self.model_loaded = false;  // Clear model loaded flag
                    }
                    Err(err) => {
                        self.state =
                            AppState::UnloadModel(UnloadModelState::Error(err.to_string()));
                    }
                }
            }

            // Check if we need to fetch shards for manual assignment
            if let AppState::Developer(developer::DeveloperState::ManualAssignment(
                developer::ManualAssignmentState::FetchingShards(model)
            )) = &self.state.clone() {
                let model = model.clone();
                match developer::ManualAssignmentState::fetch_shards_with_model(&self.config.api_url(), &model).await {
                    Ok((shards, num_layers)) => {
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::AssigningLayers {
                                model,
                                num_layers,
                                shards,
                                assignments: std::collections::HashMap::new(),
                                selected_shard: 0,
                                input_mode: false,
                                input_buffer: String::new(),
                            },
                        ));
                    }
                    Err(err) => {
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::Error(err.to_string()),
                        ));
                    }
                }
            }

            // Check if we need to submit manual topology
            if let AppState::Developer(developer::DeveloperState::ManualAssignment(
                developer::ManualAssignmentState::Submitting { model, shards, assignments }
            )) = &self.state.clone() {
                let model_name = model.clone();
                match developer::ManualAssignmentState::submit_manual_topology(
                    &self.config.api_url(),
                    &model,
                    &shards,
                    &assignments,
                ).await {
                    Ok(_) => {
                        // Topology prepared, now load the model
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::LoadingModel(model_name),
                        ));
                    }
                    Err(err) => {
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::Error(err.to_string()),
                        ));
                    }
                }
            }

            // Check if we need to load model after manual topology
            if let AppState::Developer(developer::DeveloperState::ManualAssignment(
                developer::ManualAssignmentState::LoadingModel(model)
            )) = &self.state.clone() {
                // Load the model using the existing LoadModelState functionality
                match LoadModelState::load_model(&self.config.api_url(), Some(&model)).await {
                    Ok(_response) => {
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::Success,
                        ));
                        self.model_loaded = true;  // Set model loaded flag
                    }
                    Err(err) => {
                        self.state = AppState::Developer(developer::DeveloperState::ManualAssignment(
                            developer::ManualAssignmentState::Error(format!("Failed to load model: {}", err)),
                        ));
                    }
                }
            }

            // Handle pending chat message
            if let Some(_message) = self.pending_chat_message.take() {
                if let AppState::Chat(crate::chat::ChatState::Active { messages, max_tokens, .. }) = &self.state {
                    match crate::chat::ChatState::send_message(&self.config.api_url(), messages, *max_tokens).await {
                        Ok(rx) => {
                            self.chat_stream_rx = Some(rx);
                        }
                        Err(err) => {
                            self.state = AppState::Chat(crate::chat::ChatState::Error(err));
                        }
                    }
                }
            }

            // Process chat stream - but only if we're still in chat state
            if let Some(mut rx) = self.chat_stream_rx.take() {
                // Check if we're still in chat state
                if !matches!(self.state, AppState::Chat(_)) {
                    // We've exited chat, don't process the stream
                    self.chat_stream_rx = None;
                } else {
                    let mut should_clear_rx = false;
                    let mut new_error_state = None;

                    // Try to receive messages without blocking
                    while let Ok(chunk) = rx.try_recv() {
                        if let AppState::Chat(crate::chat::ChatState::Active {
                        messages,
                        input_buffer: _,
                        cursor_position: _,
                        is_generating,
                        current_response,
                        scroll_offset,
                        max_tokens: _,
                    }) = &mut self.state {
                        if chunk == "DONE" {
                            // Finalize the response
                            if !current_response.is_empty() {
                                messages.push_back(crate::chat::ChatMessage {
                                    role: "assistant".to_string(),
                                    content: current_response.clone(),
                                    timestamp: chrono::Local::now().format("%H:%M").to_string(),
                                });
                                current_response.clear();
                            }
                            *is_generating = false;
                            // Reset scroll to allow user to scroll freely after generation
                            *scroll_offset = 0;
                            should_clear_rx = true;
                            break;
                        } else if chunk.starts_with("ERROR:") {
                            new_error_state = Some(chunk);
                            should_clear_rx = true;
                            break;
                        } else {
                            // Append chunk to current response
                            current_response.push_str(&chunk);
                            // Auto-scroll during generation to follow the new content
                            // This ensures the user sees the latest tokens being generated
                            *scroll_offset = usize::MAX; // Will be clamped in draw_messages
                        }
                    }
                }

                    // Handle state changes after processing
                    if let Some(error) = new_error_state {
                        self.state = AppState::Chat(crate::chat::ChatState::Error(error));
                    } else if !should_clear_rx {
                        // Put the receiver back if we're not done
                        self.chat_stream_rx = Some(rx);
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
            AppState::Developer(state) => self.handle_developer_input(key, state),
            AppState::Chat(state) => self.handle_chat_input(key, state),
        }
    }
}
