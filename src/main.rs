mod config;
mod topology;

use color_eyre::Result;
use config::Config;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Styled, Stylize},
    text::Line,
    widgets::{
        Block, List, ListItem, Paragraph,
        canvas::{Canvas, Circle, Line as CanvasLine, Points},
    },
};
use std::time::{Duration, Instant};
use topology::TopologyResponse;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new()?.run(terminal).await;
    ratatui::restore();
    result
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    Settings,
    Topology(TopologyState),
    ShardInteraction(String /* shard name */),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyState {
    Loading,
    Loaded(TopologyResponse),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    ViewDevices,
    ViewTopology,
    Settings,
    Exit,
}

impl MenuItem {
    fn all() -> Vec<MenuItem> {
        vec![
            MenuItem::ViewDevices,
            MenuItem::ViewTopology,
            MenuItem::Settings,
            MenuItem::Exit,
        ]
    }

    fn label(&self) -> &str {
        match self {
            MenuItem::ViewDevices => "View Devices",
            MenuItem::ViewTopology => "View Topology",
            MenuItem::Settings => "Settings",
            MenuItem::Exit => "Exit",
        }
    }

    fn description(&self) -> &str {
        match self {
            MenuItem::ViewDevices => "Call /v1/devices",
            MenuItem::ViewTopology => "Call /v1/topology",
            MenuItem::Settings => "Edit configuration",
            MenuItem::Exit => "Quit application",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsField {
    Host,
    Port,
}

#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Event stream.
    event_stream: EventStream,
    /// Configuration.
    config: Config,
    /// Temporary config for editing.
    temp_config: Config,
    /// Current application state.
    state: AppState,
    /// Selected menu item index.
    selected_menu: usize,
    /// Selected settings field.
    selected_field: SettingsField,
    /// Selected device index in topology view.
    selected_device: usize,
    /// Input buffer for editing.
    input_buffer: String,
    /// Status message.
    status_message: String,
    /// Animation start time for sliding text.
    animation_start: Instant,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            running: false,
            event_stream: EventStream::new(),
            temp_config: config.clone(),
            config,
            state: AppState::Menu,
            selected_menu: 0,
            selected_field: SettingsField::Host,
            selected_device: 0,
            input_buffer: String::new(),
            status_message: String::new(),
            animation_start: Instant::now(),
        })
    }

    /// Get sliding window of text based on elapsed time
    fn get_sliding_text(&self, full_text: &str, window_size: usize) -> String {
        if full_text.len() <= window_size {
            return full_text.to_string();
        }

        // Calculate offset based on elapsed milliseconds
        let elapsed_millis = self.animation_start.elapsed().as_millis() as usize;
        let offset = (elapsed_millis / 500) % full_text.len();

        // Create sliding window by cycling through the text
        let mut result = String::new();
        for i in 0..window_size {
            let idx = (offset + i) % full_text.len();
            result.push(full_text.chars().nth(idx).unwrap_or(' '));
        }
        result
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;

        // Create a ticker for animation updates (60 FPS for smooth animation)
        let mut interval = tokio::time::interval(Duration::from_millis(16));

        while self.running {
            // Check if we need to load topology
            if matches!(self.state, AppState::Topology(TopologyState::Loading)) {
                let api_url = self.config.api_url();
                match TopologyResponse::fetch(&api_url).await {
                    Ok(topology) => {
                        self.state = AppState::Topology(TopologyState::Loaded(topology));
                    }
                    Err(e) => {
                        self.state = AppState::Topology(TopologyState::Error(e.to_string()));
                    }
                }
            }

            terminal.draw(|frame| self.draw(frame))?;

            // Handle events with timeout to allow animation updates
            tokio::select! {
                _ = interval.tick() => {
                    // Just trigger a redraw for animation
                }
                result = self.handle_crossterm_events() => {
                    result?;
                }
            }
        }
        Ok(())
    }

    /// Renders the user interface.
    fn draw(&mut self, frame: &mut Frame) {
        match self.state.clone() {
            AppState::Menu => self.draw_menu(frame),
            AppState::Settings => self.draw_settings(frame),
            AppState::Topology(state) => self.draw_topology(frame, &state),
            AppState::ShardInteraction(device) => self.draw_shard_interaction(frame, &device),
        }
    }

    fn draw_menu(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Create layout
        let vertical = Layout::vertical([
            Constraint::Length(11), // ASCII art (no border)
            Constraint::Min(0),     // Menu
            Constraint::Length(1),  // Footer (no border)
        ]);
        let [art_area, menu_area, footer_area] = vertical.areas(area);

        // ASCII Art
        #[rustfmt::skip]
        let ascii_art = vec![
          Line::from("                                                                             "),
          Line::from("      00000    000000                                                        "),
          Line::from("   000    000000000000000   0000000000000000      000000000000          00000"),
          Line::from(" 000       000000   000000000   00000    00000 000    00000            000000"),
          Line::from("00        00000     000000     00000    00000000     00000           00000000"),
          Line::from("00       00000     0000000    00000    00000000     00000           00 000000"),
          Line::from("00      00000     0000000    0000000000000  000    00000          00  0000000"),
          Line::from(" 000   00000     0000000    00000   000000  00    000000        000   000000 "),
          Line::from("      00000      00000     00000    00000   00   00000000      00    0000000 "),
          Line::from("     00000     000000     000000   000000    00 000000  0000000000000 00000  "),
          Line::from("    00000    0000000     00000     00000 0     000000      000        00000  "),
          Line::from(" 0000000   00000       0000000    00000000  000000000    000        0000000  "),
          Line::from("                                                                             "),
        ];

        frame.render_widget(Paragraph::new(ascii_art).centered(), art_area);

        // Menu items
        let menu_items: Vec<ListItem> = MenuItem::all()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = format!("  {}  -  {}", item.label(), item.description());
                let style = if i == self.selected_menu {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(content).style(style)
            })
            .collect();

        // Calculate vertical centering for menu
        let menu_height = menu_items.len() as u16;
        let available_height = menu_area.height;
        let top_padding = (available_height.saturating_sub(menu_height)) / 2;

        // Create centered area for menu
        let centered_vertical = Layout::vertical([
            Constraint::Length(top_padding),
            Constraint::Length(menu_height),
            Constraint::Min(0),
        ]);
        let [_, centered_menu_area, _] = centered_vertical.areas(menu_area);

        let menu_list = List::new(menu_items);

        frame.render_widget(menu_list, centered_menu_area);

        // Footer - no border, gray text
        let footer_text = format!("API: {}  |  Press Esc or q to quit", self.config.api_url());
        frame.render_widget(
            Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    fn draw_topology(&mut self, frame: &mut Frame, state: &TopologyState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let title = Line::from("Topology Ring View").bold().blue().centered();
        frame.render_widget(Paragraph::new(title).block(Block::bordered()), title_area);

        // Content
        match state {
            TopologyState::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading topology...")
                        .block(Block::bordered())
                        .centered(),
                    content_area,
                );
            }
            TopologyState::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::bordered())
                        .style(Style::default().fg(Color::Red))
                        .centered(),
                    content_area,
                );
            }
            TopologyState::Loaded(topology) => {
                self.draw_topology_ring(frame, content_area, topology);
            }
        }

        // Footer
        let footer_text = match state {
            TopologyState::Loaded(_) => {
                "Use ↑↓ to select device  |  Enter to interact  |  Esc to go back"
            }
            _ => "Press Esc to go back",
        };
        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::bordered())
                .centered(),
            footer_area,
        );
    }

    fn draw_topology_ring(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        topology: &TopologyResponse,
    ) {
        use std::f64::consts::PI;

        let num_devices = topology.devices.len();
        if num_devices == 0 {
            frame.render_widget(
                Paragraph::new("No devices in topology")
                    .block(Block::bordered())
                    .centered(),
                area,
            );
            return;
        }

        // Calculate circle parameters for canvas
        let radius = 35.0;
        let center_x = 0.0;
        let center_y = 0.0;

        // Prepare device data for drawing
        struct DeviceInfo {
            x: f64,
            y: f64,
            name: String,
            ip: String,
            layers: String,
            is_selected: bool,
        }

        let mut devices_info = Vec::new();

        for (i, device) in topology.devices.iter().enumerate() {
            let angle = 2.0 * PI * (i as f64) / (num_devices as f64) - PI / 2.0; // Start from top
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();

            // Get full device name (remove "shard-" prefix)
            let full_name = device
                .instance
                .strip_prefix("shard-")
                .unwrap_or(&device.instance)
                .to_string();

            // Apply sliding window animation to device name
            let short_name = self.get_sliding_text(&full_name, 30);

            // Get IP and GRPC port
            let ip = format!("{}:{}", device.local_ip, device.shard_port);

            // Get layer assignments
            let layers = topology
                .assignments
                .iter()
                .find(|a| a.service == device.instance)
                .map(|a| TopologyResponse::format_layers(&a.layers))
                .unwrap_or_else(|| "[]".to_string());

            let is_selected = i == self.selected_device;

            devices_info.push(DeviceInfo {
                x,
                y,
                name: short_name,
                ip,
                layers,
                is_selected,
            });
        }

        // Clone for use in canvas closure
        let devices_clone = devices_info
            .iter()
            .map(|d| {
                (
                    d.x,
                    d.y,
                    d.name.clone(),
                    d.ip.clone(),
                    d.layers.clone(),
                    d.is_selected,
                )
            })
            .collect::<Vec<_>>();

        let model_info = format!(
            "Model: {}  |  Layers: {}",
            topology.model, topology.num_layers
        );

        // Draw canvas with ring
        let canvas = Canvas::default()
            .block(Block::bordered().title(model_info))
            .x_bounds([-60.0, 60.0])
            .y_bounds([-60.0, 60.0])
            .paint(move |ctx| {
                // Draw the circle
                ctx.draw(&Circle {
                    x: center_x,
                    y: center_y,
                    radius,
                    color: Color::Cyan,
                });

                // Draw connection lines between devices
                for i in 0..devices_clone.len() {
                    let (x1, y1, _, _, _, _) = devices_clone[i];
                    let next_i = (i + 1) % devices_clone.len();
                    let (x2, y2, _, _, _, _) = devices_clone[next_i];

                    ctx.draw(&CanvasLine {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: Color::DarkGray,
                    });
                }

                // Draw devices with their info
                for (x, y, name, ip, layers, is_selected) in devices_clone.iter() {
                    // Draw device point with larger size if selected
                    let color = if *is_selected {
                        Color::Yellow
                    } else {
                        Color::Green
                    };

                    // Draw a larger point for better visibility
                    ctx.draw(&Points {
                        coords: &[(*x, *y)],
                        color,
                    });

                    // If selected, draw additional points around it to make it stand out
                    if *is_selected {
                        ctx.draw(&Points {
                            coords: &[
                                (*x + 0.5, *y),
                                (*x - 0.5, *y),
                                (*x, *y + 0.5),
                                (*x, *y - 0.5),
                            ],
                            color: Color::Yellow,
                        });
                    }

                    // Calculate text offset based on position to avoid overlap with circle
                    let text_offset = 5.0;
                    let angle = y.atan2(*x);
                    let text_x = x + text_offset * angle.cos();
                    let text_y = y + text_offset * angle.sin();

                    // Draw device info: name, IP, layers (each on a separate line)
                    // Highlight text in yellow if selected
                    if *is_selected {
                        ctx.print(text_x, text_y + 3.0, name.clone().yellow());
                        ctx.print(text_x, text_y, ip.clone().yellow());
                        ctx.print(text_x, text_y - 3.0, layers.clone().yellow());
                    } else {
                        ctx.print(text_x, text_y + 3.0, name.clone());
                        ctx.print(text_x, text_y, ip.clone());
                        ctx.print(text_x, text_y - 3.0, layers.clone());
                    }
                }
            });

        frame.render_widget(canvas, area);
    }

    fn draw_shard_interaction(&mut self, frame: &mut Frame, device: &str) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ]);
        let [title_area, content_area, footer_area] = vertical.areas(area);

        // Title
        let short_name = TopologyResponse::device_short_name(device);
        let title = Line::from(format!("Shard Interaction: {}", short_name))
            .bold()
            .blue()
            .centered();
        frame.render_widget(Paragraph::new(title).block(Block::bordered()), title_area);

        // Content - Placeholder for now
        let content = vec![
            Line::from(""),
            Line::from(format!("Device: {}", device)).bold(),
            Line::from(""),
            Line::from("This window will allow you to:"),
            Line::from("  • Send GET/POST requests to this shard"),
            Line::from("  • View shard information"),
            Line::from("  • Test connectivity"),
            Line::from(""),
            Line::from("Coming soon...").dim(),
        ];

        frame.render_widget(
            Paragraph::new(content).block(Block::bordered().title("Shard Communication")),
            content_area,
        );

        // Footer
        frame.render_widget(
            Paragraph::new("Press Esc to go back to topology")
                .block(Block::bordered())
                .centered(),
            footer_area,
        );
    }

    fn draw_settings(&mut self, frame: &mut Frame) {
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

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        let event = self.event_stream.next().fuse().await;
        match event {
            Some(Ok(evt)) => match evt {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match &self.state {
            AppState::Menu => self.handle_menu_input(key),
            AppState::Settings => self.handle_settings_input(key),
            AppState::Topology(_) => self.handle_topology_input(key),
            AppState::ShardInteraction(_) => self.handle_shard_interaction_input(key),
        }
    }

    fn handle_menu_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up) => self.menu_up(),
            (_, KeyCode::Down) => self.menu_down(),
            (_, KeyCode::Enter) => self.select_menu_item(),
            _ => {}
        }
    }

    fn handle_topology_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.state = AppState::Menu;
                self.selected_device = 0; // Reset selection
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up) => self.topology_device_up(),
            (_, KeyCode::Down) => self.topology_device_down(),
            (_, KeyCode::Enter) => self.open_shard_interaction(),
            _ => {}
        }
    }

    fn handle_shard_interaction_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                // Go back to topology view - restore the topology state
                if let AppState::ShardInteraction(_) = &self.state {
                    // We need to restore the topology - for now go back to menu
                    // TODO: Keep topology state when entering shard interaction
                    self.state = AppState::Menu;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) {
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

    fn menu_up(&mut self) {
        if self.selected_menu > 0 {
            self.selected_menu -= 1;
        }
    }

    fn menu_down(&mut self) {
        let menu_count = MenuItem::all().len();
        if self.selected_menu < menu_count - 1 {
            self.selected_menu += 1;
        }
    }

    fn select_menu_item(&mut self) {
        let selected = MenuItem::all()[self.selected_menu];
        match selected {
            MenuItem::ViewDevices => {
                // TODO: Implement devices view
            }
            MenuItem::ViewTopology => {
                self.state = AppState::Topology(TopologyState::Loading);
                self.selected_device = 0;
                // Trigger async topology fetch
                // Note: We'll need to handle this in the main loop
            }
            MenuItem::Settings => {
                self.state = AppState::Settings;
                self.temp_config = self.config.clone();
                self.status_message.clear();
            }
            MenuItem::Exit => self.quit(),
        }
    }

    fn topology_device_up(&mut self) {
        if let AppState::Topology(TopologyState::Loaded(topology)) = &self.state {
            let device_count = topology.devices.len();
            if device_count > 0 {
                // Cycle: if at 0, wrap to last device
                if self.selected_device == 0 {
                    self.selected_device = device_count - 1;
                } else {
                    self.selected_device -= 1;
                }
            }
        }
    }

    fn topology_device_down(&mut self) {
        if let AppState::Topology(TopologyState::Loaded(topology)) = &self.state {
            let device_count = topology.devices.len();
            if device_count > 0 {
                // Cycle: if at last, wrap to 0
                self.selected_device = (self.selected_device + 1) % device_count;
            }
        }
    }

    fn open_shard_interaction(&mut self) {
        if let AppState::Topology(TopologyState::Loaded(topology)) = &self.state {
            if let Some(device) = topology.devices.get(self.selected_device) {
                self.state = AppState::ShardInteraction(device.instance.clone());
            }
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

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
