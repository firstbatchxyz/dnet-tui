use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub enum UnloadModelView {
    Unloading,
    Error(String),
    Success,
}

impl crate::App {
    pub(super) fn draw_unload_model(&mut self, frame: &mut Frame, state: &UnloadModelView) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Unload Model").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match state {
            UnloadModelView::Unloading => {
                frame.render_widget(
                    Paragraph::new("Unloading model...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            UnloadModelView::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            UnloadModelView::Success => {
                frame.render_widget(
                    Paragraph::new("Model unloaded successfully!")
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Green))
                        .centered(),
                    content_area,
                );
            }
        }

        // Footer
        let footer_text = match state {
            UnloadModelView::Error(_) | UnloadModelView::Success => "Press Esc to go back",
            UnloadModelView::Unloading => "Please wait...",
        };
        frame.render_widget(Paragraph::new(footer_text).centered().gray(), footer_area);
    }

    pub(super) fn handle_unload_model_input(&mut self, key: KeyEvent, _state: &UnloadModelView) {
        // only allow ESC to go back
        match key.code {
            KeyCode::Esc => self.view = crate::AppView::Menu,
            _ => {}
        }
    }

    /// Handle async operations for unload model state (called during tick).
    pub(super) async fn tick_unload_model(&mut self, view: &UnloadModelView) {
        if matches!(view, UnloadModelView::Unloading) {
            match self.api.unload_model().await {
                Ok(_) => {
                    self.view =
                        crate::AppView::Model(super::ModelView::Unload(UnloadModelView::Success));
                    if let Some(topology) = &mut self.topology {
                        topology.model = None;
                    };
                }
                Err(err) => {
                    self.view = crate::AppView::Model(super::ModelView::Unload(
                        UnloadModelView::Error(err.to_string()),
                    ));
                }
            }
        }
    }
}
