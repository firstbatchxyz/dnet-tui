use crate::developer::DeveloperState;
use crate::model::{LoadModelState, UnloadModelState};
use crate::topology::TopologyState;
use crate::views::topology::TopologyRingState;
use crate::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{List, ListItem, Paragraph},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    Chat,
    ViewDevices,
    ViewTopology,
    LoadModel,
    UnloadModel,
    Settings,
    Developer,
    Exit,
}

impl MenuItem {
    /// Determines if the menu item should be disabled based on current app state.
    pub fn is_disabled(&self, model_loaded: bool, topology_loaded: bool) -> bool {
        match self {
            MenuItem::Chat => !model_loaded,
            // MenuItem::LoadModel => model_loaded,
            MenuItem::UnloadModel => !model_loaded,

            MenuItem::ViewTopology => !topology_loaded,

            _ => false,
        }
    }
    /// Formats a menu item for display.
    pub fn fmt(&self, model_loaded: bool, topology_loaded: bool) -> String {
        format!(
            "{:<15}: {}",
            self.label(),
            self.description(model_loaded, topology_loaded)
        )
    }

    pub fn all() -> Vec<MenuItem> {
        vec![
            MenuItem::Chat,
            MenuItem::ViewDevices,
            MenuItem::ViewTopology,
            MenuItem::LoadModel,
            MenuItem::UnloadModel,
            MenuItem::Settings,
            MenuItem::Developer,
            MenuItem::Exit,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            MenuItem::Chat => "Chat",
            MenuItem::ViewDevices => "View Devices",
            MenuItem::ViewTopology => "View Topology",
            MenuItem::LoadModel => "Load Model",
            MenuItem::UnloadModel => "Unload Model",
            MenuItem::Settings => "Settings",
            MenuItem::Developer => "Developer",
            MenuItem::Exit => "Exit",
        }
    }

    pub fn description(&self, model_loaded: bool, topology_loaded: bool) -> &str {
        match self {
            MenuItem::Chat => {
                if model_loaded {
                    "Chat with loaded model"
                } else {
                    "Chat (no model loaded)"
                }
            }
            MenuItem::ViewDevices => "View discovered devices",
            MenuItem::ViewTopology => {
                if topology_loaded {
                    "View dnet topology"
                } else {
                    "View topology (no topology available)"
                }
            }
            MenuItem::LoadModel => {
                "Load a model"
                // FIXME: !!!
                // if model_loaded {
                //     "Load a model (model already loaded)"
                // } else {
                //     "Load a model"
                // }
            }
            MenuItem::UnloadModel => {
                if model_loaded {
                    "Unload model"
                } else {
                    "Unload model (no model loaded)"
                }
            }
            MenuItem::Settings => "Edit configuration",
            MenuItem::Developer => "Advanced developer tools",
            MenuItem::Exit => "Quit application",
        }
    }

    /// The total height of the menu when fully rendered.
    pub fn total_height() -> u16 {
        Self::all().len() as u16
    }

    /// The total width of the menu when fully rendered.
    pub fn total_width(model_loaded: bool, topology_loaded: bool) -> u16 {
        Self::all()
            .iter()
            .map(|item| item.fmt(model_loaded, topology_loaded).len() as u16)
            .max()
            .unwrap_or(0)
    }
}

impl App {
    /// Handle async operations for menu state (called during tick).
    pub(crate) async fn tick_menu(&mut self) {
        // Check topology every few seconds to avoid excessive API calls
        const TOPOLOGY_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

        let now = std::time::Instant::now();
        if now.duration_since(self.last_topology_check) >= TOPOLOGY_CHECK_INTERVAL {
            self.last_topology_check = now;

            match crate::common::TopologyInfo::fetch(&self.config.api_url()).await {
                Ok(topology) => {
                    self.topology = Some(topology);
                }
                Err(_) => {
                    // If topology fetch fails, clear it (model not loaded or not configured)
                    self.topology = None;
                }
            }
        }
    }

