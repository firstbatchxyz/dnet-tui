use crate::chat::ChatState;
use crate::common::TopologyInfo;
use crate::config::Config;
use crate::developer::DeveloperState;
use crate::devices::DevicesState;
use crate::model::ModelState;
use crate::settings::SettingsField;
use crate::topology::TopologyState;
use crossterm::event::EventStream;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    Settings,
    Devices(DevicesState),
    Topology(TopologyState),
    Model(ModelState),
    Developer(DeveloperState),
    Chat(ChatState),
}

#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub is_running: bool,
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
    /// Selected model index in load model view.
    pub selected_model: usize,
    /// Selected developer menu index.
    pub developer_menu_index: usize,
    /// Input buffer for editing.
    pub input_buffer: String,
    /// Status message.
    pub status_message: String,
    /// Animation start time for sliding text.
    pub animation_start: Instant,
    /// Last time we checked topology in the menu
    pub last_topology_check: Instant,
    /// Last time we refreshed devices
    pub last_devices_refresh: Instant,
    /// Pending chat message to send
    pub pending_chat_message: Option<String>,
    /// Chat message receiver for streaming responses
    pub chat_stream_rx: Option<mpsc::UnboundedReceiver<String>>,
    /// Current topology (if present).
    pub topology: Option<TopologyInfo>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            is_running: false,
            event_stream: EventStream::new(),
            temp_config: config.clone(),
            config,
            state: AppState::Menu,
            selected_menu: 0,
            selected_field: SettingsField::Host,
            selected_device: 0,
            selected_model: 0,
            developer_menu_index: 0,
            input_buffer: String::new(),
            status_message: String::new(),
            animation_start: Instant::now(),
            // make this older to trigger immediate check
            last_topology_check: Instant::now() - Duration::from_secs(10),
            // make this older to trigger immediate refresh
            last_devices_refresh: Instant::now() - Duration::from_secs(10),
            pending_chat_message: None,
            chat_stream_rx: None,
            topology: None,
        })
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.is_running = false;
    }
}
