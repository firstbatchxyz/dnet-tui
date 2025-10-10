use crate::app::{App, AppState};
use crate::config::Config;
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
        frame.render_widget(Paragraph::new(title).block(Block::bordered()), title_area);

        // Settings fields
        // TODO: a better way maybe?
        let is_editing_host =
            matches!(self.selected_field, SettingsField::Host) && !self.input_buffer.is_empty();
        let is_editing_port =
            matches!(self.selected_field, SettingsField::Port) && !self.input_buffer.is_empty();

        let host_style = if matches!(self.selected_field, SettingsField::Host) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let port_style = if matches!(self.selected_field, SettingsField::Port) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Show input_buffer if editing, otherwise show temp_config value
        let host_value = if is_editing_host {
            format!("{}_", self.input_buffer) // Add cursor
        } else {
            self.temp_config.api_host.clone()
        };

        let port_value = if is_editing_port {
            format!("{}_", self.input_buffer) // Add cursor
        } else {
            self.temp_config.api_port.to_string()
        };

        let mut settings_text = vec![
            Line::from(""),
            Line::from(vec![
                "  API Host: ".into(),
                host_value.set_style(host_style),
            ]),
            Line::from(""),
            Line::from(vec![
                "  API Port: ".into(),
                port_value.set_style(port_style),
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
            SettingsField::Port => SettingsField::Host,
            SettingsField::Host => SettingsField::Host,
        };
    }

    fn settings_down(&mut self) {
        self.selected_field = match self.selected_field {
            SettingsField::Host => SettingsField::Port,
            SettingsField::Port => SettingsField::Port,
        };
    }

    fn start_edit(&mut self) {
        self.input_buffer = match self.selected_field {
            SettingsField::Host => self.temp_config.api_host.clone(),
            SettingsField::Port => self.temp_config.api_port.to_string(),
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
