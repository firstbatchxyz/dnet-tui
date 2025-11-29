use super::DeveloperView;
use super::utils::{
    determine_next_instances, find_missing_layers, format_layers, parse_layer_input,
};
use crate::AppView;
use crate::common::{AssignmentInfo, DeviceProperties, ShardHealth};
use crate::config::{Config, KVBits};
use crate::utils::ModelConfig;
use color_eyre::eyre::OptionExt;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnSelection {
    Unassigned,
    Assigned,
}

#[derive(Debug)]
pub struct ManualAssignmentState {
    model: String,
    num_layers: u32,
    shards: Vec<ShardInfo>,
    assignments: HashMap<String /* shard */, Vec<u32> /* layers */>,
    selected_column: ColumnSelection,
    selected_unassigned_index: usize,
    selected_assigned_index: usize,
    is_typing: bool,
}

impl Default for ManualAssignmentState {
    fn default() -> Self {
        Self {
            model: String::new(),
            num_layers: 0,
            shards: Vec::new(),
            assignments: HashMap::new(),
            selected_column: ColumnSelection::Unassigned,
            selected_unassigned_index: 0,
            selected_assigned_index: 0,
            is_typing: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardInfo {
    #[serde(flatten)]
    pub device: DeviceProperties,
    pub model_loaded: bool,
    pub assigned_layers: Vec<u32>,
}

/// Helper function to create a centered rect for popup
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

/// Helper to partition shards into unassigned and assigned lists
fn partition_shards(state: &ManualAssignmentState) -> (Vec<(usize, &ShardInfo)>, Vec<(usize, &ShardInfo)>) {
    let mut unassigned = Vec::new();
    let mut assigned = Vec::new();

    for (i, shard) in state.shards.iter().enumerate() {
        let shard_layers = state
            .assignments
            .get(&shard.device.instance)
            .cloned()
            .unwrap_or_default();

        if shard_layers.is_empty() && !shard.model_loaded {
            unassigned.push((i, shard));
        } else {
            assigned.push((i, shard));
        }
    }

    (unassigned, assigned)
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
                    "←→: Switch column | ↑↓: Navigate | Enter: Assign/Submit | Ctrl+D: Deassign | Esc: Back"
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
            Paragraph::new(footer_text).centered().fg(Color::Gray),
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
            Constraint::Percentage(65), // Top: shard lists
            Constraint::Percentage(35), // Bottom: layer visualization + status
        ])
        .split(area);

        // Split top half into two columns for shards
        let shard_chunks =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

        // Get partitioned shard lists
        let (unassigned_shards, assigned_shards) = partition_shards(state);

        // Create list items for unassigned shards
        let unassigned_items: Vec<ListItem> = unassigned_shards
            .iter()
            .enumerate()
            .map(|(idx, (_, shard))| {
                let is_selected = state.selected_column == ColumnSelection::Unassigned
                    && idx == state.selected_unassigned_index;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(shard.device.instance.clone()).style(style)
            })
            .collect();

        // Create list items for assigned shards
        let assigned_items: Vec<ListItem> = assigned_shards
            .iter()
            .enumerate()
            .map(|(idx, (_, shard))| {
                let is_selected = state.selected_column == ColumnSelection::Assigned
                    && idx == state.selected_assigned_index;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let shard_layers = state
                    .assignments
                    .get(&shard.device.instance)
                    .cloned()
                    .unwrap_or_default();
                let display_text = format!(
                    "{}: {}",
                    shard.device.instance,
                    format_layers(&shard_layers)
                );
                ListItem::new(display_text).style(style)
            })
            .collect();

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

        // Determine which shard is selected for layer visualization
        // Only highlight if in assigned column
        let selected_shard_index = if state.selected_column == ColumnSelection::Assigned
            && state.selected_assigned_index < assigned_shards.len()
        {
            Some(assigned_shards[state.selected_assigned_index].0)
        } else {
            None
        };

        // Layer visualization and status
        self.draw_layer_visualization(frame, chunks[1], selected_shard_index);

        // Draw popup if typing
        if state.is_typing {
            self.draw_layer_input_popup(frame, area);
        }
    }

    /// Helper to get the currently selected shard based on column selection
    fn get_selected_shard_info(state: &ManualAssignmentState) -> (Option<usize>, Option<String>) {
        let (unassigned, assigned) = partition_shards(state);

        match state.selected_column {
            ColumnSelection::Unassigned => {
                unassigned.get(state.selected_unassigned_index)
                    .map(|(idx, shard)| (Some(*idx), Some(shard.device.instance.clone())))
                    .unwrap_or((None, None))
            }
            ColumnSelection::Assigned => {
                assigned.get(state.selected_assigned_index)
                    .map(|(idx, shard)| (Some(*idx), Some(shard.device.instance.clone())))
                    .unwrap_or((None, None))
            }
        }
    }

