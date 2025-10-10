use crate::app::{App, AppState};
use crate::topology::TopologyState;
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
    ViewDevices,
    ViewTopology,
    Settings,
    Exit,
}

impl MenuItem {
    /// Formats a menu item for display.
    pub fn fmt(&self) -> String {
        format!("{:<15}: {}", self.label(), self.description())
    }

    pub fn all() -> Vec<MenuItem> {
        vec![
            MenuItem::ViewDevices,
            MenuItem::ViewTopology,
            MenuItem::Settings,
            MenuItem::Exit,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            MenuItem::ViewDevices => "View Devices",
            MenuItem::ViewTopology => "View Topology",
            MenuItem::Settings => "Settings",
            MenuItem::Exit => "Exit",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            MenuItem::ViewDevices => "View discovered devices",
            MenuItem::ViewTopology => "View dnet topology",
            MenuItem::Settings => "Edit configuration",
            MenuItem::Exit => "Quit application",
        }
    }

    /// The total height of the menu when fully rendered.
    pub fn total_height() -> u16 {
        Self::all().len() as u16
    }

    /// The total width of the menu when fully rendered.
    pub fn total_width() -> u16 {
        Self::all()
            .iter()
            .map(|item| item.fmt().len() as u16)
            .max()
            .unwrap_or(0)
    }
}

impl App {
    pub fn draw_menu(&mut self, frame: &mut Frame) {
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
                // decide style based on selection
                let style = if i == self.selected_menu {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(item.fmt()).style(style)
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
        let menu_width = MenuItem::total_width();
        let left_padding = (vertical_centered_area.width.saturating_sub(menu_width)) / 2;
        let [_, centered_menu_area, _] = Layout::horizontal([
            Constraint::Length(left_padding),
            Constraint::Length(menu_width),
            Constraint::Min(0),
        ])
        .areas(vertical_centered_area);

        // render menu items
        frame.render_widget(List::new(menu_items), centered_menu_area);

        // Footer - no border, gray text
        let footer_text = format!("API: {}  |  Press Esc or q to quit", self.config.api_url());
        frame.render_widget(
            Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer_area,
        );
    }

    pub fn handle_menu_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
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
        let selected = MenuItem::all()[self.selected_menu];
        match selected {
            MenuItem::ViewDevices => {
                // TODO: Implement devices view
            }
            MenuItem::ViewTopology => {
                self.state = AppState::TopologyView(TopologyState::Loading);
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
}
