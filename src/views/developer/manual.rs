use super::DeveloperState;
use crate::AppState;
use crate::common::{DeviceProperties, ShardHealthResponse};
use crate::constants::AVAILABLE_MODELS;
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
pub enum ManualAssignmentState {
    SelectingModel,
    FetchingShards(String /* model name */),
    AssigningLayers {
        model: String,
        num_layers: u32,
        shards: Vec<ShardInfo>,
        assignments: HashMap<String, Vec<u32>>, // shard instance -> layers
        selected_shard: usize,
        input_mode: bool,
        input_buffer: String,
    },
    Submitting {
        model: String,
        shards: Vec<ShardInfo>,
        assignments: HashMap<String, Vec<u32>>,
    },
    LoadingModel(String), // model name - new state for loading after topology
    Success,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardInfo {
    pub instance: String,
    pub local_ip: String,
    pub server_port: u16,
    pub shard_port: u16,
    pub model_loaded: bool,
    pub assigned_layers: Vec<u32>,
}

// FIXME: kinda same as `TopologyInfo`
#[derive(Debug, Serialize)]
struct ManualTopologyRequest {
    model: String,
    devices: Vec<DeviceProperties>,
    assignments: Vec<AssignmentInfo>,
    num_layers: u32,
}

#[derive(Debug, Serialize)]
struct AssignmentInfo {
    // FIXME: this should be named "instance"
    service: String,
    layers: Vec<Vec<u32>>,
    window_size: u32,
    // FIXME: this should be named "next_instance"
    next_service: String,
}

impl crate::App {
    pub fn draw_manual_assignment(&mut self, frame: &mut Frame, state: &ManualAssignmentState) {
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

        match state {
            ManualAssignmentState::SelectingModel => {
                self.draw_model_selection_for_manual(frame, content_area);
            }
            ManualAssignmentState::FetchingShards(_) => {
                frame.render_widget(
                    Paragraph::new("Fetching available shards...")
                        .block(Block::default().borders(Borders::ALL))
                        .centered(),
                    content_area,
                );
            }
            ManualAssignmentState::AssigningLayers {
                model,
                num_layers,
                shards,
                assignments,
                selected_shard,
                input_mode,
                input_buffer,
            } => {
                self.draw_layer_assignment_interface(
                    frame,
                    content_area,
                    model,
                    *num_layers,
                    shards,
                    assignments,
                    *selected_shard,
                    *input_mode,
                    input_buffer,
                );
            }
            ManualAssignmentState::Submitting { model, .. } => {
                frame.render_widget(
                    Paragraph::new(format!("Submitting topology for {}...", model))
                        .block(Block::default().borders(Borders::ALL))
                        .centered(),
                    content_area,
                );
            }
            ManualAssignmentState::LoadingModel(model) => {
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
            ManualAssignmentState::Success => {
                frame.render_widget(
                    Paragraph::new("Manual topology prepared and model loaded successfully!")
                        .block(Block::default().borders(Borders::ALL))
                        .style(Style::default().fg(Color::Green))
                        .centered(),
                    content_area,
                );
            }
            ManualAssignmentState::Error(err) => {
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
        let footer_text = match state {
            ManualAssignmentState::SelectingModel => {
                "↑↓: Select model | Enter: Continue | Esc: Back"
            }
            ManualAssignmentState::AssigningLayers { input_mode, .. } => {
                if *input_mode {
                    "Type layers (e.g., 0,1,2 or 0-5) | Enter: Save | Esc: Cancel input"
                } else {
                    "↑↓: Select shard | Enter: Assign layers | C: Complete | Esc: Back"
                }
            }
            ManualAssignmentState::Success | ManualAssignmentState::Error(_) => {
                "Press Esc to go back"
            }
            ManualAssignmentState::LoadingModel(_) => "Loading model...",
            ManualAssignmentState::FetchingShards(_) => "Fetching shards...",
            ManualAssignmentState::Submitting { .. } => "Submitting topology...",
        };

        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::default().borders(Borders::TOP))
                .centered(),
            footer_area,
        );
    }

    fn draw_model_selection_for_manual(&mut self, frame: &mut Frame, area: Rect) {
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

        let list = List::new(model_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select a model"),
        );

        frame.render_widget(list, area);
    }

    fn draw_layer_assignment_interface(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        model: &str,
        num_layers: u32,
        shards: &[ShardInfo],
        assignments: &HashMap<String, Vec<u32>>,
        selected_shard: usize,
        input_mode: bool,
        input_buffer: &str,
    ) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Model info
            Constraint::Min(10),   // Shard list
            Constraint::Length(5), // Assignment status
        ])
        .split(area);

