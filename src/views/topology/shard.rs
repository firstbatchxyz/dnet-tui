use crate::common::ShardHealthResponse;
use crate::{App, app::AppView, views::topology::TopologyView};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ShardView {
    Loading,
    Loaded(ShardHealthResponse),
    Error(String),
}

impl ShardView {
    /// Fetch shard health from the shard's HTTP endpoint
    pub async fn fetch(device_ip: &str, http_port: u16) -> Result<ShardHealthResponse, String> {
        let url = format!("http://{}:{}/health", device_ip, http_port);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to connect to shard: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Shard returned error: {}", response.status()));
        }

        let health: ShardHealthResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(health)
    }
}

impl App {
    pub(super) fn draw_shard_interaction(
        &mut self,
        frame: &mut Frame,
        device_instance: &str,
        state: &ShardView,
    ) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from(format!("Shard: {}", device_instance))
            .bold()
            .cyan()
            .centered();
        frame.render_widget(
            Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        // Content
        match state {
            ShardView::Loading => {
                let lines = vec![
                    Line::from(""),
                    Line::from("Loading shard health...").bold(),
                    Line::from(""),
                ];
                frame.render_widget(
                    Paragraph::new(lines).block(Block::bordered()).centered(),
                    content_area,
                );
            }
            ShardView::Error(err) => {
                let error_lines = vec![
                    Line::from(""),
                    Line::from("Error Loading Shard Health").bold().red(),
                    Line::from(""),
                    Line::from(err.as_str()),
                    Line::from(""),
                ];
                frame.render_widget(
                    Paragraph::new(error_lines)
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            ShardView::Loaded(health) => {
                self.draw_shard_health(frame, content_area, health);
            }
        }

        // Footer
        frame.render_widget(
            Paragraph::new("Press Esc to go back to topology")
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    fn draw_shard_health(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        health: &ShardHealthResponse,
    ) {
        // Create a nice layout displaying all health information
        let mut lines = Vec::new();

        // Status header with color coding
        let status_line = if health.status == "ok" && health.running {
            Line::from(vec![
                "Status: ".into(),
                health.status.clone().bold().green(),
                " ● ".green(),
                "RUNNING".bold().green(),
            ])
        } else if health.running {
            Line::from(vec![
                "Status: ".into(),
                health.status.clone().bold().yellow(),
                " ● ".yellow(),
                "RUNNING".bold().yellow(),
            ])
        } else {
            Line::from(vec![
                "Status: ".into(),
                health.status.clone().bold().red(),
                " ● ".red(),
                "STOPPED".bold().red(),
            ])
        };
        lines.push(Line::from(""));
        lines.push(status_line);
        lines.push(Line::from(""));

        // Node information
        lines.push("━━━ Node Information ━━━".bold().cyan().into());
        lines.push(format!("  Instance:       {}", health.instance).into());
        lines.push(format!("  HTTP Port:      {}", health.http_port).into());
        lines.push(format!("  gRPC Port:      {}", health.grpc_port).into());
        lines.push("".into());

        // Model information
        lines.push("━━━ Model Information ━━━".bold().cyan().into());
        let model_status = if health.model_loaded {
            "Loaded".bold().green()
        } else {
            "Not Loaded".bold().yellow()
        };
        lines.push(format!("  Model Status:   {}", model_status).into());

        if let Some(model_path) = &health.model_path {
            lines.push(format!("  Model Path:     {}", model_path).into());
        }
        lines.push("".into());

        // Layer assignments
        lines.push("━━━ Layer Assignment ━━━".bold().cyan().into());
        if health.assigned_layers.is_empty() {
            lines.push("  No layers assigned".dark_gray().into());
        } else {
            let layers_display = format_layer_ranges(&health.assigned_layers);
            lines.push(format!("  Assigned:       {}", layers_display).into());
            lines.push(format!("  Count:          {} layers", health.assigned_layers.len()).into());
        }
        lines.push("".into());

        // Queue information
        lines.push("━━━ Queue Status ━━━".bold().cyan().into());
        let (queue_health, queue_status) = match health.queue_size {
            0 => (
                health.queue_size.to_string().bold().green(),
                "idle".dark_gray(),
            ),
            1..=9 => (
                health.queue_size.to_string().bold().yellow(),
                "active".dark_gray(),
            ),
            _ => (
                health.queue_size.to_string().bold().red(),
                "busy".dark_gray(),
            ),
        };
        let queue_display = format!("  Queue Size:     {queue_health} ({})", queue_status).into();
        lines.push(queue_display);

        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title("Health Status")),
            area,
        );
    }

    pub(super) fn handle_shard_interaction_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // go back to topology view
                if let AppView::Topology(TopologyView::Shard(_, _)) = &self.view {
                    self.view = AppView::Topology(super::TopologyView::Ring(
                        super::TopologyRingView::Loaded,
                    ));
                }
            }
            _ => {}
        }
    }

    /// Handle async operations for shard interaction state (called during tick).
    pub(super) async fn tick_topology_shard(&mut self, device: &str, state: &ShardView) {
        if matches!(state, ShardView::Loading) {
            // Find the device in the topology to get its IP and port
            if let Some(topology) = &self.topology {
                if let Some(dev) = topology.devices.iter().find(|d| d.instance == device) {
                    let device_ip = dev.local_ip.clone();
                    let http_port = dev.server_port;

                    match ShardView::fetch(&device_ip, http_port).await {
                        Ok(health) => {
                            self.view = AppView::Topology(TopologyView::Shard(
                                device.to_string(),
                                ShardView::Loaded(health),
                            ));
                        }
                        Err(err) => {
                            self.view = AppView::Topology(TopologyView::Shard(
                                device.to_string(),
                                ShardView::Error(err),
                            ));
                        }
                    }
                } else {
                    self.view = AppView::Topology(TopologyView::Shard(
                        device.to_string(),
                        ShardView::Error(format!("Device '{}' not found in topology", device)),
                    ));
                }
            } else {
                self.view = AppView::Topology(TopologyView::Shard(
                    device.to_string(),
                    ShardView::Error("No topology information available".to_string()),
                ));
            }
        }
    }
}

/// Format layer numbers into compact ranges (e.g., "0-5, 10-15, 20")
fn format_layer_ranges(layers: &[u32]) -> String {
    if layers.is_empty() {
        return "none".to_string();
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
                ranges.push(format!("{}", start));
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = layer;
            end = layer;
        }
    }

    // Push the last range
    if start == end {
        ranges.push(format!("{}", start));
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    ranges.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_layer_ranges() {
        assert_eq!(format_layer_ranges(&[]), "none");
        assert_eq!(format_layer_ranges(&[0]), "0");
        assert_eq!(format_layer_ranges(&[0, 1, 2, 3]), "0-3");
        assert_eq!(format_layer_ranges(&[0, 1, 2, 5, 6, 7]), "0-2, 5-7");
        assert_eq!(format_layer_ranges(&[0, 2, 4, 6]), "0, 2, 4, 6");
        assert_eq!(
            format_layer_ranges(&[0, 1, 2, 10, 20, 21, 22]),
            "0-2, 10, 20-22"
        );
    }
}
