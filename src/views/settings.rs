use crate::config::Config;
use crate::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Span;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
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
    SeqLen,
    /// Max batch size as power of 2 exponent.
    MaxBatchExp,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum SettingsStatus {
    #[default]
    None,
    Info(String),
    Error(String),
}

impl SettingsStatus {
    pub fn to_span(&self) -> Span<'_> {
        match self {
            SettingsStatus::None => Span::default(),
            SettingsStatus::Info(msg) => Span::styled(msg, Style::default().fg(Color::Green)),
            SettingsStatus::Error(msg) => Span::styled(msg, Style::default().fg(Color::Red)),
        }
    }

    pub fn clear(&mut self) {
        *self = SettingsStatus::None;
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, SettingsStatus::None)
    }
}

impl SettingsField {
    pub const ALL: [SettingsField; 8] = [
        SettingsField::Host,
        SettingsField::Port,
        SettingsField::MaxTokens,
        SettingsField::Temperature,
        SettingsField::DevicesRefreshInterval,
        SettingsField::KVBits,
        SettingsField::MaxBatchExp,
        SettingsField::SeqLen,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SettingsField::Host => "API Host",
            SettingsField::Port => "API Port",
            SettingsField::MaxTokens => "Max Tokens",
            SettingsField::Temperature => "Temperature",
            SettingsField::DevicesRefreshInterval => "Device Refresh (s)",
            SettingsField::KVBits => "KV Bits",
            SettingsField::MaxBatchExp => "Max Batch Exponent",
            SettingsField::SeqLen => "Sequence Length",
        }
    }

    pub fn to_line(
        &self,
        selection: SettingsField,
        is_editing: bool,
        input: &str,
        tmp: &Config,
    ) -> Line {
        let is_selected = *self == selection;

        // highlight if selected
        let field_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let label_span = Span::styled(format!("  {:<20}", self.label()), field_style);
        if is_editing {
            if is_selected {
                Line::from_iter(vec![
                    label_span,
                    Span::styled(input.to_string(), field_style),
                    Span::styled(
                        "_",
                        Style::new()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::RAPID_BLINK),
                    ),
                ])
            } else {
                Line::from_iter(vec![
                    label_span,
                    Span::styled(tmp.read_setting(*self), field_style),
                ])
            }
        } else {
            Line::from_iter(vec![
                label_span,
                Span::styled(tmp.read_setting(*self), field_style),
            ])
        }
    }
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

        // Body
        let settings_lines = SettingsField::ALL
            .iter()
            .map(|s| {
                s.to_line(
                    self.settings_selected_field,
                    self.is_editing_setting,
                    &self.input_buffer,
                    &self.temp_config,
                )
            })
            .collect::<Vec<_>>();

        // start with one empty line
        let mut body_lines = vec![];
        body_lines.extend_from_slice(&settings_lines);
        body_lines.push(
            vec![
                "  Current config:  ".dim(),
                Config::current_location().dim(),
            ]
            .into(),
        );

        // if there is a status message, add that as well
        if !self.settings_status.is_empty() {
            body_lines.push(Line::from(self.settings_status.to_span()));
        }

        // add an empty line in between every element (better readability)
        for i in 1..body_lines.len() {
            body_lines.insert(i * 2 - 1, Line::from(" "));
        }

        frame.render_widget(
            Paragraph::new(body_lines)
                .block(Block::default().title("Use ↑↓ to select field, Enter to edit, s to save")),
            settings_area,
        );

        // Footer
        let footer_text = "Press Esc to go back  |  Enter to edit field  |  s to save";
        frame.render_widget(Paragraph::new(footer_text).centered(), footer_area);
    }

    pub fn handle_settings_input(&mut self, key: KeyEvent) {
        if self.is_editing_setting {
            // editing mode
            match key.code {
                KeyCode::Enter => self.apply_edit(),
                KeyCode::Esc => {
                    self.is_editing_setting = false;
                    self.input_buffer.clear();
                    self.settings_status.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }
        } else {
            // normal settings navigation
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.state = AppState::Menu;
                    self.settings_status.clear();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                (_, KeyCode::Up) => self.settings_up(),
                (_, KeyCode::Down) => self.settings_down(),
                (_, KeyCode::Enter) => self.start_edit(),
                (_, KeyCode::Char('s')) => self.save_config(),
                _ => {}
            }
        }
    }

    fn settings_up(&mut self) {
        let idx = SettingsField::ALL
            .iter()
            .position(|s| *s == self.settings_selected_field)
            .unwrap_or_default(); // guaranteed to unwrap anyways

        self.settings_selected_field = SettingsField::ALL[(idx - 1) % SettingsField::ALL.len()];
    }

    fn settings_down(&mut self) {
        let idx = SettingsField::ALL
            .iter()
            .position(|s| *s == self.settings_selected_field)
            .unwrap_or_default(); // guaranteed to unwrap anyways

        self.settings_selected_field = SettingsField::ALL[(idx + 1) % SettingsField::ALL.len()];
    }

    fn start_edit(&mut self) {
        self.is_editing_setting = true;
        self.input_buffer = self.temp_config.read_setting(self.settings_selected_field);
        self.settings_status.clear();
    }

    fn apply_edit(&mut self) {
        match self
            .temp_config
            .write_setting(self.settings_selected_field, &self.input_buffer)
        {
            Ok(_) => {
                self.settings_status = SettingsStatus::Info(format!(
                    "{} updated (press 's' to save)",
                    self.settings_selected_field.label()
                ));
                self.input_buffer.clear();
                self.is_editing_setting = false;
            }
            Err(e) => {
                self.settings_status = SettingsStatus::Error(format!("[ERROR] {}", e));
            }
        };
    }

    fn save_config(&mut self) {
        match self.temp_config.save_to_dria() {
            Ok(_) => {
                self.config = self.temp_config.clone();
                self.settings_status = SettingsStatus::Info(format!(
                    "Configuration saved to {}",
                    Config::current_location()
                ));
            }
            Err(e) => {
                self.settings_status =
                    SettingsStatus::Error(format!("[ERROR] Could not save config: {}", e));
            }
        }
    }
}
