use crate::{app::AppState, views::topology::TopologyResponse};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardHealthResponse {
    /// Health status (e.g., 'ok')
    pub status: String,
    /// Node identifier
    pub node_id: u32,
    /// Whether the node is running
    pub running: bool,
    /// Whether a model is currently loaded
    pub model_loaded: bool,
    /// Path to currently loaded model
    pub model_path: Option<String>,
    /// Layers assigned to this shard
    pub assigned_layers: Vec<u32>,
    /// Current activation queue size
    pub queue_size: u32,
    /// gRPC server port
    pub grpc_port: u16,
    /// HTTP server port
    pub http_port: u16,
    /// Short shard instance name (service label)
    pub instance: Option<String>,
}

impl crate::App {
    pub fn draw_shard_interaction(&mut self, frame: &mut Frame, device: &str) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from(format!("Shard Interaction: {}", device))
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

    pub fn handle_shard_interaction_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                // Go back to topology view - restore the topology state
                if let AppState::ShardView(_) = &self.state {
                    // We need to restore the topology - for now go back to menu
                    // TODO: Keep topology state when entering shard interaction
                    self.state.reset_to_menu();
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }
}
