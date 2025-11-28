use super::DeveloperView;
use super::utils::{
    determine_next_instances, find_missing_layers, format_layers, parse_layer_input,
};
use crate::AppView;
use crate::common::{AssignmentInfo, DeviceProperties, ShardHealthResponse};
use crate::config::{Config, KVBits};
use crate::utils::ModelConfig;
use color_eyre::eyre::OptionExt;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum ManualAssignmentView {
    SelectingModel,
    FetchingShards(String /* model name */),
    AssigningLayers,
    Submitting,
    LoadingModel(String /* model name */),
    Success,
    Error(String),
}

#[derive(Default, Debug)]
pub struct ManualAssignmentState {
    model: String,
    num_layers: u32,
    shards: Vec<ShardInfo>,
    assignments: HashMap<String /* shard */, Vec<u32> /* layers */>,
    selected_shard: usize,
    is_typing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardInfo {
    #[serde(flatten)]
    pub device: DeviceProperties,
    pub model_loaded: bool,
    pub assigned_layers: Vec<u32>,
}

impl crate::App {
    pub fn draw_manual_assignment(&mut self, frame: &mut Frame, view: &ManualAssignmentView) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Manual Layer Assignment")
            .bold()
            .yellow()
            .centered();
        frame.render_widget(Paragraph::new(title), title_area);

