use crate::config::Config;
use crate::settings::SettingsField;
use crate::topology::TopologyState;
use crossterm::event::EventStream;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum LoadModelState {
    SelectingModel,
    PreparingTopology(String /* model name */),
    LoadingModel(String /* model name */),
    Error(String),
    Success(crate::model::LoadModelResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnloadModelState {
    Unloading,
    Error(String),
    Success,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    Settings,
    TopologyView(TopologyState),
    ShardView(String /* shard name */),
    LoadModel(LoadModelState),
    UnloadModel(UnloadModelState),
}

impl AppState {
    /// Check if the state is in `Loading` topology state.
    ///
    /// This should trigger [`Self::load_topology`] in the main loop.
    pub fn is_loading_topology(&self) -> bool {
        matches!(self, Self::TopologyView(TopologyState::Loading))
    }

    /// Load topology asynchronously and update state.
    pub async fn load_topology(&mut self, api_url: &str) {
        match TopologyState::fetch(api_url).await {
            Ok(topology) => {
                *self = Self::TopologyView(TopologyState::Loaded(topology));
            }
            Err(err) => {
                *self = Self::TopologyView(TopologyState::Error(err.to_string()));
            }
        }
    }
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
            selected_model: 0,
            input_buffer: String::new(),
            status_message: String::new(),
            animation_start: Instant::now(),
        })
    }

    /// Get sliding window of text based on elapsed time
    pub fn get_sliding_text(&self, full_text: &str, window_size: usize) -> String {
        if full_text.len() <= window_size {
            // return full text if it fits
            full_text.to_string()
        } else {
            // calculate offset based on elapsed milliseconds
            let elapsed_millis = self.animation_start.elapsed().as_millis() as usize;
            let offset = (elapsed_millis / 500) % full_text.len();

            // create sliding window by cycling through the text
            // TODO: do this more performant
            let mut result = String::new();
            for i in 0..window_size {
                let idx = (offset + i) % full_text.len();
                result.push(full_text.chars().nth(idx).unwrap_or(' '));
            }
            result
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
