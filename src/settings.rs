use crate::app::{App, AppState, SettingsField};
use crate::config::Config;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Styled, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

impl App {
    pub fn draw_settings(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Create layout
        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Settings fields
            Constraint::Length(3), // Status
            Constraint::Length(3), // Footer
        ]);
        let [title_area, settings_area, status_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Settings").bold().blue().centered();
        frame.render_widget(Paragraph::new(title).block(Block::bordered()), title_area);

        // Settings fields
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

        let settings_text = vec![
            Line::from(""),
            Line::from(vec![
                "  API Host: ".into(),
                self.temp_config.api_host.clone().set_style(host_style),
            ]),
            Line::from(""),
            Line::from(vec![
                "  API Port: ".into(),
                self.temp_config.api_port.to_string().set_style(port_style),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Current config: ".dim(),
                Config::current_location().dim(),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(settings_text)
                .block(Block::bordered().title("Use ↑↓ to select field, Enter to edit, s to save")),
            settings_area,
        );

        // Status message
        let status_text = if !self.status_message.is_empty() {
            self.status_message.clone()
        } else if !self.input_buffer.is_empty() {
            format!("Editing: {}", self.input_buffer)
        } else {
            String::new()
        };

        frame.render_widget(
            Paragraph::new(status_text)
                .block(Block::bordered())
                .centered(),
            status_area,
        );

        // Footer
        frame.render_widget(
            Paragraph::new("Press Esc to go back  |  Enter to edit field  |  s to save")
                .block(Block::bordered())
                .centered(),
            footer_area,
        );
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
                    format!("Configuration saved to {}", self.temp_config.api_url());
            }
            Err(e) => {
                self.status_message = format!("Failed to save: {}", e);
            }
        }
    }
}
