use crate::config::Config;
use crate::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Styled, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

/// Possible settings fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsField {
    /// API Host.
    Host,
    /// API Port.
    Port,
    /// Max tokens for chat responses.
    MaxTokens,
    /// Temperature for chat responses.
    Temperature,
}

impl App {
    pub fn draw_settings(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Create layout
        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Settings fields
            Constraint::Length(3), // Footer
        ]);
        let [title_area, settings_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Settings").bold().blue().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Settings fields
        let is_editing = !self.input_buffer.is_empty();

        let field_style = |field: SettingsField| {
            if matches!(self.selected_field, f if f == field) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }
        };

        // Show input_buffer if editing, otherwise show temp_config value
        let host_value = if is_editing && matches!(self.selected_field, SettingsField::Host) {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.api_host.clone()
        };

        let port_value = if is_editing && matches!(self.selected_field, SettingsField::Port) {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.api_port.to_string()
        };

        let max_tokens_value = if is_editing && matches!(self.selected_field, SettingsField::MaxTokens) {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.max_tokens.to_string()
        };

        let temperature_value = if is_editing && matches!(self.selected_field, SettingsField::Temperature) {
            format!("{}_", self.input_buffer)
        } else {
            format!("{:.2}", self.temp_config.temperature)
        };

        let mut settings_text = vec![
            Line::from(""),
            Line::from(vec![
                "  API Host:        ".into(),
                host_value.set_style(field_style(SettingsField::Host)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  API Port:        ".into(),
                port_value.set_style(field_style(SettingsField::Port)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Max Tokens:      ".into(),
                max_tokens_value.set_style(field_style(SettingsField::MaxTokens)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Temperature:     ".into(),
                temperature_value.set_style(field_style(SettingsField::Temperature)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Current config: ".dim(),
                Config::current_location().dim(),
            ]),
        ];

        // Add status message below the current config line if present
        if !self.status_message.is_empty() {
            settings_text.push(Line::from(""));
            settings_text.push(Line::from(format!("  {}", self.status_message)).green());
        }

        frame.render_widget(
            Paragraph::new(settings_text)
                .block(Block::default().title("Use ↑↓ to select field, Enter to edit, s to save")),
            settings_area,
        );

        // Footer
        let footer_text = "Press Esc to go back  |  Enter to edit field  |  s to save";
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    pub fn handle_settings_input(&mut self, key: KeyEvent) {
        // If we're currently editing (input_buffer is not empty)
        if !self.input_buffer.is_empty() {
            match key.code {
                KeyCode::Enter => self.apply_edit(),
                KeyCode::Esc => {
                    self.input_buffer.clear();
                    self.status_message.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        // Normal settings navigation
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.state = AppState::Menu;
                self.status_message.clear();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up) => self.settings_up(),
            (_, KeyCode::Down) => self.settings_down(),
            (_, KeyCode::Enter) => self.start_edit(),
            (_, KeyCode::Char('s')) => self.save_config(),
            _ => {}
        }
    }

    fn settings_up(&mut self) {
        self.selected_field = match self.selected_field {
            SettingsField::Host => SettingsField::Host,
            SettingsField::Port => SettingsField::Host,
            SettingsField::MaxTokens => SettingsField::Port,
            SettingsField::Temperature => SettingsField::MaxTokens,
        };
    }

    fn settings_down(&mut self) {
        self.selected_field = match self.selected_field {
            SettingsField::Host => SettingsField::Port,
            SettingsField::Port => SettingsField::MaxTokens,
            SettingsField::MaxTokens => SettingsField::Temperature,
            SettingsField::Temperature => SettingsField::Temperature,
        };
    }

    fn start_edit(&mut self) {
        self.input_buffer = match self.selected_field {
            SettingsField::Host => self.temp_config.api_host.clone(),
            SettingsField::Port => self.temp_config.api_port.to_string(),
            SettingsField::MaxTokens => self.temp_config.max_tokens.to_string(),
            SettingsField::Temperature => format!("{:.2}", self.temp_config.temperature),
        };
        self.status_message.clear();
    }

    fn apply_edit(&mut self) {
        match self.selected_field {
            SettingsField::Host => {
                self.temp_config.api_host = self.input_buffer.clone();
                self.status_message = "Host updated (press 's' to save)".to_string();
            }
            SettingsField::Port => match self.input_buffer.parse::<u16>() {
                Ok(port) => {
                    self.temp_config.api_port = port;
                    self.status_message = "Port updated (press 's' to save)".to_string();
                }
                Err(_) => {
                    self.status_message = "Invalid port number!".to_string();
                }
            },
            SettingsField::MaxTokens => match self.input_buffer.parse::<u32>() {
                Ok(tokens) if tokens > 0 && tokens <= 100000 => {
                    self.temp_config.max_tokens = tokens;
                    self.status_message = "Max tokens updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message = "Invalid max tokens (must be 1-100000)!".to_string();
                }
            },
            SettingsField::Temperature => match self.input_buffer.parse::<f32>() {
                Ok(temp) if temp >= 0.0 && temp <= 2.0 => {
                    self.temp_config.temperature = temp;
                    self.status_message = "Temperature updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message = "Invalid temperature (must be 0.0-2.0)!".to_string();
                }
            },
        }
        self.input_buffer.clear();
    }

    fn save_config(&mut self) {
        match self.temp_config.save_to_dria() {
            Ok(_) => {
                self.config = self.temp_config.clone();
                self.status_message =
                    format!("Configuration saved to {}", Config::current_location());
            }
            Err(e) => {
                self.status_message = format!("Failed to save: {}", e);
            }
        }
    }
}
