use crate::config::Config;
use crate::topology::TopologyResponse;
use crossterm::event::EventStream;
use std::time::Instant;

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
pub enum SettingsField {
    Host,
    Port,
}

#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event stream.
    pub event_stream: EventStream,
    /// Configuration.
    pub config: Config,
    /// Temporary config for editing.
    pub temp_config: Config,
    /// Current application state.
    pub state: AppState,
    /// Selected menu item index.
    pub selected_menu: usize,
    /// Selected settings field.
    pub selected_field: SettingsField,
    /// Selected device index in topology view.
    pub selected_device: usize,
    /// Input buffer for editing.
    pub input_buffer: String,
    /// Status message.
    pub status_message: String,
    /// Animation start time for sliding text.
    pub animation_start: Instant,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> color_eyre::Result<Self> {
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
    pub fn get_sliding_text(&self, full_text: &str, window_size: usize) -> String {
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

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
