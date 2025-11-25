use crate::common::TopologyInfo;
use crate::config::{Config, KVBits};
use crate::{App, AppView};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, List, ListItem, Paragraph},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardLoadStatus {
    /// Shard name
    pub instance: String,
    /// Whether loading succeeded
    pub success: bool,
    /// Layers successfully loaded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers_loaded: Option<Vec<u32>>,
    /// Status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadModelView {
    SelectingModel,
    PreparingTopology(String /* model name */),
    LoadingModel(String /* model name */),
    Error(String),
    Success(LoadModelResponse),
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

impl LoadModelView {
    /// Prepare topology by calling the API.
    pub async fn prepare_topology(
        config: &Config,
        model: &str,
    ) -> color_eyre::Result<TopologyInfo> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct PrepareTopologyRequest {
            pub model: String,
            kv_bits: KVBits,
            seq_len: u32,
            max_batch_exp: u8,
        }

        let url = format!("{}/v1/prepare_topology", config.api_url());
        let client = reqwest::Client::new();
        let request = PrepareTopologyRequest {
            model: model.to_string(),
            kv_bits: config.kv_bits,
            seq_len: config.seq_len,
            max_batch_exp: config.max_batch_exp,
        };

        let response = client.post(&url).json(&request).send().await?;
        let topology: TopologyInfo = response.json().await?;
        Ok(topology)
    }

    /// Load model by calling the API with just the model name
    pub async fn load_model(
        api_url: &str,
        model: Option<&str>,
    ) -> color_eyre::Result<LoadModelResponse> {
        let url = format!("{}/v1/load_model", api_url);
        let client = reqwest::Client::new();

        // Create request body - either empty {} or {"model": "model_name"}
        let body = if let Some(model_name) = model {
            serde_json::json!({"model": model_name})
        } else {
            serde_json::json!({})
        };

        let response = client.post(&url).json(&body).send().await?;

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

impl App {
    pub(super) fn draw_load_model(&mut self, frame: &mut Frame, view: &LoadModelView) {
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
        match view {
            LoadModelView::SelectingModel => {
                self.draw_model_selection(frame, content_area);
            }
            LoadModelView::PreparingTopology(model) => {
                frame.render_widget(
                    Paragraph::new(format!("Preparing topology for {}...", model))
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            LoadModelView::LoadingModel(model) => {
                frame.render_widget(
                    Paragraph::new(format!("Loading model {}...", model))
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            LoadModelView::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            LoadModelView::Success(response) => {
                self.draw_load_success(frame, content_area, response);
            }
        }

        // Footer
        let footer_text = match view {
            LoadModelView::SelectingModel => {
                "Use ↑↓ to select model  |  Enter to load  |  Esc to go back"
            }
            LoadModelView::Error(_) | LoadModelView::Success(_) => "Press Esc to go back",
            _ => "Loading...",
        };
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    fn draw_model_selection(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let model_items: Vec<ListItem> = self
            .available_models
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
                ListItem::new(format!("  {}", model.id)).style(style)
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
                Line::from(format!("  {} {}", status_icon, shard_status.instance)).fg(status_color),
            );

            if let Some(layers) = &shard_status.layers_loaded {
                // TODO: we do have a util for this?
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

    pub(super) fn handle_load_model_input(&mut self, key: KeyEvent, state: &LoadModelView) {
        match state {
            LoadModelView::SelectingModel => match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.view = AppView::Menu;
                    self.selected_model = 0;
                }
                (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                (_, KeyCode::Up) => self.model_up(),
                (_, KeyCode::Down) => self.model_down(),
                (_, KeyCode::Enter) => self.start_model_load(),
                _ => {}
            },
            LoadModelView::Error(_) | LoadModelView::Success(_) => {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        self.view = AppView::Menu;
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
        if self.selected_model < self.available_models.len() - 1 {
            self.selected_model += 1;
        }
    }

    fn start_model_load(&mut self) {
        let model = self.available_models[self.selected_model].id.clone();
        self.view = AppView::Model(super::ModelView::Load(LoadModelView::PreparingTopology(
            model,
        )));
    }

    /// Handle async operations for load model state (called during tick).
    pub(super) async fn tick_load_model(&mut self, state: &LoadModelView) {
        match state {
            LoadModelView::PreparingTopology(model) => {
                match LoadModelView::prepare_topology(&self.config, model).await {
                    Ok(topology) => {
                        // move to loading model state and trigger load
                        self.view = AppView::Model(super::ModelView::Load(
                            LoadModelView::LoadingModel(model.clone()),
                        ));
                        self.topology = Some(topology);

                        // load the model
                        match LoadModelView::load_model(&self.config.api_url(), Some(&model)).await
                        {
                            Ok(load_response) => {
                                self.view = AppView::Model(super::ModelView::Load(
                                    LoadModelView::Success(load_response),
                                ));
                            }
                            Err(err) => {
                                self.view = AppView::Model(super::ModelView::Load(
                                    LoadModelView::Error(err.to_string()),
                                ));
                            }
                        }
                    }
                    Err(err) => {
                        self.view = AppView::Model(super::ModelView::Load(LoadModelView::Error(
                            err.to_string(),
                        )));
                    }
                }
            }
            _ => {
                // No async operations needed for other states
            }
        }
    }
}
