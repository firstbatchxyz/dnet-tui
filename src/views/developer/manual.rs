use super::DeveloperView;
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
    input_mode: bool,
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
                if self.state.developer.manual.input_mode {
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
            ManualAssignmentView::Submitting { .. } => "Submitting topology...",
        };

        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::default().borders(Borders::TOP))
                .centered(),
            footer_area,
        );
    }

    fn draw_model_selection_for_manual(&mut self, frame: &mut Frame, area: Rect) {
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

        let list = List::new(model_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select a model"),
        );

        frame.render_widget(list, area);
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
            let style = if is_selected && state.input_mode {
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

            let item = if is_selected && state.input_mode {
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
            ManualAssignmentView::SelectingModel => match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.view = AppView::Developer(DeveloperView::Menu);
                }
                (_, KeyCode::Up) => {
                    if self.selected_model > 0 {
                        self.selected_model -= 1;
                    }
                }
                (_, KeyCode::Down) => {
                    if self.selected_model < self.available_models.len() - 1 {
                        self.selected_model += 1;
                    }
                }
                (_, KeyCode::Enter) => {
                    let model = self.available_models[self.selected_model].id.clone();
                    self.view = AppView::Developer(DeveloperView::ManualAssignment(
                        ManualAssignmentView::FetchingShards(model),
                    ));
                }
                _ => {}
            },
            ManualAssignmentView::AssigningLayers => {
                let state = &mut self.state.developer.manual;

                if state.input_mode {
                    // In input mode
                    match key.code {
                        KeyCode::Esc => {
                            state.input_mode = false;
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
                            state.input_mode = false;
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
                            if state.selected_shard < state.shards.len() - 1 {
                                state.selected_shard += 1;
                            }
                        }
                        (_, KeyCode::Enter) => {
                            state.input_mode = true;
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

                // FIXME: do we need this?
                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                    ManualAssignmentView::AssigningLayers,
                ));
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
}

// Helper functions
fn format_layers(layers: &[u32]) -> String {
    if layers.is_empty() {
        return "[]".to_string();
    }

    let mut sorted = layers.to_vec();
    sorted.sort_unstable();

    let mut ranges = Vec::new();
    let mut start = sorted[0];
    let mut end = sorted[0];

    for &layer in &sorted[1..] {
        if layer == end + 1 {
            end = layer;
        } else {
            if start == end {
                ranges.push(start.to_string());
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = layer;
            end = layer;
        }
    }

    if start == end {
        ranges.push(start.to_string());
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    ranges.join(",")
}

fn parse_layer_input(input: &str, max_layers: u32) -> Option<Vec<u32>> {
    let mut layers = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(dash_pos) = part.find('-') {
            // Range
            let start_str = &part[..dash_pos].trim();
            let end_str = &part[dash_pos + 1..].trim();

            if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
                if start < max_layers && end < max_layers && start <= end {
                    layers.extend(start..=end);
                }
            }
        } else {
            // Single number
            if let Ok(layer) = part.parse::<u32>() {
                if layer < max_layers {
                    layers.push(layer);
                }
            }
        }
    }

    layers.sort_unstable();
    layers.dedup();

    if layers.is_empty() {
        None
    } else {
        Some(layers)
    }
}

fn find_missing_layers(assigned: &HashSet<u32>, total: u32) -> Vec<u32> {
    let mut missing = Vec::new();
    for i in 0..total {
        if !assigned.contains(&i) {
            missing.push(i);
        }
    }
    missing
}

fn determine_next_instances(assignments: &HashMap<String, Vec<u32>>) -> HashMap<String, String> {
    let mut next_instances = HashMap::new();

    // Create a map of first_layer -> shard
    let mut layer_to_shard: HashMap<u32, String> = HashMap::new();
    for (shard, layers) in assignments {
        if !layers.is_empty() {
            let min_layer = *layers.iter().min().unwrap();
            layer_to_shard.insert(min_layer, shard.clone());
        }
    }

    // For each shard, find its next shard
    for (shard, layers) in assignments {
        if !layers.is_empty() {
            let max_layer = *layers.iter().max().unwrap();

            // Find the shard that has max_layer + 1
            if let Some(next_shard) = layer_to_shard.get(&(max_layer + 1)) {
                next_instances.insert(shard.clone(), next_shard.clone());
            } else {
                // This is the last shard, connect back to the first
                if let Some(first_shard) = layer_to_shard.get(&0) {
                    next_instances.insert(shard.clone(), first_shard.clone());
                }
            }
        }
    }

    next_instances
}

// API functions
impl ManualAssignmentView {
    pub async fn submit_manual_topology(
        config: &Config,
        model: &str,
        shards: &[ShardInfo],
        assignments: &HashMap<String, Vec<u32>>,
    ) -> color_eyre::Result<()> {
        #[derive(Debug, Serialize, Deserialize)]
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
            let error_text = response.text().await?;
            Err(color_eyre::eyre::eyre!(
                "Failed to submit topology: {}",
                error_text
            ))
        }
    }
}

impl crate::App {
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
                                    input_mode: false,
                                };
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::AssigningLayers,
                                ));
                            }
                            Err(err) => {
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::Error(format!("{:#?}", err.to_string())),
                                ));
                            }
                        }
                    }
                    Err(err) => {
                        self.view = AppView::Developer(DeveloperView::ManualAssignment(
                            ManualAssignmentView::Error(format!("{:#?}", err.to_string())),
                        ));
                    }
                }
            }
            ManualAssignmentView::Submitting => {
                let model = self.state.developer.manual.model.clone();
                match ManualAssignmentView::submit_manual_topology(
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
                            ManualAssignmentView::Error(format!("{:#?}", err.to_string())),
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
                            self.topology = Some(topology);
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
