use crate::config::Config;
use crate::{App, AppView};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Span;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph},
};

#[derive(Default, Debug)]
pub struct SettingsState {
    /// Selected settings field.
    pub selection: SettingsField,
    /// Status message for the settings view.
    pub status: SettingsStatus,
    /// Whether we're currently editing a settings field.
    pub is_editing: bool,
    /// Temporary config for editing stuff.
    pub temp_config: Config,
}

/// Possible settings fields.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum SettingsField {
    /// API Host.
    #[default]
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
                    self.state.settings.selection,
                    self.state.settings.is_editing,
                    &self.input_buffer,
                    &self.state.settings.temp_config,
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
        if !self.state.settings.status.is_empty() {
            body_lines.push(Line::from(self.state.settings.status.to_span()));
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
        if self.state.settings.is_editing {
            // editing mode
            match key.code {
                KeyCode::Enter => self.apply_edit(),
                KeyCode::Esc => {
                    self.state.settings.is_editing = false;
                    self.input_buffer.clear();
                    self.state.settings.status.clear();
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
                    self.view = AppView::Menu;
                    self.state.settings.status.clear();
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
            .position(|s| *s == self.state.settings.selection)
            .unwrap_or_default(); // guaranteed to unwrap anyways

        self.state.settings.selection = if idx == 0 {
            SettingsField::ALL[SettingsField::ALL.len() - 1]
        } else {
            SettingsField::ALL[idx - 1]
        };
    }

    fn settings_down(&mut self) {
        let idx = SettingsField::ALL
            .iter()
            .position(|s| *s == self.state.settings.selection)
            .unwrap_or_default(); // guaranteed to unwrap anyways

        self.state.settings.selection = SettingsField::ALL[(idx + 1) % SettingsField::ALL.len()];
    }

    fn start_edit(&mut self) {
        self.state.settings.is_editing = true;
        self.input_buffer = self
            .state
            .settings
            .temp_config
            .read_setting(self.state.settings.selection);
        self.state.settings.status.clear();
    }

    fn apply_edit(&mut self) {
        match self
            .state
            .settings
            .temp_config
            .write_setting(self.state.settings.selection, &self.input_buffer)
        {
            Ok(_) => {
                self.state.settings.status = SettingsStatus::Info(format!(
                    "{} updated (press 's' to save)",
                    self.state.settings.selection.label()
                ));
                self.input_buffer.clear();
                self.state.settings.is_editing = false;
            }
            Err(e) => {
                self.state.settings.status = SettingsStatus::Error(format!("[ERROR] {}", e));
            }
        };
    }

    fn save_config(&mut self) {
        match self.state.settings.temp_config.save_to_dria() {
            Ok(_) => {
                use crate::common::ApiClient;

                self.config = self.state.settings.temp_config.clone();
                // update API client as well
                self.api = ApiClient::new(&self.config.api_host, self.config.api_port);
                self.state.settings.status = SettingsStatus::Info(format!(
                    "Configuration saved to {}",
                    Config::current_location()
                ));
            }
            Err(e) => {
                self.state.settings.status =
                    SettingsStatus::Error(format!("[ERROR] Could not save config: {}", e));
            }
        }
    }
}