        // Model info
        let model_info = vec![Line::from(format!(
            "Model: {} | Total Layers: {}",
            model, num_layers
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

        for (i, shard) in shards.iter().enumerate() {
            let shard_layers = assignments
                .get(&shard.instance)
                .cloned()
                .unwrap_or_default();

            let display_text = if shard_layers.is_empty() {
                format!("{}", shard.instance)
            } else {
                format!("{}: {}", shard.instance, format_layers(&shard_layers))
            };

            let is_selected = i == selected_shard;
            let style = if is_selected && input_mode {
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

            let item = if is_selected && input_mode {
                ListItem::new(format!("{} > {}", display_text, input_buffer)).style(style)
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
        let assigned_count: usize = assignments.values().map(|v| v.len()).sum();
        let all_layers: HashSet<u32> = assignments
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        let missing_layers = find_missing_layers(&all_layers, num_layers);

        let status_color = if missing_layers.is_empty() {
            Color::Green
        } else {
            Color::Yellow
        };

        let status_text = if missing_layers.is_empty() {
            format!("All {} layers assigned! Press 'C' to complete.", num_layers)
        } else {
            format!(
                "Assigned: {}/{} | Missing: {}",
                assigned_count,
                num_layers,
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
        state: &ManualAssignmentState,
    ) {
        match state {
            ManualAssignmentState::SelectingModel => match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.state = AppState::Developer(DeveloperState::Menu);
                }
                (_, KeyCode::Up) => {
                    if self.selected_model > 0 {
                        self.selected_model -= 1;
                    }
                }
                (_, KeyCode::Down) => {
                    if self.selected_model < AVAILABLE_MODELS.len() - 1 {
                        self.selected_model += 1;
                    }
                }
                (_, KeyCode::Enter) => {
                    let model = AVAILABLE_MODELS[self.selected_model].to_string();
                    self.state = AppState::Developer(DeveloperState::ManualAssignment(
                        ManualAssignmentState::FetchingShards(model),
                    ));
                }
                _ => {}
            },
            ManualAssignmentState::AssigningLayers {
                model,
                num_layers,
                shards,
                assignments,
                selected_shard,
                input_mode,
                input_buffer,
            } => {
                let model = model.clone();
                let num_layers = *num_layers;
                let shards = shards.clone();
                let mut assignments = assignments.clone();
                let mut selected_shard = *selected_shard;
                let mut input_mode = *input_mode;
                let mut input_buffer = input_buffer.clone();

                if input_mode {
                    // In input mode
                    match key.code {
                        KeyCode::Esc => {
                            input_mode = false;
                            input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            // Parse and save layers
                            if let Some(layers) = parse_layer_input(&input_buffer, num_layers) {
                                if selected_shard < shards.len() {
                                    assignments
                                        .insert(shards[selected_shard].instance.clone(), layers);
                                }
                            }
                            input_mode = false;
                            input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Char(c)
                            if c.is_ascii_digit() || c == ',' || c == '-' || c == ' ' =>
                        {
                            input_buffer.push(c);
                        }
                        _ => {}
                    }
                } else {
                    // Not in input mode
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Esc) => {
                            self.state = AppState::Developer(DeveloperState::ManualAssignment(
                                ManualAssignmentState::SelectingModel,
                            ));
                            return;
                        }
                        (_, KeyCode::Up) => {
                            if selected_shard > 0 {
                                selected_shard -= 1;
                            }
                        }
                        (_, KeyCode::Down) => {
                            if selected_shard < shards.len() - 1 {
                                selected_shard += 1;
                            }
                        }
                        (_, KeyCode::Enter) => {
                            input_mode = true;
                            input_buffer.clear();
                        }
                        (_, KeyCode::Char('c') | KeyCode::Char('C')) => {
                            // Complete assignment
                            let all_layers: HashSet<u32> = assignments
                                .values()
                                .flat_map(|v| v.iter().cloned())
                                .collect();
                            let missing_layers = find_missing_layers(&all_layers, num_layers);

                            if missing_layers.is_empty() {
                                self.state = AppState::Developer(DeveloperState::ManualAssignment(
                                    ManualAssignmentState::Submitting {
                                        model: model.clone(),
                                        shards: shards.clone(),
                                        assignments: assignments.clone(),
                                    },
                                ));
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                self.state = AppState::Developer(DeveloperState::ManualAssignment(
                    ManualAssignmentState::AssigningLayers {
                        model,
                        num_layers,
                        shards,
                        assignments,
                        selected_shard,
                        input_mode,
                        input_buffer,
                    },
                ));
            }
            ManualAssignmentState::LoadingModel(_) => {
                // Loading is in progress, just wait
            }
            ManualAssignmentState::Success | ManualAssignmentState::Error(_) => {
                if key.code == KeyCode::Esc {
                    self.state = AppState::Developer(DeveloperState::Menu);
                }
            }
            _ => {}
        }
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

fn determine_next_services(assignments: &HashMap<String, Vec<u32>>) -> HashMap<String, String> {
    let mut next_services = HashMap::new();

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
                next_services.insert(shard.clone(), next_shard.clone());
            } else {
                // This is the last shard, connect back to the first
                if let Some(first_shard) = layer_to_shard.get(&0) {
                    next_services.insert(shard.clone(), first_shard.clone());
                }
            }
        }
    }

    next_services
}

// API functions
impl ManualAssignmentState {
    pub async fn fetch_shards_with_model(
        api_url: &str,
        model: &str,
    ) -> color_eyre::Result<(Vec<ShardInfo>, u32)> {
        // Get model info to know number of layers
        let num_layers = get_model_layers(model);

        // Get devices from the /v1/devices endpoint
        let devices_url = format!("{}/v1/devices", api_url);
        let response = reqwest::get(&devices_url).await?;
        let devices_response: Vec<DeviceProperties> = response.json().await?;

        let mut shards = Vec::new();
        for device in devices_response {
            // Skip the API manager node
            if device.is_manager {
                continue;
            }

            let instance = device.instance.clone();
            let local_ip = device.local_ip.clone();
            let server_port = device.server_port;
            let shard_port = device.shard_port;

            // Get shard health info
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
                instance,
                local_ip,
                server_port,
                shard_port,
                model_loaded,
                assigned_layers,
            });
        }

        Ok((shards, num_layers))
    }

    pub async fn submit_manual_topology(
        api_url: &str,
        model: &str,
        shards: &[ShardInfo],
        assignments: &HashMap<String, Vec<u32>>,
    ) -> color_eyre::Result<()> {
        let num_layers = get_model_layers(model);

        // Determine next services automatically
        let next_services = determine_next_services(assignments);

        // Build devices array
        let devices: Vec<DeviceProperties> = shards
            .iter()
            .filter(|s| assignments.contains_key(&s.instance))
            .map(|shard| DeviceProperties {
                instance: shard.instance.clone(),
                local_ip: shard.local_ip.clone(),
                server_port: shard.server_port,
                shard_port: shard.shard_port,
                is_busy: false,
                is_manager: false,
                thunderbolt: None,
            })
            .collect();

        // Build assignments array
        let assignment_infos: Vec<AssignmentInfo> = shards
            .iter()
            .filter_map(|shard| {
                assignments.get(&shard.instance).map(|layers| {
                    let next_service = next_services
                        .get(&shard.instance)
                        .and_then(|next_instance| {
                            shards
                                .iter()
                                .find(|s| s.instance == *next_instance)
                                .map(|s| s.instance.clone())
                        })
                        .unwrap_or_else(|| shard.instance.clone());

                    AssignmentInfo {
                        service: shard.instance.clone(),
                        layers: vec![layers.clone()],
                        window_size: layers.len() as u32,
                        next_service: next_service,
                    }
                })
            })
            .collect();

        let request = ManualTopologyRequest {
            model: model.to_string(),
            devices,
            assignments: assignment_infos,
            num_layers,
        };

        let url = format!("{}/v1/prepare_topology_manual", api_url);
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

// Helper to get number of layers for a model
fn get_model_layers(model: &str) -> u32 {
    // This should match the actual model layer counts
    // You might want to get this from an API endpoint
    match model {
        "Qwen/Qwen3-4B-MLX-4bit" | "Qwen/Qwen3-4B-MLX-8bit" => 36,
        "Qwen/Qwen3-30B-A3B-MLX-8bit"
        | "Qwen/Qwen3-30B-A3B-MLX-bf16"
        | "Qwen/Qwen3-30B-A3B-MLX-6bit" => 30,
        "Qwen/Qwen3-32B-MLX-bf16" | "Qwen/Qwen3-32B-MLX-8bit" | "Qwen/Qwen3-32B-MLX-6bit" => 32,
        "NousResearch/Hermes-4-70B" => 70,
        "openai/gpt-oss-120b" => 120,
        "openai/gpt-oss-20b" => 20,
        _ => 36, // Default
    }
}

impl crate::App {
    /// Handle async operations for manual assignment state (called during tick).
    pub(super) async fn tick_manual_assignment(&mut self, state: &ManualAssignmentState) {
        match state {
            ManualAssignmentState::FetchingShards(model) => {
                let model = model.clone();
                match ManualAssignmentState::fetch_shards_with_model(&self.config.api_url(), &model)
                    .await
                {
                    Ok((shards, num_layers)) => {
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::AssigningLayers {
                                model,
                                num_layers,
                                shards,
                                assignments: HashMap::new(),
                                selected_shard: 0,
                                input_mode: false,
                                input_buffer: String::new(),
                            },
                        ));
                    }
                    Err(err) => {
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::Error(err.to_string()),
                        ));
                    }
                }
            }
            ManualAssignmentState::Submitting {
                model,
                shards,
                assignments,
            } => {
                let model_name = model.clone();
                match ManualAssignmentState::submit_manual_topology(
                    &self.config.api_url(),
                    &model,
                    &shards,
                    &assignments,
                )
                .await
                {
                    Ok(_) => {
                        // Topology prepared, now load the model
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::LoadingModel(model_name),
                        ));
                    }
                    Err(err) => {
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::Error(err.to_string()),
                        ));
                    }
                }
            }
            ManualAssignmentState::LoadingModel(model) => {
                // Load the model using the existing LoadModelState functionality
                match crate::model::LoadModelState::load_model(&self.config.api_url(), Some(&model))
                    .await
                {
                    Ok(_response) => {
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::Success,
                        ));
                        self.is_model_loaded = true; // Set model loaded flag
                    }
                    Err(err) => {
                        self.state = AppState::Developer(DeveloperState::ManualAssignment(
                            ManualAssignmentState::Error(format!("Failed to load model: {}", err)),
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
