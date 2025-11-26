use crate::chat::{ChatState, ChatView};
use crate::common::{ApiClient, ModelInfo, TopologyInfo};
use crate::config::Config;
use crate::developer::{DeveloperState, DeveloperView};
use crate::devices::{DevicesState, DevicesView};
use crate::menu::MenuState;
use crate::model::ModelView;
use crate::settings::SettingsState;
use crate::topology::{TopologyState, TopologyView};
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

#[derive(Default, Debug)]
pub struct AppState {
    pub menu: MenuState,
    pub settings: SettingsState,
    pub devices: DevicesState,
    pub topology: TopologyState,
    pub developer: DeveloperState,
    pub chat: ChatState,
}

/// 35 FPS = 1000ms / 35
const FPS_RATE: Duration = Duration::from_millis(1000 / 35);

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
    /// Global input buffer for text inputs.
    pub input_buffer: String,
    /// Configurations.
    pub config: Config,

    pub api: ApiClient,

    /// Selected model index in load model view.
    pub selected_model: usize,

    /// Status message.
    pub status_message: String,
    /// Animation start time for sliding text.
    pub animation_start: Instant,

    /// Current topology (if present).
    pub topology: Option<TopologyInfo>,
    /// Available models.
    ///
    /// If this is empty, we treat the API to be offline.
    pub available_models: Vec<ModelInfo>,
    /// Whether the API is online.
    pub is_api_online: bool,
    /// Last time an arrow key was pressed (for ESC debouncing).
    /// See [`App::handle_crossterm_events`] for details.
    pub last_arrow_key_time: Instant,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Result<Self> {
        Self::new_at_view(AppView::Menu)
    }

    pub fn new_at_view(view: AppView) -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            is_running: false,

            api: ApiClient::new(&config.api_host, config.api_port),
            event_stream: EventStream::new(),
            config,
            view,
            state: AppState::default(),
            selected_model: 0,
            topology: None,
            is_api_online: false,
            available_models: Vec::new(),
            input_buffer: String::new(),
            status_message: String::new(),
            animation_start: Instant::now(),
            last_arrow_key_time: Instant::now(),
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
                _ => {}
            }

            // handle events with timeout to allow animation updates
            tokio::select! {
                _ = interval.tick() => {
                    // trigger a redraw for animation by looping
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
    ///
    /// TODO: separate footer and header here, and give the frame only the body area.
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        match self.view.clone() {
            AppView::Menu => self.draw_menu(frame),
            AppView::Settings => self.draw_settings(frame),
            AppView::Devices(view) => self.draw_devices(frame, &view),
            AppView::Topology(view) => self.draw_topology(frame, &view),
            AppView::Model(view) => self.draw_model(frame, &view),
            AppView::Developer(view) => self.draw_developer(frame, &view),
            AppView::Chat(view) => self.draw_chat(frame, &view),
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        use crossterm::event::{Event, KeyEventKind};
        use futures::{FutureExt, StreamExt};

        let event = self.event_stream.next().fuse().await;
        match event {
            Some(Ok(evt)) => match evt {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    use crossterm::event::KeyCode;

                    // track arrow key presses for ESC debouncing
                    if matches!(
                        key.code,
                        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
                    ) {
                        self.last_arrow_key_time = Instant::now();
                    }

                    // debounce ESC key: ignore if it comes just after an arrow key
                    // this prevents spurious ESC from arrow key escape sequences under load
                    // see: https://github.com/firstbatchxyz/dnet-tui/issues/15
                    //
                    // note that this will still cause the event queue to be filled up,
                    // which may delay other inputs, but it's a reasonable trade-off
                    if matches!(key.code, KeyCode::Esc) {
                        if Instant::now().duration_since(self.last_arrow_key_time)
                            < Duration::from_millis(50)
                        {
                            return Ok(());
                        }
                    }

                    match &self.view.clone() {
                        AppView::Menu => self.handle_menu_input(key),
                        AppView::Settings => self.handle_settings_input(key),
                        AppView::Devices(view) => self.handle_devices_input(key, view),
                        AppView::Topology(view) => self.handle_topology_input(key, view),
                        AppView::Model(view) => self.handle_model_input(key, view),
                        AppView::Developer(view) => self.handle_developer_input(key, view),
                        AppView::Chat(view) => self.handle_chat_input(key, view),
                    }
                }
                Event::Mouse(_) => {} // no mouse events
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
