use crate::chat::ChatState;
use crate::config::Config;
use crate::developer::DeveloperState;
use crate::model::ModelState;
use crate::settings::SettingsField;
use crate::topology::TopologyState;
use crossterm::event::EventStream;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    Settings,
    Topology(TopologyState),
    Model(ModelState),
    Developer(DeveloperState),
    Chat(ChatState),
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
    /// Pending chat message to send
    pub pending_chat_message: Option<String>,
    /// Chat message receiver for streaming responses
    pub chat_stream_rx: Option<mpsc::UnboundedReceiver<String>>,
    /// Whether a model is currently loaded
    pub model_loaded: bool,
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
            selected_model: 0,
            developer_menu_index: 0,
            input_buffer: String::new(),
            status_message: String::new(),
            animation_start: Instant::now(),
            pending_chat_message: None,
            chat_stream_rx: None,
            model_loaded: false,
        })
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
