use crate::app::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, Paragraph,
        canvas::{Canvas, Circle, Line as CanvasLine, Points},
    },
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyState {
    Loading,
    Loaded(TopologyResponse),
    Error(String),
}

impl TopologyState {
    /// Fetch topology from the API
    pub async fn fetch(api_url: &str) -> color_eyre::Result<TopologyResponse> {
        let url = format!("{}/v1/topology", api_url);
        let response = reqwest::get(&url).await?;

        // Get the response text first, regardless of status
        let status = response.status();
        let text = response.text().await?;

        // Check if the response contains an error detail message (for any status code)
        if let Ok(error_response) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(detail) = error_response.get("detail").and_then(|d| d.as_str()) {
                if detail.contains("No topology configured") || detail.contains("prepare_topology") {
                    return Err(color_eyre::eyre::eyre!("No topology configured"));
                }
                // For other detail messages, include them in the error
                if !status.is_success() {
                    return Err(color_eyre::eyre::eyre!("{}", detail));
                }
            }
        }

        // If we couldn't parse a detail message and status is not success, return generic error
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(color_eyre::eyre::eyre!("No topology found - model not loaded"));
            }
            return Err(color_eyre::eyre::eyre!(
                "API returned error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown error")
            ));
        }

        // Try to parse as successful topology response
        let topology: TopologyResponse = serde_json::from_str(&text)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse topology response: {}", e))?;
        Ok(topology)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub model: String,
    pub num_layers: u32,
    pub devices: Vec<Device>,
    pub assignments: Vec<Assignment>,
    pub solution: Solution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub is_manager: bool,
    pub is_busy: bool,
    pub instance: String,
    pub host: String,
    pub server_port: u16,
    pub shard_port: u16,
    pub local_ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thunderbolt: Option<ThunderboltInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThunderboltInfo {
    pub ip_addr: String,
    // Using serde_json::Value to handle the complex nested structure
    pub instances: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub service: String,
    pub layers: Vec<Vec<u32>>,
    pub next_service: String,
    pub window_size: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Solution {
    Manual {
        source: String
    },
    Optimized {
        #[serde(default)]
        w: Vec<u32>,
        #[serde(default)]
        n: Vec<u32>,
        #[serde(default)]
        k: u32,
        #[serde(default)]
        obj_value: f64,
        #[serde(default)]
        sets: SolutionSets,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SolutionSets {
    #[serde(rename = "M1", default)]
    pub m1: Vec<u32>,
    #[serde(rename = "M2", default)]
    pub m2: Vec<u32>,
    #[serde(rename = "M3", default)]
    pub m3: Vec<u32>,
}

impl TopologyResponse {
    /// Get device short name (extract first part before dots)
    pub fn device_short_name(device: &str) -> String {
        device.split('.').next().unwrap_or(device).to_string()
    }

    /// Format layer assignments compactly (e.g., [0..11, 12..23, 24..35])
    pub fn format_layers(layers: &[Vec<u32>]) -> String {
        let ranges: Vec<String> = layers
            .iter()
            .map(|range| {
                if range.is_empty() {
                    "[]".to_string()
                } else if range.len() == 1 {
                    range[0].to_string()
                } else {
                    format!("{}..{}", range.first().unwrap(), range.last().unwrap())
                }
            })
            .collect();
        format!("[{}]", ranges.join(", "))
    }
}
impl App {
    pub fn draw_topology(&mut self, frame: &mut Frame, state: &TopologyState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Topology Ring View").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match state {
            TopologyState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading topology...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            TopologyState::Error(err) => {
                // Check if it's a "no topology" message and style accordingly
                let (text, style) = if err.contains("No topology configured") || err.contains("No topology available") {
                    (
                        vec![
                            Line::from(""),
                            Line::from("No Topology Configured").bold().yellow(),
                            Line::from(""),
                            Line::from("The API is running, but no topology has been set up yet."),
                            Line::from("Please load a model first to create a topology."),
                            Line::from(""),
                            Line::from("You can load a model by:"),
                            Line::from("  1. Going back to the main menu (Esc)"),
                            Line::from("  2. Selecting 'Load Model'"),
                            Line::from("  3. Choosing your desired model"),
                            Line::from(""),
                            Line::from("This will automatically prepare the topology for you.").dim(),
                            Line::from(""),
                        ],
                        Style::default().fg(Color::Yellow)
                    )
                } else if err.contains("Cannot connect to API server") {
                    (
                        vec![
                            Line::from(""),
                            Line::from("Connection Error").bold().red(),
                            Line::from(""),
                            Line::from(err.as_str()),
                            Line::from(""),
                            Line::from("Please check:"),
                            Line::from("  1. The API server is running"),
                            Line::from("  2. The URL in settings is correct"),
                            Line::from("  3. Your network connection"),
                            Line::from(""),
                        ],
                        Style::default().fg(Color::Red)
                    )
                } else {
                    (
                        vec![
                            Line::from(""),
                            Line::from("Error Loading Topology").bold().red(),
                            Line::from(""),
                            Line::from(err.as_str()),
                            Line::from(""),
                        ],
                        Style::default().fg(Color::Red)
                    )
                };

                frame.render_widget(
                    Paragraph::new(text)
                        .block(Block::bordered())
                        .style(style)
                        .centered(),
                    content_area,
                );
            }
            TopologyState::Loaded(topology) => {
                self.draw_topology_ring(frame, content_area, topology);
            }
        }

        // Footer
        let footer_text = match state {
            TopologyState::Loaded(_) => {
                "Use ↑↓ to select device  |  Enter to interact  |  Esc to go back"
            }
            _ => "Press Esc to go back",
        };
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    pub fn draw_topology_ring(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        topology: &TopologyResponse,
    ) {
        use std::f64::consts::PI;

        let num_devices = topology.devices.len();
        if num_devices == 0 {
            frame.render_widget(
                Paragraph::new("No devices in topology")
                    .block(Block::bordered())
                    .centered(),
                area,
            );
            return;
        }

        // Calculate circle parameters for canvas
        let radius = 35.0;
        let center_x = 0.0;
        let center_y = 0.0;

        // Prepare device data for drawing
        #[derive(Clone)]
        struct DeviceInfo {
            x: f64,
            y: f64,
            name: String,
            ip: String,
            layers: String,
            is_selected: bool,
            num_rounds: u32,
            window_size: u32,
        }

        let mut devices_info = Vec::new();

        for (i, device) in topology.devices.iter().enumerate() {
            let angle = 2.0 * PI * (i as f64) / (num_devices as f64) - PI / 2.0;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();

            // assignment info - match by checking if service contains the device instance
            let Some(assignment) = topology
                .assignments
                .iter()
                .find(|a| a.service.contains(&device.instance))
            else {
                continue;
            };

            // Get full device name (remove "shard-" prefix)
            let full_name = device
                .instance
                .strip_prefix("shard-")
                .unwrap_or(&device.instance)
                .to_string();

            // Apply sliding window animation to device name
            let short_name = self.get_sliding_text(&full_name, 30);

            // Get IP and GRPC port
            let ip = format!(
                "{}:{} ({})",
                device.local_ip, device.shard_port, device.server_port
            );

            // Get layer assignments
            let layers = TopologyResponse::format_layers(&assignment.layers);

            let is_selected = i == self.selected_device;

            devices_info.push(DeviceInfo {
                x,
                y,
                name: short_name,
                ip,
                layers,
                is_selected,
                num_rounds: assignment.layers.len() as u32,
                window_size: assignment.window_size,
            });
        }

        // Clone for use in canvas closure
        let devices_clone = devices_info
            .iter()
            .map(|d| {
                (
                    // TODO: clone with `.clone()`
                    d.x,
                    d.y,
                    d.name.clone(),
                    d.ip.clone(),
                    d.layers.clone(),
                    d.is_selected,
                    d.num_rounds,
                    d.window_size,
                )
            })
            .collect::<Vec<_>>();

        let model_info = format!(
            "Model: {}  |  Layers: {}",
            topology.model, topology.num_layers
        );

        // Draw canvas with ring
        let canvas = Canvas::default()
            .block(Block::bordered().title(model_info))
            .x_bounds([-60.0, 60.0])
            .y_bounds([-60.0, 60.0])
            .paint(move |ctx| {
                // Draw the circle
                ctx.draw(&Circle {
                    x: center_x,
                    y: center_y,
                    radius,
                    color: Color::Cyan,
                });

                // Draw connection lines between devices
                for i in 0..devices_clone.len() {
                    let (x1, y1, _, _, _, _, _, _) = devices_clone[i];
                    let next_i = (i + 1) % devices_clone.len();
                    let (x2, y2, _, _, _, _, _, _) = devices_clone[next_i];

                    ctx.draw(&CanvasLine {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: Color::DarkGray,
                    });
                }

                // Draw devices with their info
                for (x, y, name, ip, layers, is_selected, num_rounds, window_size) in
                    devices_clone.iter()
                {
                    // Draw device point with larger size if selected
                    let color = if *is_selected {
                        Color::Yellow
                    } else {
                        Color::Green
                    };

                    // Draw a larger point for better visibility
                    ctx.draw(&Points {
                        coords: &[(*x, *y)],
                        color,
                    });

                    // If selected, draw additional points around it to make it stand out
                    if *is_selected {
                        ctx.draw(&Points {
                            coords: &[
                                (*x + 0.5, *y),
                                (*x - 0.5, *y),
                                (*x, *y + 0.5),
                                (*x, *y - 0.5),
                            ],
                            color: Color::Yellow,
                        });
                    }

                    // Calculate text offset based on position to avoid overlap with circle
                    let text_offset = 5.0;
                    let angle = y.atan2(*x);
                    let text_x = x + text_offset * angle.cos();
                    let text_y = y + text_offset * angle.sin();

                    // Draw device info: name, IP, layers, rounds/window (each on a separate line)
                    // Highlight text in yellow if selected
                    let rounds_window_text =
                        format!("Rounds: {}, Window: {}", num_rounds, window_size);
                    if *is_selected {
                        ctx.print(text_x, text_y + 4.5, name.clone().yellow());
                        ctx.print(text_x, text_y + 1.2, ip.clone().yellow());
                        ctx.print(text_x, text_y - 1.2, layers.clone().yellow());
                        ctx.print(text_x, text_y - 4.5, rounds_window_text.yellow());
                    } else {
                        ctx.print(text_x, text_y + 4.5, name.clone());
                        ctx.print(text_x, text_y + 1.2, ip.clone());
                        ctx.print(text_x, text_y - 1.2, layers.clone());
                        ctx.print(text_x, text_y - 4.5, rounds_window_text);
                    }
                }
            });

        frame.render_widget(canvas, area);
    }

    pub fn draw_shard_interaction(&mut self, frame: &mut Frame, device: &str) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let short_name = TopologyResponse::device_short_name(device);
        let title = Line::from(format!("Shard Interaction: {}", short_name))
            .bold()
            .blue()
            .centered();
        frame.render_widget(Paragraph::new(title).block(Block::bordered()), title_area);

        // Content - Placeholder for now
        let content = vec![
            Line::from(""),
            Line::from(format!("Device: {}", device)).bold(),
            Line::from(""),
            Line::from("This window will allow you to:"),
            Line::from("  • Send GET/POST requests to this shard"),
            Line::from("  • View shard information"),
            Line::from("  • Test connectivity"),
            Line::from(""),
            Line::from("Coming soon...").dim(),
        ];

        frame.render_widget(
            Paragraph::new(content).block(Block::bordered().title("Shard Communication")),
            content_area,
        );

        // Footer
        frame.render_widget(
            Paragraph::new("Press Esc to go back to topology")
                .block(Block::bordered())
                .centered(),
            footer_area,
        );
    }

    pub fn handle_topology_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.state = AppState::Menu;
                self.selected_device = 0; // Reset selection
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up) => self.topology_device_up(),
            (_, KeyCode::Down) => self.topology_device_down(),
            (_, KeyCode::Enter) => self.open_shard_interaction(),
            _ => {}
        }
    }

    pub fn handle_shard_interaction_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                // Go back to topology view - restore the topology state
                if let AppState::ShardView(_) = &self.state {
                    // We need to restore the topology - for now go back to menu
                    // TODO: Keep topology state when entering shard interaction
                    self.state = AppState::Menu;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    fn topology_device_up(&mut self) {
        if let AppState::TopologyView(TopologyState::Loaded(topology)) = &self.state {
            let device_count = topology.devices.len();
            if device_count > 0 {
                // Cycle: if at 0, wrap to last device
                if self.selected_device == 0 {
                    self.selected_device = device_count - 1;
                } else {
                    self.selected_device -= 1;
                }
            }
        }
    }

    fn topology_device_down(&mut self) {
        if let AppState::TopologyView(TopologyState::Loaded(topology)) = &self.state {
            let device_count = topology.devices.len();
            if device_count > 0 {
                // Cycle: if at last, wrap to 0
                self.selected_device = (self.selected_device + 1) % device_count;
            }
        }
    }

    fn open_shard_interaction(&mut self) {
        if let AppState::TopologyView(TopologyState::Loaded(topology)) = &self.state {
            if let Some(device) = topology.devices.get(self.selected_device) {
                self.state = AppState::ShardView(device.instance.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_fetch_topology() {
        let api_url = "http://localhost:8080";
        let topology = TopologyState::fetch(api_url).await;
        println!("{:#?}", topology);
        assert!(topology.is_ok());
    }
}
