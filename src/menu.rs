use crate::app::{App, AppState, TopologyState};
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
            MenuItem::ViewDevices => "Call /v1/devices",
            MenuItem::ViewTopology => "Call /v1/topology",
            MenuItem::Settings => "Edit configuration",
            MenuItem::Exit => "Quit application",
        }
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
                let content = format!("  {}  -  {}", item.label(), item.description());
                let style = if i == self.selected_menu {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(content).style(style)
            })
            .collect();

        // Calculate vertical centering for menu
        let menu_height = menu_items.len() as u16;
        let available_height = menu_area.height;
        let top_padding = (available_height.saturating_sub(menu_height)) / 2;

        // Create centered area for menu
        let centered_vertical = Layout::vertical([
            Constraint::Length(top_padding),
            Constraint::Length(menu_height),
            Constraint::Min(0),
        ]);
        let [_, centered_menu_area, _] = centered_vertical.areas(menu_area);

        let menu_list = List::new(menu_items);

        frame.render_widget(menu_list, centered_menu_area);

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
                self.state = AppState::Topology(TopologyState::Loading);
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
