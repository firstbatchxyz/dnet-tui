use crate::common::DeviceProperties;
use crate::{App, app::AppView};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct DevicesState {
    /// Last time we refreshed devices.
    pub refreshed_at: Instant,
}

impl Default for DevicesState {
    fn default() -> Self {
        Self {
            // make this older to trigger immediate refresh
            refreshed_at: Instant::now() - Duration::from_secs(10),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum DevicesView {
    Loading,
    Loaded(HashMap<String, DeviceProperties>),
    Error(String),
}

impl App {
    pub(crate) fn draw_devices(&mut self, frame: &mut Frame, view: &DevicesView) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Discovered Devices").bold().cyan().centered();
        frame.render_widget(Paragraph::new(title), title_area);

        // Content
        match view {
            DevicesView::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading devices...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            DevicesView::Error(err) => {
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
            DevicesView::Loaded(devices) => {
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
            Paragraph::new("Press Esc to go back").centered().gray(),
            footer_area,
        );
    }

    fn draw_devices_list(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        devices: &HashMap<String, DeviceProperties>,
    ) {
        // Convert HashMap to Vec and sort by local IP
        let mut devices_vec: Vec<(&String, &DeviceProperties)> = devices.iter().collect();
        devices_vec.sort_by(|a, b| {
            format!("{}:{}", a.1.local_ip, a.1.server_port)
                .cmp(&format!("{}:{}", b.1.local_ip, b.1.server_port))
        });

        // Create table headers
        let header = Row::new(vec![
            Cell::from("Instance").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("IP Address").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("HTTP Port").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("gRPC Port").style(Style::default().add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().fg(Color::Yellow))
        .bottom_margin(1);

        // Create table rows
        let rows: Vec<Row> = devices_vec
            .iter()
            .map(|(_key, device)| {
                // Determine row style based on status
                let style = if device.is_manager {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if device.is_busy {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };

                Row::new(vec![
                    Cell::from(device.instance.clone()),
                    Cell::from(device.local_ip.clone()),
                    Cell::from(device.server_port.to_string()),
                    Cell::from(device.shard_port.to_string()),
                ])
                .style(style)
            })
            .collect();

        // create table with widths
        let widths = [
            Constraint::Percentage(56), // Instance
            Constraint::Percentage(24), // IP Address
            Constraint::Percentage(10), // HTTP Port
            Constraint::Percentage(10), // gRPC Port
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::bordered()
                    .title(format!("{} Devices", devices.len()))
                    .title_style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .column_spacing(1);

        frame.render_widget(table, area);
    }

    pub(crate) fn handle_devices_input(&mut self, key: KeyEvent, _view: &DevicesView) {
        if key.code == KeyCode::Esc {
            self.view = AppView::Menu;
        }
    }

    /// Handle async operations for devices state (called during tick).
    pub(crate) async fn tick_devices(&mut self, view: &DevicesView) {
        use std::time::Duration;

        let refresh_interval = Duration::from_secs(self.config.devices_refresh_interval);
        let should_refresh = self.state.devices.refreshed_at.elapsed() >= refresh_interval;

        // Refresh if loading or if refresh interval has elapsed
        if matches!(view, DevicesView::Loading) || should_refresh {
            self.load_devices().await;
        }
    }

    /// Load devices asynchronously and update state.
    async fn load_devices(&mut self) {
        use std::time::Instant;

        match self.api.get_devices().await {
            Ok(devices) => {
                self.view = AppView::Devices(DevicesView::Loaded(devices));
            }
            Err(err) => {
                self.view = AppView::Devices(DevicesView::Error(err.to_string()));
            }
        };

        self.state.devices.refreshed_at = Instant::now();
    }
}
