use std::time::{Duration, Instant};

use crate::developer::DeveloperView;
use crate::model::{LoadModelView, UnloadModelView};
use crate::topology::TopologyView;
use crate::views::topology::TopologyRingView;
use crate::{App, AppView};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Span;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{List, ListItem, Paragraph},
};

#[derive(Debug)]
pub struct MenuState {
    /// Selected menu item index.
    pub selection_idx: usize,
    /// Last time we checked topology in the menu
    pub last_topology_check: Instant,
    /// Last time we performed a health check
    pub last_health_check: Instant,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            selection_idx: 0,
            // make instants older to trigger immediate check
            last_topology_check: Instant::now() - Duration::from_secs(10),
            last_health_check: Instant::now() - Duration::from_secs(10),
        }
    }
}

/// A menu item.
///
/// Two things determine whether a menu item is enabled or disabled:
///
/// - Whether a model is loaded within the topology.
/// - Whether there are available models to load.
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

// TODO: smelly code here, should be much simpler

impl MenuItem {
    pub const ALL: [MenuItem; 8] = [
        MenuItem::Chat,
        MenuItem::ViewDevices,
        MenuItem::ViewTopology,
        MenuItem::LoadModel,
        MenuItem::UnloadModel,
        MenuItem::Settings,
        MenuItem::Developer,
        MenuItem::Exit,
    ];