    fn draw_layer_input_popup(&self, frame: &mut Frame, area: Rect) {
        let state = &self.state.developer.manual;

        // Calculate remaining (unassigned) layers
        let all_assigned_layers: HashSet<u32> = state
            .assignments
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        let remaining_layers = find_missing_layers(&all_assigned_layers, state.num_layers);

        // Get the actual shard index based on current column
        let (_shard_index, shard_name) = Self::get_selected_shard_info(state);
        let shard_name = shard_name.as_deref().unwrap_or("Unknown");

        // Build popup content
        let mut content = vec![
            Line::from(vec![
                "Assigning layers to: ".into(),
                shard_name.bold().cyan(),
            ]),
            Line::from(""),
            Line::from(vec![
                "Input: ".into(),
                self.input_buffer.clone().yellow(),
            ]),
            Line::from(""),
            Line::from("Remaining layers:".bold()),
        ];

        if remaining_layers.is_empty() {
            content.push(Line::from("  All layers assigned!".green()));
        } else {
            content.push(Line::from(format!("  {}", format_layers(&remaining_layers))));
        }

        content.push(Line::from(""));
        content.push(Line::from("Examples: 0,1,2 or 0-5".dark_gray()));

        // Create popup area
        let popup_area = centered_rect(60, 40, area);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Render popup
        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Assign Layers ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(popup, popup_area);
    }

