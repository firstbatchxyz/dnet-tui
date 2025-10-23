use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::DeveloperState;

impl crate::App {
    pub(super) fn draw_developer_menu(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Developer Menu").bold().yellow().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Menu items - just one option now
        let menu_items = vec!["Manual Layer Assignment - Manually assign layers to shards"];

        let items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.developer_menu_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL));
        frame.render_widget(list, content_area);

        // Footer
        frame.render_widget(
            Paragraph::new("Enter: Select | Esc: Back to main menu").centered(),
            footer_area,
        );
    }

    pub(super) fn handle_developer_menu_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.state.reset_to_menu();
                self.developer_menu_index = 0;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Enter) => {
                // Only one option now - Manual Layer Assignment
                if self.developer_menu_index == 0 {
                    self.state = crate::AppState::Developer(DeveloperState::ManualAssignment(
                        super::ManualAssignmentState::SelectingModel,
                    ));
                    self.selected_model = 0;
                }
            }
            _ => {}
        }
    }
}
