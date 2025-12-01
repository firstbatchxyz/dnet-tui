use super::ModelView;
use crate::common::LoadModelResponse;
use crate::{App, AppView};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub enum LoadModelView {
    SelectingModel,
    PreparingTopology(String /* model name */),
    LoadingModel(String /* model name */),
    Error(String),
    Success(LoadModelResponse),
}

impl App {
    pub(super) fn draw_load_model(&mut self, frame: &mut Frame, view: &LoadModelView) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
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
        frame.render_widget(Paragraph::new(footer_text).centered().gray(), footer_area);
    }

    fn draw_model_selection(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let model_names: Vec<String> = self
            .available_models
            .iter()
            .map(|model| model.id.clone())
            .collect();

        let selector = crate::widgets::ModelSelector::new(&model_names)
            .block(Block::bordered().title("Select a model"));

        frame.render_stateful_widget(selector, area, &mut self.model_selector_state);
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
                (_, KeyCode::Esc) => self.view = AppView::Menu,
                (_, KeyCode::Up) => self.model_up(),
                (_, KeyCode::Down) => self.model_down(),
                (_, KeyCode::Enter) => self.start_model_load(),
                _ => {}
            },
            LoadModelView::Error(_) | LoadModelView::Success(_) => {
                // only allow escape
                if key.code == KeyCode::Esc {
                    self.view = AppView::Menu;
                }
            }
            _ => {}
        }
    }

    fn model_up(&mut self) {
        self.model_selector_state
            .move_up(self.available_models.len());
    }

    fn model_down(&mut self) {
        self.model_selector_state
            .move_down(self.available_models.len());
    }

    fn start_model_load(&mut self) {
        let model = self.available_models[self.model_selector_state.selected()]
            .id
            .clone();
        self.view = AppView::Model(ModelView::Load(LoadModelView::PreparingTopology(model)));
    }

    /// Handle async operations for load model state (called during tick).
    pub(super) async fn tick_load_model(&mut self, state: &LoadModelView) {
        match state {
            LoadModelView::PreparingTopology(model) => {
                match self.api.prepare_topology(&self.config, model).await {
                    Ok(topology) => {
                        // move to loading model state and trigger load
                        self.view = AppView::Model(ModelView::Load(LoadModelView::LoadingModel(
                            model.clone(),
                        )));
                        self.topology = Some(topology);

                        // load the model
                        match self.api.load_model(model).await {
                            Ok(load_response) => {
                                self.view = AppView::Model(ModelView::Load(
                                    LoadModelView::Success(load_response),
                                ));
                            }
                            Err(err) => {
                                self.view = AppView::Model(ModelView::Load(LoadModelView::Error(
                                    err.to_string(),
                                )));
                            }
                        }
                    }
                    Err(err) => {
                        self.view =
                            AppView::Model(ModelView::Load(LoadModelView::Error(err.to_string())));
                    }
                }
            }
            _ => {
                // No async operations needed for other states
            }
        }
    }
}