    fn draw_layer_visualization(
        &self,
        frame: &mut Frame,
        area: Rect,
        selected_shard_index: Option<usize>,
    ) {
        let state = &self.state.developer.manual;

        // Split area into layer viz and status message
        let chunks = Layout::vertical([
            Constraint::Min(3),     // Layer visualization
            Constraint::Length(3),  // Status message
        ])
        .split(area);

        // Collect all assigned layers
        let all_assigned_layers: HashSet<u32> = state
            .assignments
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();

        let missing_layers = find_missing_layers(&all_assigned_layers, state.num_layers);
        let all_assigned = missing_layers.is_empty();

        // Get layers for the selected shard (if any)
        let selected_shard_layers: HashSet<u32> = if let Some(idx) = selected_shard_index {
            if idx < state.shards.len() {
                state
                    .assignments
                    .get(&state.shards[idx].device.instance)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };

        // Apply colors using spans
        let mut spans = Vec::new();
        for layer in 0..state.num_layers {
            let (symbol, color) = if selected_shard_layers.contains(&layer) {
                ("■ ", Color::Cyan)
            } else if all_assigned_layers.contains(&layer) {
                ("■ ", Color::White)
            } else {
                ("□ ", Color::Gray)
            };
            spans.push(symbol.fg(color));
        }

        let layer_line = Line::from(spans);

        // Title with model info
        let title = format!(
            "Layer Assignments: {} | Total Layers: {}",
            state.model, state.num_layers
        );

        frame.render_widget(
            Paragraph::new(layer_line)
                .block(Block::default().borders(Borders::ALL).title(title))
                .wrap(Wrap { trim: false })
                .centered(),
            chunks[0],
        );

        // Status message area
        let status_widget = if !self.status_message.is_empty() {
            // Show error message
            Paragraph::new(self.status_message.clone())
                .red()
                .bold()
                .centered()
        } else if all_assigned {
            // Show completion message
            Paragraph::new("All layers assigned! Press Enter to continue.")
                .green()
                .bold()
                .centered()
        } else {
            // Show missing layers
            Paragraph::new(format!("Missing: {}", format_layers(&missing_layers)))
                .yellow()
                .centered()
        };

        frame.render_widget(
            status_widget.block(Block::default().borders(Borders::ALL).title("Status")),
            chunks[1],
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
                    self.model_selector_state
                        .move_up(self.available_models.len());
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
                // Get shard info before borrowing state mutably
                let shard_info = Self::get_selected_shard_info(&self.state.developer.manual);
                let state = &mut self.state.developer.manual;

                if state.is_typing {
                    // In input mode
                    match key.code {
                        KeyCode::Esc => {
                            state.is_typing = false;
                            self.input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            // Parse and save layers with collision detection
                            if let Some(layers) =
                                parse_layer_input(&self.input_buffer, state.num_layers)
                            {
                                if let (Some(_idx), Some(name)) = shard_info {
                                    // Check for collisions with other shards
                                    let has_collision = state
                                        .assignments
                                        .iter()
                                        .filter(|(instance, _)| **instance != name)
                                        .any(|(_, assigned_layers)| {
                                            layers.iter().any(|l| assigned_layers.contains(l))
                                        });

                                    if has_collision {
                                        // Collision detected - don't assign and show error in status
                                        self.status_message = "Error: Layer collision detected! Those layers are already assigned to another shard.".to_string();
                                    } else {
                                        // No collision - proceed with assignment
                                        state.assignments.insert(name, layers);
                                        self.status_message.clear();

                                        // Auto-switch to Assigned column if all shards are now assigned
                                        let (unassigned, assigned) = partition_shards(state);
                                        if unassigned.is_empty() && !assigned.is_empty() {
                                            state.selected_column = ColumnSelection::Assigned;
                                            state.selected_assigned_index = 0;
                                        }
                                    }
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
                    let (unassigned, assigned) = partition_shards(state);
                    let unassigned_count = unassigned.len();
                    let assigned_count = assigned.len();

                    match (key.modifiers, key.code) {
                        (_, KeyCode::Esc) => {
                            self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                ManualAssignmentView::SelectingModel,
                            ));
                            return;
                        }
                        (_, KeyCode::Left) => {
                            // Move to unassigned column
                            if unassigned_count > 0 {
                                state.selected_column = ColumnSelection::Unassigned;
                                // Clamp selection to valid range
                                if state.selected_unassigned_index >= unassigned_count {
                                    state.selected_unassigned_index = unassigned_count.saturating_sub(1);
                                }
                            }
                        }
                        (_, KeyCode::Right) => {
                            // Move to assigned column
                            if assigned_count > 0 {
                                state.selected_column = ColumnSelection::Assigned;
                                // Clamp selection to valid range
                                if state.selected_assigned_index >= assigned_count {
                                    state.selected_assigned_index = assigned_count.saturating_sub(1);
                                }
                            }
                        }
                        (_, KeyCode::Up) => {
                            // Navigate within current column
                            match state.selected_column {
                                ColumnSelection::Unassigned => {
                                    if state.selected_unassigned_index > 0 {
                                        state.selected_unassigned_index -= 1;
                                    }
                                }
                                ColumnSelection::Assigned => {
                                    if state.selected_assigned_index > 0 {
                                        state.selected_assigned_index -= 1;
                                    }
                                }
                            }
                        }
                        (_, KeyCode::Down) => {
                            // Navigate within current column
                            match state.selected_column {
                                ColumnSelection::Unassigned => {
                                    if unassigned_count > 0 && state.selected_unassigned_index < unassigned_count - 1 {
                                        state.selected_unassigned_index += 1;
                                    }
                                }
                                ColumnSelection::Assigned => {
                                    if assigned_count > 0 && state.selected_assigned_index < assigned_count - 1 {
                                        state.selected_assigned_index += 1;
                                    }
                                }
                            }
                        }
                        (_, KeyCode::Enter) => {
                            // Check if all layers are assigned
                            let all_assigned_layers: HashSet<u32> = state
                                .assignments
                                .values()
                                .flat_map(|v| v.iter().cloned())
                                .collect();
                            let missing_layers = find_missing_layers(&all_assigned_layers, state.num_layers);

                            if missing_layers.is_empty() {
                                // All layers assigned - submit!
                                self.view = AppView::Developer(DeveloperView::ManualAssignment(
                                    ManualAssignmentView::Submitting,
                                ));
                                return;
                            } else {
                                // Not all assigned - enter typing mode
                                state.is_typing = true;
                                self.input_buffer.clear();
                                self.status_message.clear();
                            }
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                            // Deassign layers from the selected shard
                            if let (_, Some(name)) = shard_info {
                                state.assignments.remove(&name);

                                // Auto-switch to Unassigned column if no more assigned shards
                                let (unassigned, assigned) = partition_shards(state);
                                if assigned.is_empty() && !unassigned.is_empty() {
                                    state.selected_column = ColumnSelection::Unassigned;
                                    state.selected_unassigned_index = 0;
                                } else if state.selected_column == ColumnSelection::Assigned {
                                    // Clamp selection if we removed the last item in assigned list
                                    if state.selected_assigned_index >= assigned.len() && !assigned.is_empty() {
                                        state.selected_assigned_index = assigned.len() - 1;
                                    }
                                }
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
                    if let Ok(health) = health_response.json::<ShardHealth>().await {
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
                                    selected_column: ColumnSelection::Unassigned,
                                    selected_unassigned_index: 0,
                                    selected_assigned_index: 0,
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
