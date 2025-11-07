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

// TODO: sloppy code here, will fix & shall do better styled error messages

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
    /// Devices refresh interval in seconds.
    DevicesRefreshInterval,
    /// Quantization level.
    KVBits,
    /// Sequence length to optimize for.
    SequenceLength,
    /// Max batch size as power of 2 exponent.
    MaxBatchExp,
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
        let host_value =
            if self.is_editing_setting && matches!(self.selected_field, SettingsField::Host) {
                format!("{}_", self.input_buffer)
            } else {
                self.temp_config.api_host.clone()
            };

        let port_value =
            if self.is_editing_setting && matches!(self.selected_field, SettingsField::Port) {
                format!("{}_", self.input_buffer)
            } else {
                self.temp_config.api_port.to_string()
            };

        let max_tokens_value =
            if self.is_editing_setting && matches!(self.selected_field, SettingsField::MaxTokens) {
                format!("{}_", self.input_buffer)
            } else {
                self.temp_config.max_tokens.to_string()
            };

        let temperature_value = if self.is_editing_setting
            && matches!(self.selected_field, SettingsField::Temperature)
        {
            format!("{}_", self.input_buffer)
        } else {
            format!("{:.2}", self.temp_config.temperature)
        };

        let devices_refresh_value = if self.is_editing_setting
            && matches!(self.selected_field, SettingsField::DevicesRefreshInterval)
        {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.devices_refresh_interval.to_string()
        };

        let kv_bits_value =
            if self.is_editing_setting && matches!(self.selected_field, SettingsField::KVBits) {
                format!("{}_", self.input_buffer)
            } else {
                self.temp_config.kv_bits.to_string()
            };

        let max_batch_exp_value = if self.is_editing_setting
            && matches!(self.selected_field, SettingsField::MaxBatchExp)
        {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.max_batch_exp.to_string()
        };

        let sequence_length_value = if self.is_editing_setting
            && matches!(self.selected_field, SettingsField::SequenceLength)
        {
            format!("{}_", self.input_buffer)
        } else {
            self.temp_config.seq_len.to_string()
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
                "  Devices Refresh: ".into(),
                devices_refresh_value.set_style(field_style(SettingsField::DevicesRefreshInterval)),
                " seconds".into(),
            ]),
            Line::from(""),
            Line::from(vec![
                "  KV Bits:         ".into(),
                kv_bits_value.set_style(field_style(SettingsField::KVBits)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Max Batch Exp:   ".into(),
                max_batch_exp_value.set_style(field_style(SettingsField::MaxBatchExp)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Sequence Length: ".into(),
                sequence_length_value.set_style(field_style(SettingsField::SequenceLength)),
            ]),
            // config
            Line::from(""),
            Line::from(vec![
                "  Current config:  ".dim(),
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
        // If we're currently editing
        if self.is_editing_setting {
            match key.code {
                KeyCode::Enter => self.apply_edit(),
                KeyCode::Esc => {
                    self.is_editing_setting = false;
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
            SettingsField::DevicesRefreshInterval => SettingsField::Temperature,
            SettingsField::KVBits => SettingsField::DevicesRefreshInterval,
            SettingsField::MaxBatchExp => SettingsField::KVBits,
            SettingsField::SequenceLength => SettingsField::MaxBatchExp,
        };
    }

    fn settings_down(&mut self) {
        self.selected_field = match self.selected_field {
            SettingsField::Host => SettingsField::Port,
            SettingsField::Port => SettingsField::MaxTokens,
            SettingsField::MaxTokens => SettingsField::Temperature,
            SettingsField::Temperature => SettingsField::DevicesRefreshInterval,
            SettingsField::DevicesRefreshInterval => SettingsField::KVBits,
            SettingsField::KVBits => SettingsField::MaxBatchExp,
            SettingsField::MaxBatchExp => SettingsField::SequenceLength,
            SettingsField::SequenceLength => SettingsField::SequenceLength,
        };
    }

    fn start_edit(&mut self) {
        self.is_editing_setting = true;
        self.input_buffer = match self.selected_field {
            SettingsField::Host => self.temp_config.api_host.clone(),
            SettingsField::Port => self.temp_config.api_port.to_string(),
            SettingsField::MaxTokens => self.temp_config.max_tokens.to_string(),
            SettingsField::Temperature => format!("{:.2}", self.temp_config.temperature),
            SettingsField::DevicesRefreshInterval => {
                self.temp_config.devices_refresh_interval.to_string()
            }
            SettingsField::KVBits => self.temp_config.kv_bits.to_string(),
            SettingsField::MaxBatchExp => self.temp_config.max_batch_exp.to_string(),
            SettingsField::SequenceLength => self.temp_config.seq_len.to_string(),
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
            SettingsField::DevicesRefreshInterval => match self.input_buffer.parse::<u64>() {
                Ok(interval) if interval > 0 && interval <= 3600 => {
                    self.temp_config.devices_refresh_interval = interval;
                    self.status_message =
                        "Devices refresh interval updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message = "Invalid interval (must be 1-3600 seconds)!".to_string();
                }
            },
            SettingsField::KVBits => match self.input_buffer.as_str() {
                "4bit" => {
                    self.temp_config.kv_bits = crate::config::KVBits::Bits4;
                    self.status_message = "KV Bits updated (press 's' to save)".to_string();
                }
                "8bit" => {
                    self.temp_config.kv_bits = crate::config::KVBits::Bits8;
                    self.status_message = "KV Bits updated (press 's' to save)".to_string();
                }
                "fp16" => {
                    self.temp_config.kv_bits = crate::config::KVBits::FP16;
                    self.status_message = "KV Bits updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message =
                        "Invalid KV Bits (must be '4bit', '8bit', or 'fp16')!".to_string();
                }
            },
            SettingsField::MaxBatchExp => match self.input_buffer.parse::<u8>() {
                Ok(exp) if exp <= 8 => {
                    self.temp_config.max_batch_exp = exp;
                    self.status_message =
                        "Max Batch Exponent updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message = "Invalid Max Batch Exponent (must be 0-8)!".to_string();
                }
            },
            SettingsField::SequenceLength => match self.input_buffer.parse::<u32>() {
                Ok(len) if len != 0 => {
                    self.temp_config.seq_len = len;
                    self.status_message = "Sequence Length updated (press 's' to save)".to_string();
                }
                _ => {
                    self.status_message = "Invalid Sequence Length (must be non-zero)!".to_string();
                }
            },
        }
        self.input_buffer.clear();
        self.is_editing_setting = false;
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
