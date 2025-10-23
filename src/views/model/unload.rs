use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub enum UnloadModelState {
    Unloading,
    Error(String),
    Success,
}

impl UnloadModelState {
    /// Unload model by calling the API
    pub async fn unload_model(api_url: &str) -> color_eyre::Result<()> {
        let url = format!("{}/v1/unload_model", api_url);
        let client = reqwest::Client::new();

        let response = client.post(&url).send().await?;

        // Check if response is successful
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(color_eyre::eyre::eyre!(
                "Failed to unload model: {}",
                error_text
            ))
        }
    }
}

impl crate::App {
    pub fn draw_unload_model(&mut self, frame: &mut Frame, state: &UnloadModelState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Unload Model").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match state {
            UnloadModelState::Unloading => {
                frame.render_widget(
                    Paragraph::new("Unloading model...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            UnloadModelState::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            UnloadModelState::Success => {
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
            UnloadModelState::Error(_) | UnloadModelState::Success => "Press Esc to go back",
            _ => "Unloading...",
        };
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    pub fn handle_unload_model_input(&mut self, key: KeyEvent, state: &UnloadModelState) {
        match state {
            UnloadModelState::Error(_) | UnloadModelState::Success => {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        self.state.reset_to_menu();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                    _ => {}
                }
            }
            _ => {
                // During unloading, only allow quitting
                if matches!(
                    (key.modifiers, key.code),
                    (
                        KeyModifiers::CONTROL,
                        KeyCode::Char('c') | KeyCode::Char('C')
                    )
                ) {
                    self.quit();
                }
            }
        }
    }
}