    pub fn draw_menu(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // ASCII Art
        let ascii_art: Vec<_> = crate::constants::MENU_BANNER
            .map(|line| Line::from(line).centered())
            .into_iter()
            .collect();

        // Create layout
        let vertical = Layout::vertical([
            Constraint::Length(ascii_art.len() as u16), // ASCII art
            Constraint::Min(0),                         // Menu
            Constraint::Length(1),                      // Footer
        ]);
        let [art_area, menu_area, footer_area] = vertical.areas(area);

        frame.render_widget(Paragraph::new(ascii_art).centered(), art_area);

        let is_topology_loaded = self.topology.is_some();
        let is_model_loaded = self.topology.as_ref().is_some_and(|t| t.model.is_some());

        // Menu items
        let menu_items: Vec<ListItem> = MenuItem::all()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // decide style based on selection and availability
                let is_disabled = item.is_disabled(is_model_loaded, is_topology_loaded);
                let is_selected = i == self.selected_menu;

                let style = match (is_selected, is_disabled) {
                    // selected & disable
                    (true, true) => Style::default()
                        .fg(Color::DarkGray)
                        .bg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                    // selected & available
                    (true, false) => Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    // not selected & disabled
                    (false, true) => Style::default().fg(Color::DarkGray),
                    // not selected & available
                    (false, false) => Style::default(),
                };

                ListItem::new(item.fmt(is_model_loaded, is_topology_loaded)).style(style)
            })
            .collect();

        // Calculate vertical centering for menu
        let menu_height = MenuItem::total_height();
        let top_padding = (menu_area.height.saturating_sub(menu_height)) / 2;
        let [_, vertical_centered_area, _] = Layout::vertical([
            Constraint::Length(top_padding),
            Constraint::Length(menu_height),
            Constraint::Min(0),
        ])
        .areas(menu_area);

        // Calculate horizontal centering for menu
        let menu_width = MenuItem::total_width(is_model_loaded, is_topology_loaded);
        let left_padding = (vertical_centered_area.width.saturating_sub(menu_width)) / 2;
        let [_, centered_menu_area, _] = Layout::horizontal([
            Constraint::Length(left_padding),
            Constraint::Length(menu_width),
            Constraint::Min(0),
        ])
        .areas(vertical_centered_area);

        // render menu items
        frame.render_widget(List::new(menu_items), centered_menu_area);

        // Footer
        let footer_text = format!("API: {}  |  Press Esc quit", self.config.api_url());
        frame.render_widget(
            Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    pub fn handle_menu_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up) => self.menu_up(),
            (_, KeyCode::Down) => self.menu_down(),
            (_, KeyCode::Enter) => self.select_menu_item(),
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
        match MenuItem::all()[self.selected_menu] {
            MenuItem::Chat => {
                if let Some(model) = &self.topology.as_ref().and_then(|t| t.model.clone()) {
                    self.state = AppState::Chat(crate::chat::ChatState::new(
                        model.clone(),
                        self.config.max_tokens,
                    ));
                } else {
                    // if topology not loaded, do nothing (item is disabled)
                }
            }
            MenuItem::ViewDevices => {
                self.state = AppState::Devices(crate::devices::DevicesState::Loading);
            }
            MenuItem::ViewTopology => {
                if self.topology.is_some() {
                    self.state = AppState::Topology(TopologyState::Ring(TopologyRingState::Loaded));
                    self.selected_device = 0;
                } else {
                    // if topology not loaded, do nothing (item is disabled)
                }
            }
            MenuItem::LoadModel => {
                if self.topology.as_ref().is_some_and(|t| t.model.is_some()) {
                    // if model already loaded, do nothing (item is disabled)
                } else {
                    self.state = AppState::Model(super::model::ModelState::Load(
                        LoadModelState::SelectingModel,
                    ));
                    self.selected_model = 0;
                    self.status_message.clear();
                }
            }
            MenuItem::UnloadModel => {
                if self.topology.is_some() {
                    self.state = AppState::Model(super::model::ModelState::Unload(
                        UnloadModelState::Unloading,
                    ));
                    self.status_message.clear();
                } else {
                    // if topology not loaded, do nothing (item is disabled)
                }
            }
            MenuItem::Settings => {
                self.state = AppState::Settings;
                self.temp_config = self.config.clone();
                self.status_message.clear();
            }
            MenuItem::Developer => {
                self.state = AppState::Developer(DeveloperState::Menu);
                self.developer_menu_index = 0;
            }
            MenuItem::Exit => self.quit(),
        }
    }
}
