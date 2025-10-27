use crate::common::{DeviceProperties, DevicesResponse};
use crate::{App, app::AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum DevicesState {
    Loading,
    Loaded(HashMap<String, DeviceProperties>),
    Error(String),
}

impl DevicesState {
    /// Fetch devices from the API
    pub async fn fetch(api_url: &str) -> Result<HashMap<String, DeviceProperties>, String> {
        let url = format!("{}/v1/devices", api_url);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to connect to API: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error: {}", response.status()));
        }

        let devices_response: DevicesResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(devices_response.devices)
    }
}

impl App {
    pub(crate) fn draw_devices(&mut self, frame: &mut Frame, state: &DevicesState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Discovered Devices").bold().cyan().centered();
        frame.render_widget(
            Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        // Content
        match state {
            DevicesState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading devices...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            DevicesState::Error(err) => {
                let error_text = vec![
                    Line::from(""),
                    Line::from("Error Loading Devices").bold().red(),
                    Line::from(""),
                    Line::from(err.as_str()),
                    Line::from(""),
                ];

                frame.render_widget(
                    Paragraph::new(error_text)
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            DevicesState::Loaded(devices) => {
                if devices.is_empty() {
                    frame.render_widget(
                        Paragraph::new("No devices found")
                            .block(Block::bordered())
                            .centered(),
                        content_area,
                    );
                } else {
                    self.draw_devices_list(frame, content_area, devices);
                }
            }
        }

        // Footer
        frame.render_widget(
            Paragraph::new("Press Esc to go back")
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    fn draw_devices_list(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        devices: &HashMap<String, DeviceProperties>,
    ) {
        // Convert HashMap to Vec and sort by key for consistent display
        let mut devices_vec: Vec<(&String, &DeviceProperties)> = devices.iter().collect();
        devices_vec.sort_by(|a, b| a.0.cmp(b.0));

        let items: Vec<ListItem> = devices_vec
            .iter()
            .map(|(_key, device)| {
                // Build the device info line
                let mut line_parts = vec![
                    format!("{:<20}", device.instance),
                    format!("{:<16}", device.local_ip),
                    format!("HTTP:{:<6}", device.server_port),
                    format!("gRPC:{:<6}", device.shard_port),
                ];

                // Add status indicators
                let mut status_parts = Vec::new();
                if device.is_manager {
                    status_parts.push("[MANAGER]");
                }
                if device.is_busy {
                    status_parts.push("[BUSY]");
                }
                if !status_parts.is_empty() {
                    line_parts.push(status_parts.join(" "));
                }

                let line = line_parts.join("  ");

                // Style based on status
                let style = if device.is_manager {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if device.is_busy {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };

                ListItem::new(line).style(style)
            })
            .collect();

        // Header line
        let header = format!(
            "{:<20}  {:<16}  {:<13}  {:<13}  {}",
            "INSTANCE", "IP ADDRESS", "HTTP PORT", "gRPC PORT", "STATUS"
        );

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(header)
                    .title_style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .style(Style::default());

        frame.render_widget(list, area);
    }

    pub(crate) fn handle_devices_input(&mut self, key: KeyEvent, _state: &DevicesState) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.state = AppState::Menu;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    /// Handle async operations for devices state (called during tick).
    pub(crate) async fn tick_devices(&mut self, state: &DevicesState) {
        if matches!(state, DevicesState::Loading) {
            self.load_devices().await;
        }
    }

    /// Load devices asynchronously and update state.
    async fn load_devices(&mut self) {
        match DevicesState::fetch(&self.config.api_url()).await {
            Ok(devices) => {
                self.state = AppState::Devices(DevicesState::Loaded(devices));
            }
            Err(err) => {
                self.state = AppState::Devices(DevicesState::Error(err));
            }
        }
    }
}