        match view {
            ManualAssignmentView::SelectingModel => {
                self.draw_model_selection_for_manual(frame, content_area);
            }
            ManualAssignmentView::FetchingShards(_) => {
                frame.render_widget(
                    Paragraph::new("Fetching available shards...")
                        .block(Block::default().borders(Borders::ALL))
                        .centered(),
                    content_area,
                );
            }
            ManualAssignmentView::AssigningLayers => {
                self.draw_layer_assignment_interface(frame, content_area);
            }
            ManualAssignmentView::Submitting => {
                frame.render_widget(
                    Paragraph::new(format!(
                        "Submitting topology for {}...",
                        self.state.developer.manual.model
                    ))
                    .block(Block::default().borders(Borders::ALL))
                    .centered(),
                    content_area,
                );
            }
            ManualAssignmentView::LoadingModel(model) => {
                frame.render_widget(
                    Paragraph::new(format!(
                        "Loading model {}...\nThis may take a few moments.",
                        model
                    ))
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Cyan))
                    .centered(),
                    content_area,
                );
            }
            ManualAssignmentView::Success => {
                frame.render_widget(
                    Paragraph::new("Manual topology prepared and model loaded successfully!")
                        .block(Block::default().borders(Borders::ALL))
                        .style(Style::default().fg(Color::Green))
                        .centered(),
                    content_area,
                );
            }
            ManualAssignmentView::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::default().borders(Borders::ALL))
                        .style(Style::default().fg(Color::Red))
                        .wrap(Wrap { trim: true }),
                    content_area,
                );
            }
        }

        // Footer with context-specific help
        let footer_text = match view {
            ManualAssignmentView::SelectingModel => {
                "↑↓: Select model | Enter: Continue | Esc: Back"
            }
            ManualAssignmentView::AssigningLayers => {
                if self.state.developer.manual.is_typing {
                    "Type layers (e.g., 0,1,2 or 0-5) | Enter: Save | Esc: Cancel input"
                } else {
                    "↑↓: Select shard | Enter: Assign layers | C: Complete | Esc: Back"
                }
            }
            ManualAssignmentView::Success | ManualAssignmentView::Error(_) => {
                "Press Esc to go back"
            }
            ManualAssignmentView::LoadingModel(_) => "Loading model...",
            ManualAssignmentView::FetchingShards(_) => "Fetching shards...",
            ManualAssignmentView::Submitting => "Submitting topology...",
        };

        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::default().borders(Borders::TOP))
                .centered(),
            footer_area,
        );
    }

    fn draw_model_selection_for_manual(&mut self, frame: &mut Frame, area: Rect) {
        let model_names: Vec<String> = self
            .available_models
            .iter()
            .map(|model| model.id.clone())
            .collect();

        let selector = crate::wigets::ModelSelector::new(&model_names)
            .block(Block::bordered().title("Select a model"));

        frame.render_stateful_widget(selector, area, &mut self.model_selector_state);
    }

    fn draw_layer_assignment_interface(&mut self, frame: &mut Frame, area: Rect) {
        let state = &self.state.developer.manual;
        let chunks = Layout::vertical([
            Constraint::Length(3), // Model info
            Constraint::Min(10),   // Shard list
            Constraint::Length(5), // Assignment status
        ])
        .split(area);

        // Model info
        let model_info: Vec<Line<'_>> = vec![Line::from(format!(
            "Model: {} | Total Layers: {}",
            state.model, state.num_layers
        ))];
        frame.render_widget(
            Paragraph::new(model_info).block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        // Split shards into two groups
        let shard_chunks =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

        // Unassigned shards
        let mut unassigned_items = Vec::new();
        let mut assigned_items = Vec::new();

        for (i, shard) in state.shards.iter().enumerate() {
            let shard_layers = state
                .assignments
                .get(&shard.device.instance)
                .cloned()
                .unwrap_or_default();

            let display_text = if shard_layers.is_empty() {
                format!("{}", shard.device.instance)
            } else {
                format!(
                    "{}: {}",
                    shard.device.instance,
                    format_layers(&shard_layers)
                )
            };

            let is_selected = i == state.selected_shard;
            let style = if is_selected && state.is_typing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let item = if is_selected && state.is_typing {
                ListItem::new(format!("{} > {}", display_text, self.input_buffer)).style(style)
            } else {
                ListItem::new(display_text).style(style)
            };

            if shard_layers.is_empty() && !shard.model_loaded {
                unassigned_items.push(item);
            } else {
                assigned_items.push(item);
            }
        }

        let unassigned_list = List::new(unassigned_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Unassigned Shards"),
        );
        frame.render_widget(unassigned_list, shard_chunks[0]);

        let assigned_list = List::new(assigned_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Assigned Shards"),
        );
        frame.render_widget(assigned_list, shard_chunks[1]);

        // Assignment status
        let assigned_count: usize = state.assignments.values().map(|v| v.len()).sum();
        let all_layers: HashSet<u32> = state
            .assignments
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        let missing_layers = find_missing_layers(&all_layers, state.num_layers);

        let status_color = if missing_layers.is_empty() {
            Color::Green
        } else {
            Color::Yellow
        };

        let status_text = if missing_layers.is_empty() {
            format!(
                "All {} layers assigned! Press 'C' to complete.",
                state.num_layers
            )
        } else {
            format!(
                "Assigned: {}/{} | Missing: {}",
                assigned_count,
                state.num_layers,
                format_layers(&missing_layers)
            )
        };

        frame.render_widget(
            Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .style(Style::default().fg(status_color)),
            chunks[2],
        );
    }

    pub(super) fn handle_manual_assignment_input(
        &mut self,
        key: KeyEvent,
        view: &ManualAssignmentView,
    ) {
        match view {
            ManualAssignmentView::SelectingModel => match key.code {
                KeyCode::Esc => {
                    self.view = AppView::Developer(DeveloperView::Menu);
                }
                KeyCode::Up => {
                    self.model_selector_state.move_up();
                }
                KeyCode::Down => {
                    self.model_selector_state
                        .move_down(self.available_models.len());
                }
                KeyCode::Enter => {
                    let model = self.available_models[self.model_selector_state.selected()]
                        .id
                        .clone();
                    self.view = AppView::Developer(DeveloperView::ManualAssignment(
                        ManualAssignmentView::FetchingShards(model),
                    ));
                }
                _ => {}
            },
            ManualAssignmentView::AssigningLayers => {
                let state = &mut self.state.developer.manual;

                if state.is_typing {
                    // In input mode
                    match key.code {
                        KeyCode::Esc => {
                            state.is_typing = false;
                            self.input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            // Parse and save layers
                            if let Some(layers) =
                                parse_layer_input(&self.input_buffer, state.num_layers)
                            {
                                if state.selected_shard < state.shards.len() {
                                    state.assignments.insert(
                                        state.shards[state.selected_shard].device.instance.clone(),
                                        layers,
                                    );
                                }
                            }
                            state.is_typing = false;
                            self.input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            self.input_buffer.pop();
                        }
                        KeyCode::Char(c)
                            if c.is_ascii_digit() || c == ',' || c == '-' || c == ' ' =>
                        {
                            self.input_buffer.push(c);
                        }
                        _ => {}
                    }
                } else {
                    // Not in input mode
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Esc) => {
                            self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                ManualAssignmentView::SelectingModel,
                            ));
                            return;
                        }
                        (_, KeyCode::Up) => {
                            if state.selected_shard > 0 {
                                state.selected_shard -= 1;
                            }
                        }
                        (_, KeyCode::Down) => {
                            if !state.shards.is_empty()
                                && state.selected_shard < state.shards.len() - 1
                            {
                                state.selected_shard += 1;
                            }
                        }
                        (_, KeyCode::Enter) => {
                            state.is_typing = true;
                            self.input_buffer.clear();
                        }
                        (_, KeyCode::Char('c') | KeyCode::Char('C')) => {
                            // Complete assignment
                            let all_layers: HashSet<u32> = state
                                .assignments
                                .values()
                                .flat_map(|v| v.iter().cloned())
                                .collect();
                            let missing_layers = find_missing_layers(&all_layers, state.num_layers);

                            if missing_layers.is_empty() {
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::Submitting,
                                ));
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
            ManualAssignmentView::LoadingModel(_) => {
                // loading is in progress, just wait
            }
            ManualAssignmentView::Success | ManualAssignmentView::Error(_) => {
                if key.code == KeyCode::Esc {
                    self.view = AppView::Developer(DeveloperView::Menu);
                }
            }
            _ => {}
        }
    }

    pub async fn fetch_shards_with_model(&self) -> color_eyre::Result<Vec<ShardInfo>> {
        let devices = self.api.get_devices().await?;

        let mut shards = Vec::new();
        for device in devices.into_values() {
            if device.is_manager {
                continue; // skip the manager nodes (API)
            }

            // get shard health info
            let health_url = format!("http://{}:{}/health", device.local_ip, device.server_port);
            let (model_loaded, assigned_layers) =
                if let Ok(health_response) = reqwest::get(&health_url).await {
                    if let Ok(health) = health_response.json::<ShardHealthResponse>().await {
                        (health.model_loaded, health.assigned_layers)
                    } else {
                        (false, Vec::new())
                    }
                } else {
                    (false, Vec::new())
                };

            shards.push(ShardInfo {
                device,
                model_loaded,
                assigned_layers,
            });
        }

        Ok(shards)
    }

    async fn submit_manual_topology(
        &self,
        config: &Config,
        model: &str,
        shards: &[ShardInfo],
        assignments: &HashMap<String, Vec<u32>>,
    ) -> color_eyre::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct PrepareManualTopologyRequest {
            model: String,
            devices: Vec<DeviceProperties>,
            assignments: Vec<AssignmentInfo>,
            num_layers: u32,
            kv_bits: KVBits,
            seq_len: u32,
            max_batch_size: u8,
        }
        let num_layers = ModelConfig::get_model_config(model)
            .await?
            .num_layers()
            .ok_or_eyre("Could not determine number of layers")? as u32;

        // Determine next instances automatically
        let next_instances = determine_next_instances(assignments);

        // Build devices array
        let devices: Vec<DeviceProperties> = shards
            .iter()
            .filter(|s| assignments.contains_key(&s.device.instance))
            .map(|shard| shard.device.clone())
            .collect();

        // Build assignments array
        let assignment_infos: Vec<AssignmentInfo> = shards
            .iter()
            .filter_map(|shard| {
                assignments.get(&shard.device.instance).map(|layers| {
                    let next_instance = next_instances
                        .get(&shard.device.instance)
                        .and_then(|next_instance| {
                            shards
                                .iter()
                                .find(|s| s.device.instance == *next_instance)
                                .map(|s| s.device.instance.clone())
                        })
                        .unwrap_or_else(|| shard.device.instance.clone());

                    AssignmentInfo {
                        instance: shard.device.instance.clone(),
                        layers: vec![layers.clone()],
                        window_size: layers.len() as u32,
                        residency_size: layers.len() as u32, // FIXME: adjust this?
                        next_instance,
                    }
                })
            })
            .collect();

        let request = PrepareManualTopologyRequest {
            model: model.to_string(),
            devices,
            assignments: assignment_infos,
            num_layers,
            kv_bits: config.kv_bits,
            seq_len: config.seq_len,
            max_batch_size: config.max_batch_exp,
        };

        let url = format!("{}/v1/prepare_topology_manual", config.api_url());
        let client = reqwest::Client::new();
        let response = client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            color_eyre::eyre::bail!("Failed to submit topology: {}", response.text().await?)
        }
    }

    /// Handle async operations for manual assignment state (called during tick).
    pub(super) async fn tick_manual_assignment(&mut self, view: &ManualAssignmentView) {
        match view {
            ManualAssignmentView::FetchingShards(model) => {
                match self.fetch_shards_with_model().await {
                    Ok(shards) => {
                        match ModelConfig::get_model_config(model)
                            .await
                            .and_then(|config| {
                                config
                                    .num_layers()
                                    .ok_or_eyre("Could not determine number of layers from config")
                            }) {
                            Ok(num_layers) => {
                                self.state.developer.manual = ManualAssignmentState {
                                    model: model.clone(),
                                    num_layers: num_layers as u32,
                                    shards,
                                    assignments: HashMap::new(),
                                    selected_shard: 0,
                                    is_typing: false,
                                };
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::AssigningLayers,
                                ));
                            }
                            Err(err) => {
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::Error(format!("{:#?}", err)),
                                ));
                            }
                        }
                    }
                    Err(err) => {
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::Error(format!("{:#?}", err)),
                        ));
                    }
                }
            }
            ManualAssignmentView::Submitting => {
                let model = self.state.developer.manual.model.clone();
                match self
                    .submit_manual_topology(
                        &self.config,
                        &model,
                        &self.state.developer.manual.shards,
                        &self.state.developer.manual.assignments,
                    )
                    .await
                {
                    Ok(_) => {
                        // Topology prepared, now load the model
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::LoadingModel(model),
                        ));
                    }
                    Err(err) => {
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::Error(format!("{:#?}", err)),
                        ));
                    }
                }
            }
            ManualAssignmentView::LoadingModel(model) => {
                // Load the model using the existing LoadModelState functionality
                match self.api.load_model(&model).await {
                    Ok(_response) => {
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::Success,
                        ));
                        // Fetch topology after successful manual assignment
                        if let Ok(topology) = self.api.get_topology().await {
                            self.topology = topology;
                        }
                    }
                    Err(err) => {
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::Error(format!("Failed to load model: {}", err)),
                        ));
                    }
                }
            }
            _ => {
                // No async operations needed for other states
            }
        }
    }
}
