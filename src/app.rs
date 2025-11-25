use crate::chat::{ChatActiveState, ChatView};
use crate::common::{ModelInfo, TopologyInfo};
use crate::config::Config;
use crate::developer::DeveloperView;
use crate::devices::DevicesView;
use crate::menu::MenuState;
use crate::model::ModelView;
use crate::settings::SettingsState;
use crate::topology::TopologyView;
use color_eyre::eyre::Result;
use crossterm::event::EventStream;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum AppView {
    Menu,
    Settings,
    Devices(DevicesView),
    Topology(TopologyView),
    Model(ModelView),
    Developer(DeveloperView),
    Chat(ChatView),
}

#[derive(Default, Debug, Clone)]
pub struct AppState {
    pub settings: SettingsState,
    pub menu: MenuState,
}

/// 60 FPS = 1000ms / 60 = 16.67ms per frame
const FPS_RATE: Duration = Duration::from_millis(1000 / 60);

#[derive(Debug)]
pub struct App {
    /// Active application view.
    pub view: AppView,
    /// Application state.
    ///
    /// This is shared among all views.
    pub state: AppState,
    /// Is the application running?
    pub is_running: bool,
    /// Event stream.
    pub event_stream: EventStream,
    /// Configuration.
    pub config: Config,
    /// Temporary config for editing.
    /// TODO: move to settings
    pub temp_config: Config,

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
    /// Active chat state, persistent across chat sessions.
    pub chat: ChatActiveState,
    /// Current topology (if present).
    pub topology: Option<TopologyInfo>,
    /// Available models fetched from API
    pub available_models: Vec<ModelInfo>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Result<Self> {
        Self::new_with_state(AppView::Menu)
    }

    pub fn new_with_state(state: AppView) -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            is_running: false,
            event_stream: EventStream::new(),
            temp_config: config.clone(),
            config,
            view: state,
            state: AppState::default(),
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
            chat: ChatActiveState::new(),
            topology: None,
            available_models: Vec::new(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: ratatui::DefaultTerminal) -> Result<()> {
        self.is_running = true;

        // create a ticker for animation updates
        let mut interval = tokio::time::interval(FPS_RATE);

        while self.is_running {
            // draw first (to disguise async stuff in ticks)
            terminal.draw(|frame| self.draw(frame))?;

            // process ticks
            match self.view.clone() {
                AppView::Menu => {
                    self.tick_menu().await;
                }
                AppView::Devices(devices_state) => {
                    self.tick_devices(&devices_state).await;
                }
                AppView::Topology(topology_state) => {
                    self.tick_topology(&topology_state).await;
                }
                AppView::Model(model_state) => {
                    self.tick_model(&model_state).await;
                }
                AppView::Developer(developer_state) => {
                    self.tick_developer(&developer_state).await;
                }
                AppView::Chat(chat_state) => {
                    self.tick_chat(&chat_state).await;
                }
                _ => {
                    // no async operations for Settings
                }
            }

            // handle events with timeout to allow animation updates
            tokio::select! {
                _ = interval.tick() => {
                    // will trigger a redraw for animation by looping
                    continue;
                }
                result = self.handle_crossterm_events() => {
                    result?;
                }
            }
        }
        Ok(())
    }

    /// Renders the user interface.
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        match self.view.clone() {
            AppView::Menu => self.draw_menu(frame),
            AppView::Settings => self.draw_settings(frame),
            AppView::Devices(state) => self.draw_devices(frame, &state),
            AppView::Topology(state) => self.draw_topology(frame, &state),
            AppView::Model(state) => self.draw_model(frame, &state),
            AppView::Developer(state) => self.draw_developer(frame, &state),
            AppView::Chat(state) => self.draw_chat(frame, &state),
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        use crossterm::event::{Event, KeyEventKind};
        use futures::{FutureExt, StreamExt};

        let event = self.event_stream.next().fuse().await;
        match event {
            Some(Ok(evt)) => match evt {
                Event::Key(key) if key.kind == KeyEventKind::Press => match &self.view.clone() {
                    AppView::Menu => self.handle_menu_input(key),
                    AppView::Settings => self.handle_settings_input(key),
                    AppView::Devices(state) => self.handle_devices_input(key, state),
                    AppView::Topology(state) => self.handle_topology_input(key, state),
                    AppView::Model(state) => self.handle_model_input(key, state),
                    AppView::Developer(state) => self.handle_developer_input(key, state),
                    AppView::Chat(state) => self.handle_chat_input(key, state),
                },
                Event::Mouse(_) => {} // TODO: do we want mouse events?
                Event::Resize(_, _) => {}
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.is_running = false;
    }
}