    /// Determines if the menu item should be disabled based on current app state.
    pub fn is_disabled(
        &self,
        model_loaded: bool,
        topology_loaded: bool,
        is_api_online: bool,
    ) -> bool {
        match self {
            MenuItem::Chat => !model_loaded,
            MenuItem::LoadModel => model_loaded || !is_api_online,
            MenuItem::UnloadModel => !model_loaded,
            MenuItem::ViewTopology => !topology_loaded,
            // FIXME: we treat this as API disabled, but we should have a bool for that
            MenuItem::ViewDevices => !is_api_online,

            _ => false,
        }
    }
    /// Formats a menu item for display.
    pub fn fmt(&self, model_loaded: bool, topology_loaded: bool, is_api_online: bool) -> String {
        format!(
            "{:<15}: {}",
            self.label(),
            self.description(model_loaded, topology_loaded, is_api_online)
        )
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

    pub fn description(
        &self,
        model_loaded: bool,
        topology_loaded: bool,
        is_api_online: bool,
    ) -> &str {
        match self {
            MenuItem::Chat => {
                if model_loaded {
                    "Chat with loaded model"
                } else {
                    "Chat (no model loaded)"
                }
            }
            MenuItem::ViewDevices => {
                if is_api_online {
                    "View devices"
                } else {
                    "View devices (API unavailable)"
                }
            }
            MenuItem::ViewTopology => {
                if topology_loaded {
                    "View topology"
                } else {
                    "View topology (no topology available)"
                }
            }
            MenuItem::LoadModel => {
                if model_loaded {
                    "Load a model (model already loaded)"
                } else if is_api_online {
                    "Load a model"
                } else {
                    "Load a model (API unavailable)"
                }
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
        Self::ALL.len() as u16
    }

    /// The total width of the menu when fully rendered.
    pub fn total_width(model_loaded: bool, topology_loaded: bool, is_api_online: bool) -> u16 {
        Self::ALL
            .iter()
            .map(|item| item.fmt(model_loaded, topology_loaded, is_api_online).len() as u16)
            .max()
            .unwrap_or(0)
    }
}

impl App {
    const TOPOLOGY_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);
    const HEALTH_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

    /// Handle async operations for menu state (called during tick).
    pub(crate) async fn tick_menu(&mut self) {
        let now = std::time::Instant::now();

        // if API is offline, perform health-checks
        if !self.is_api_online {
            if now.duration_since(self.state.menu.last_health_check) >= Self::HEALTH_CHECK_INTERVAL
            {
                self.state.menu.last_health_check = now;
                self.is_api_online = self.api.is_healthy().await.unwrap_or(false);
            }
        }

        if self.is_api_online {
            // API is online, check models if we haven't fetched them yet
            if self.available_models.is_empty() {
                match self.api.get_models().await {
                    Ok(models) => self.available_models = models,
                    Err(_) => self.is_api_online = false,
                }
            }

            // check topology as well
            if now.duration_since(self.state.menu.last_topology_check)
                >= Self::TOPOLOGY_CHECK_INTERVAL
            {
                self.state.menu.last_topology_check = now;
                match self.api.get_topology().await {
                    Ok(topology) => self.topology = topology,
                    Err(_) => self.is_api_online = false,
                }
            }
        }
    }

    pub fn draw_menu(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // ASCII Art
        let ascii_art: Vec<_> = MENU_BANNER
            .map(|line| Line::from(line).centered())
            .into_iter()
            .collect();

        // Create layout
        let vertical = Layout::vertical([
            Constraint::Length(ascii_art.len() as u16), // ASCII art
            Constraint::Min(0),                         // Menu
            Constraint::Length(3),                      // Footer
        ]);
        let [art_area, menu_area, footer_area] = vertical.areas(area);

        frame.render_widget(Paragraph::new(ascii_art).centered(), art_area);

        let is_api_online = self.is_api_online;
        let is_topology_loaded = self.topology.is_some();
        let is_model_loaded = self.topology.as_ref().is_some_and(|t| t.model.is_some());

        // Menu items
        let menu_items: Vec<ListItem> = MenuItem::ALL
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // decide style based on selection and availability
                let is_disabled =
                    item.is_disabled(is_model_loaded, is_topology_loaded, is_api_online);
                let is_selected = i == self.state.menu.selection_idx;

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

                ListItem::new(item.fmt(is_model_loaded, is_topology_loaded, is_api_online))
                    .style(style)
            })
            .collect();

        // calculate vertical centering for menu
        let menu_height = MenuItem::total_height();
        let top_padding = (menu_area.height.saturating_sub(menu_height)) / 2;
        let [_, vertical_centered_area, _] = Layout::vertical([
            Constraint::Length(top_padding),
            Constraint::Length(menu_height),
            Constraint::Min(0),
        ])
        .areas(menu_area);

        // calculate horizontal centering for menu
        let menu_width = MenuItem::total_width(is_model_loaded, is_topology_loaded, is_api_online);
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
        let footer_line = Line::from_iter([
            Span::styled(
                format!("API: {} ", self.config.api_url()),
                Style::default().fg(Color::DarkGray),
            ),
            if self.is_api_online {
                Span::styled("●", Style::default().fg(Color::Green))
            } else {
                Span::styled("●", Style::default().fg(Color::Red))
            },
            Span::styled(" | Press Esc quit", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(
            Paragraph::new(footer_line)
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    pub fn handle_menu_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.quit(),
            KeyCode::Up => self.menu_up(),
            KeyCode::Down => self.menu_down(),
            KeyCode::Enter => self.select_menu_item(),
            _ => {}
        }
    }

    fn menu_up(&mut self) {
        if self.state.menu.selection_idx > 0 {
            self.state.menu.selection_idx -= 1;
        }
    }

    fn menu_down(&mut self) {
        let menu_count = MenuItem::ALL.len();
        if self.state.menu.selection_idx < menu_count - 1 {
            self.state.menu.selection_idx += 1;
        }
    }

    fn select_menu_item(&mut self) {
        let is_api_online = self.is_api_online;
        let topology_loaded = self.topology.is_some();
        let model_loaded = self.topology.as_ref().is_some_and(|t| t.model.is_some());
        match MenuItem::ALL[self.state.menu.selection_idx] {
            MenuItem::Chat => {
                // only allow entering chat if model is loaded
                if model_loaded {
                    self.view = AppView::Chat(crate::chat::ChatView::Active);
                }
            }
            MenuItem::ViewDevices => {
                if is_api_online {
                    self.view = AppView::Devices(crate::devices::DevicesView::Loading);
                }
            }
            MenuItem::ViewTopology => {
                // if topology not loaded, do nothing (item is disabled)
                if topology_loaded {
                    self.state.topology.selected_device = 0; // reset to not overflow
                    self.view = AppView::Topology(TopologyView::Ring(TopologyRingView::Loaded));
                }
            }
            MenuItem::LoadModel => {
                // if model already loaded, do nothing (item is disabled)
                if !model_loaded && is_api_online {
                    self.view = AppView::Model(super::model::ModelView::Load(
                        LoadModelView::SelectingModel,
                    ));
                    self.model_selector_state.reset();
                    self.status_message.clear();
                }
            }
            MenuItem::UnloadModel => {
                // if topology not loaded, do nothing (item is disabled)
                if model_loaded && topology_loaded {
                    self.view =
                        AppView::Model(super::model::ModelView::Unload(UnloadModelView::Unloading));
                    self.status_message.clear();
                }
            }
            MenuItem::Settings => {
                // reset settings config
                self.state.settings.temp_config = self.config.clone();
                self.status_message.clear();
                self.view = AppView::Settings;
            }
            MenuItem::Developer => {
                self.view = AppView::Developer(DeveloperView::Menu);
            }
            MenuItem::Exit => self.quit(),
        }
    }
}

/// A Dria & DNET ASCII art banner for the menu screen.
const MENU_BANNER: [&str; 18] = [
    "                                                                             ",
    "      00000    000000                                                        ",
    "   000    000000000000000   0000000000000000      000000000000          00000",
    " 000       000000   000000000   00000    00000 000    00000            000000",
    "00        00000     000000     00000    00000000     00000           00000000",
    "00       00000     0000000    00000    00000000     00000           00 000000",
    "00      00000     0000000    0000000000000  000    00000          00  0000000",
    " 000   00000     0000000    00000   000000  00    000000        000   000000 ",
    "      00000      00000     00000    00000   00   00000000      00    0000000 ",
    "     00000     000000     000000   000000    00 000000  0000000000000 00000  ",
    "    00000    0000000     00000     00000 0     000000      000        00000  ",
    " 0000000   00000       0000000    00000000  000000000    000        0000000  ",
    "",
    "",
    " ⠀⠀⣠⣤⠐⣦⡀⠀⠴⠢⣤⣄⠀⢀⠄⠀⠀⢠⣶⠂⠀⢐⠆⢀⡤⢠⣤⠂⢤",
    " ⠀⣰⡟⠀⢠⣿⠁⠀⠀⠌⢹⣿⢀⠎⠀⡄⢠⣿⠃⡴⠀⠀⠀⠊⢀⣾⠃⠀⠁",
    "⢀⣰⡟⢀⡴⠟⠁⠀⢀⠈⠀⠘⣿⠏⠀⠀⣰⣿⡁⢀⡰⠀⠀⠀⣠⣿⠃⠀⠀⠀",
    env!("CARGO_PKG_VERSION"),
];
