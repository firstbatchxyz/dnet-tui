use crate::{app::AppState, views::topology::TopologyState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

impl crate::App {
    pub(super) fn draw_shard_interaction(&mut self, frame: &mut Frame, device: &str) {
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

    pub(super) fn handle_shard_interaction_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                // go back to topology view - restore the topology state
                if let AppState::Topology(TopologyState::Shard(_)) = &self.state {
                    self.state = AppState::Topology(super::TopologyState::Ring(
                        super::TopologyRingState::Loaded,
                    ));
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    /// Handle async operations for shard interaction state (called during tick).
    pub(super) async fn tick_topology_shard(
        &mut self,
        _device: &str,
    ) {
        // No async operations for shard view yet (placeholder for future functionality)
    }
}
