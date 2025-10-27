use crate::common::TopologyInfo;
use crate::{app::AppState, utils::get_sliding_text};
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

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyRingState {
    Loading,
    Loaded,
    Error(String),
}

impl TopologyInfo {
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
impl crate::App {
    pub(super) fn draw_topology_ring_view(&mut self, frame: &mut Frame, state: &TopologyRingState) {
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
            TopologyRingState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading topology...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            TopologyRingState::Error(err) => {
                // Check if it's a "no topology" message and style accordingly
                let (text, style) = if err.contains("No topology configured")
                    || err.contains("No topology available")
                {
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
                            Line::from("This will automatically prepare the topology for you.")
                                .dim(),
                            Line::from(""),
                        ],
                        Style::default().fg(Color::Yellow),
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
                        Style::default().fg(Color::Red),
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
                        Style::default().fg(Color::Red),
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
            TopologyRingState::Loaded => {
                if let Some(topology) = self.topology_info.clone() {
                    self.draw_topology_ring(frame, content_area, &topology);
                } else {
                    frame.render_widget(
                        Paragraph::new("No topology data available")
                            .block(Block::bordered())
                            .centered(),
                        content_area,
                    );
                }
            }
        }

        // Footer
        let footer_text = match state {
            TopologyRingState::Loaded => {
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
        topology: &TopologyInfo,
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

            // Get device name without "shard-" prefix
            let instance = device
                .instance
                .strip_prefix("shard-")
                .unwrap_or(&device.instance)
                .to_string();

            // Apply sliding window animation to device name
            let short_name = get_sliding_text(self.animation_start.elapsed(), &instance, 30);

            // Get IP and GRPC port
            let ip = format!(
                "{}:{} ({})",
                device.local_ip, device.shard_port, device.server_port
            );

            // Get layer assignments
            let layers = TopologyInfo::format_layers(&assignment.layers);

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
                            #[rustfmt::skip]
                            coords: &[
                                (*x + 0.5, *y      ),
                                (*x - 0.5, *y      ),
                                (*x      , *y + 0.5),
                                (*x      , *y - 0.5)
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

    pub(super) fn handle_topology_ring_input(&mut self, key: KeyEvent) {
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

    fn topology_device_up(&mut self) {
        if let AppState::Topology(super::TopologyState::Ring(TopologyRingState::Loaded)) =
            &self.state
        {
            if let Some(topology) = &self.topology_info {
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
    }

    fn topology_device_down(&mut self) {
        if let AppState::Topology(super::TopologyState::Ring(TopologyRingState::Loaded)) =
            &self.state
        {
            if let Some(topology) = &self.topology_info {
                let device_count = topology.devices.len();
                if device_count > 0 {
                    // Cycle: if at last, wrap to 0
                    self.selected_device = (self.selected_device + 1) % device_count;
                }
            }
        }
    }

    fn open_shard_interaction(&mut self) {
        if let AppState::Topology(super::TopologyState::Ring(TopologyRingState::Loaded)) =
            &self.state
        {
            if let Some(topology) = &self.topology_info {
                if let Some(device) = topology.devices.get(self.selected_device) {
                    self.state = AppState::Topology(super::TopologyState::Shard(
                        device.instance.clone(),
                    ));
                }
            }
        }
    }

    /// Handle async operations for topology ring state (called during tick).
    pub(super) async fn tick_topology_ring(&mut self, state: &TopologyRingState) {
        if matches!(state, TopologyRingState::Loading) {
            self.load_topology().await;
        }
    }

    /// Load topology asynchronously and update state.
    async fn load_topology(&mut self) {
        match TopologyInfo::fetch(&self.config.api_url()).await {
            Ok(topology) => {
                self.topology_info = Some(topology);
                self.state = AppState::Topology(super::TopologyState::Ring(
                    TopologyRingState::Loaded,
                ));
            }
            Err(err) => {
                // TODO: handle this better
                // Check if the error is likely due to no model being loaded
                let error_msg = err.to_string();
                let friendly_msg = if error_msg.contains("No topology configured")
                    || error_msg.contains("No topology found")
                    || error_msg.contains("model not loaded")
                    || error_msg.contains("prepare_topology")
                    || error_msg.contains("404")
                    || error_msg.contains("Not Found")
                {
                    "No topology configured yet. Please load a model first to create a topology."
                        .to_string()
                } else if error_msg.contains("connection")
                    || error_msg.contains("refused")
                    || error_msg.contains("error sending request")
                {
                    format!(
                        "Cannot connect to API server. Please check your settings and ensure the server is running.",
                    )
                } else {
                    format!("Error: {}", error_msg)
                };

                self.state = AppState::Topology(super::TopologyState::Ring(
                    TopologyRingState::Error(friendly_msg),
                ));
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
        let topology = TopologyInfo::fetch(api_url).await;
        println!("{:#?}", topology);
        assert!(topology.is_ok());
    }

    #[test]
    fn test_format_layers() {
        let layers = vec![
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            vec![12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
            vec![24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35],
            vec![36],
        ];
        let formatted = TopologyInfo::format_layers(&layers);
        assert_eq!(formatted, "[0..11, 12..23, 24..35, 36]");
    }
}
