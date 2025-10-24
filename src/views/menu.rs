use crate::developer::DeveloperState;
use crate::model::{LoadModelState, UnloadModelState};
use crate::topology::TopologyState;
use crate::views::topology::TopologyViewState;
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
    // ViewDevices, // TODO: add
    Chat,
    ViewTopology,
    LoadModel,
    UnloadModel,
    Settings,
    Developer,
    Exit,
}

impl MenuItem {
    /// Formats a menu item for display.
    pub fn fmt(&self, model_loaded: bool) -> String {
        format!("{:<15}: {}", self.label(), self.description(model_loaded))
    }

    pub fn all() -> Vec<MenuItem> {
        vec![
            // MenuItem::ViewDevices,
            MenuItem::Chat,
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
            // MenuItem::ViewDevices => "View Devices",
            MenuItem::Chat => "Chat",
            MenuItem::ViewTopology => "View Topology",
            MenuItem::LoadModel => "Load Model",
            MenuItem::UnloadModel => "Unload Model",
            MenuItem::Settings => "Settings",
            MenuItem::Developer => "Developer",
            MenuItem::Exit => "Exit",
        }
    }

    pub fn description(&self, model_loaded: bool) -> &str {
        match self {
            // MenuItem::ViewDevices => "View discovered devices",
            MenuItem::Chat => {
                if model_loaded {
                    "Chat with loaded model"
                } else {
                    "Chat (no model loaded)"
                }
            }
            MenuItem::ViewTopology => "View dnet topology",
            MenuItem::LoadModel => "Prepare & load a model",
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
    pub fn total_width(model_loaded: bool) -> u16 {
        Self::all()
            .iter()
            .map(|item| item.fmt(model_loaded).len() as u16)
            .max()
            .unwrap_or(0)
    }
}

impl App {
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

        // Menu items
        let menu_items: Vec<ListItem> = MenuItem::all()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // decide style based on selection and availability
                let is_disabled = (matches!(item, MenuItem::Chat | MenuItem::UnloadModel)
                    && !self.is_model_loaded);
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

                ListItem::new(item.fmt(self.is_model_loaded)).style(style)
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
        let menu_width = MenuItem::total_width(self.is_model_loaded);
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
            // MenuItem::ViewDevices => {
            //     // TODO: Implement devices view
            // }
            MenuItem::Chat => {
                if self.is_model_loaded {
                    self.state = AppState::Chat(crate::chat::ChatState::new());
                } else {
                    // if model not loaded, do nothing (item is disabled)
                }
            }
            MenuItem::ViewTopology => {
                self.state = AppState::Topology(TopologyState::Ring(TopologyViewState::Loading));
                self.selected_device = 0;
                // Trigger async topology fetch
                // Note: We'll need to handle this in the main loop
            }
            MenuItem::LoadModel => {
                self.state = AppState::Model(super::model::ModelState::Loading(
                    LoadModelState::SelectingModel,
                ));
                self.selected_model = 0;
                self.status_message.clear();
            }
            MenuItem::UnloadModel => {
                if self.is_model_loaded {
                    self.state = AppState::Model(super::model::ModelState::Unloading(
                        UnloadModelState::Unloading,
                    ));
                    self.status_message.clear();
                } else {
                    // if model not loaded, do nothing (item is disabled)
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
