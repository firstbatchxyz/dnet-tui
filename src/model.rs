use crate::app::{App, AppState, LoadModelState};
use crate::topology::TopologyResponse;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, List, ListItem, Paragraph},
};
use serde::{Deserialize, Serialize};

/// Available models for loading
pub const AVAILABLE_MODELS: &[&str] = &[
    "Qwen/Qwen3-4B-MLX-4bit",
    "Qwen/Qwen3-4B-MLX-8bit",
    "Qwen/Qwen3-32B-MLX-bf16",
    "Qwen/Qwen3-32B-MLX-8bit",
    "Qwen/Qwen3-32B-MLX-6bit",
    "NousResearch/Hermes-4-70B",
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
    "Qwen/Qwen3-30B-A3B-MLX-8bit",
    "Qwen/Qwen3-30B-A3B-MLX-bf16",
    "Qwen/Qwen3-30B-A3B-MLX-6bit",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareTopologyRequest {
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardLoadStatus {
    /// Shard service name
    pub service_name: String,
    /// Whether loading succeeded
    pub success: bool,
    /// Layers successfully loaded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers_loaded: Option<Vec<u32>>,
    /// Status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadModelResponse {
    /// Model name
    pub model: String,
    /// Whether all shards loaded successfully
    pub success: bool,
    /// Status of each shard
    pub shard_statuses: Vec<ShardLoadStatus>,
    /// Overall status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl App {
    pub fn draw_load_model(&mut self, frame: &mut Frame, state: &LoadModelState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Load Model").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match state {
            LoadModelState::SelectingModel => {
                self.draw_model_selection(frame, content_area);
            }
            LoadModelState::PreparingTopology(model) => {
                frame.render_widget(
                    Paragraph::new(format!("Preparing topology for {}...", model))
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            LoadModelState::LoadingModel(model) => {
                frame.render_widget(
                    Paragraph::new(format!("Loading model {}...", model))
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            LoadModelState::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            LoadModelState::Success(response) => {
                self.draw_load_success(frame, content_area, response);
            }
        }

        // Footer
        let footer_text = match state {
            LoadModelState::SelectingModel => {
                "Use ↑↓ to select model  |  Enter to load  |  Esc to go back"
            }
            LoadModelState::Error(_) | LoadModelState::Success(_) => "Press Esc to go back",
            _ => "Loading...",
        };
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    fn draw_model_selection(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let model_items: Vec<ListItem> = AVAILABLE_MODELS
            .iter()
            .enumerate()
            .map(|(i, model)| {
                let style = if i == self.selected_model {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {}", model)).style(style)
            })
            .collect();

        let list = List::new(model_items).block(Block::bordered().title("Select a model"));

        frame.render_widget(list, area);
    }

    fn draw_load_success(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        response: &LoadModelResponse,
    ) {
        let mut lines = vec![
            Line::from(""),
            Line::from(format!("Model: {}", response.model))
                .bold()
                .green(),
            Line::from(""),
        ];

        // Overall status
        if response.success {
            lines.push(Line::from("Status: All shards loaded successfully!").green());
        } else {
            lines.push(Line::from("Status: Some shards failed to load").red());
        }

        if let Some(msg) = &response.message {
            lines.push(Line::from(format!("Message: {}", msg)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Shard Statuses:").bold());
        lines.push(Line::from(""));

        // List each shard status
        for shard_status in &response.shard_statuses {
            let status_icon = if shard_status.success { "✓" } else { "✗" };
            let status_color = if shard_status.success {
                Color::Green
            } else {
                Color::Red
            };

            lines.push(
                Line::from(format!("  {} {}", status_icon, shard_status.service_name))
                    .fg(status_color),
            );

            if let Some(layers) = &shard_status.layers_loaded {
                let layers_str = if layers.is_empty() {
                    "[]".to_string()
                } else {
                    format!(
                        "[{}..{}]",
                        layers.first().unwrap_or(&0),
                        layers.last().unwrap_or(&0)
                    )
                };
                lines.push(Line::from(format!("    Layers: {}", layers_str)).dim());
            }

            if let Some(msg) = &shard_status.message {
                lines.push(Line::from(format!("    {}", msg)).dim());
            }
        }

        let paragraph = Paragraph::new(lines).block(Block::bordered().title("Load Complete"));
        frame.render_widget(paragraph, area);
    }

    pub fn handle_load_model_input(&mut self, key: KeyEvent, state: &LoadModelState) {
        match state {
            LoadModelState::SelectingModel => match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.state = AppState::Menu;
                    self.selected_model = 0;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                (_, KeyCode::Up) => self.model_up(),
                (_, KeyCode::Down) => self.model_down(),
                (_, KeyCode::Enter) => self.start_model_load(),
                _ => {}
            },
            LoadModelState::Error(_) | LoadModelState::Success(_) => {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        self.state = AppState::Menu;
                        self.selected_model = 0;
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                    _ => {}
                }
            }
            _ => {
                // During loading states, only allow quitting
                if matches!(
                    (key.modifiers, key.code),
                    (
                        KeyModifiers::CONTROL,
                        KeyCode::Char('c') | KeyCode::Char('C')
                    )
                ) {
                    self.quit();
                }
            }
        }
    }

    fn model_up(&mut self) {
        if self.selected_model > 0 {
            self.selected_model -= 1;
        }
    }

    fn model_down(&mut self) {
        if self.selected_model < AVAILABLE_MODELS.len() - 1 {
            self.selected_model += 1;
        }
    }

    fn start_model_load(&mut self) {
        let model = AVAILABLE_MODELS[self.selected_model].to_string();
        self.state = AppState::LoadModel(LoadModelState::PreparingTopology(model));
    }
}

impl LoadModelState {
    /// Prepare topology by calling the API
    pub async fn prepare_topology(
        api_url: &str,
        model: &str,
    ) -> color_eyre::Result<TopologyResponse> {
        let url = format!("{}/v1/prepare_topology", api_url);
        let client = reqwest::Client::new();
        let request = PrepareTopologyRequest {
            model: model.to_string(),
        };

        let response = client.post(&url).json(&request).send().await?;
        let topology: TopologyResponse = response.json().await?;
        Ok(topology)
    }

    /// Load model by calling the API with the topology
    pub async fn load_model(
        api_url: &str,
        topology: &TopologyResponse,
    ) -> color_eyre::Result<LoadModelResponse> {
        let url = format!("{}/v1/load_model", api_url);
        let client = reqwest::Client::new();

        let response = client.post(&url).json(topology).send().await?;

        // Check if response is successful
        if response.status().is_success() {
            let load_response: LoadModelResponse = response.json().await?;
            Ok(load_response)
        } else {
            let error_text = response.text().await?;
            Err(color_eyre::eyre::eyre!(
                "Failed to load model: {}",
                error_text
            ))
        }
    }
}

impl crate::app::UnloadModelState {
    /// Unload model by calling the API
    pub async fn unload_model(api_url: &str) -> color_eyre::Result<()> {
        let url = format!("{}/v1/unload_model", api_url);
        let client = reqwest::Client::new();

        let response = client.post(&url).send().await?;

        // Check if response is successful
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(color_eyre::eyre::eyre!(
                "Failed to unload model: {}",
                error_text
            ))
        }
    }
}

impl App {
    pub fn draw_unload_model(&mut self, frame: &mut Frame, state: &crate::app::UnloadModelState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Unload Model").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match state {
            crate::app::UnloadModelState::Unloading => {
                frame.render_widget(
                    Paragraph::new("Unloading model...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            crate::app::UnloadModelState::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            crate::app::UnloadModelState::Success => {
                frame.render_widget(
                    Paragraph::new("Model unloaded successfully!")
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Green))
                        .centered(),
                    content_area,
                );
            }
        }

        // Footer
        let footer_text = match state {
            crate::app::UnloadModelState::Error(_) | crate::app::UnloadModelState::Success => {
                "Press Esc to go back"
            }
            _ => "Unloading...",
        };
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    pub fn handle_unload_model_input(
        &mut self,
        key: KeyEvent,
        state: &crate::app::UnloadModelState,
    ) {
        match state {
            crate::app::UnloadModelState::Error(_) | crate::app::UnloadModelState::Success => {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        self.state = AppState::Menu;
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                    _ => {}
                }
            }
            _ => {
                // During unloading, only allow quitting
                if matches!(
                    (key.modifiers, key.code),
                    (
                        KeyModifiers::CONTROL,
                        KeyCode::Char('c') | KeyCode::Char('C')
                    )
                ) {
                    self.quit();
                }
            }
        }
    }
}
