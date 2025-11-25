mod manual;
pub use manual::*;

mod menu;

use crossterm::event::KeyEvent;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
pub enum DeveloperView {
    Menu,
    ManualAssignment(ManualAssignmentView),
}

impl crate::App {
    pub fn draw_developer(&mut self, frame: &mut Frame, state: &DeveloperView) {
        match state {
            DeveloperView::Menu => self.draw_developer_menu(frame),
            DeveloperView::ManualAssignment(ma_state) => {
                self.draw_manual_assignment(frame, ma_state)
            }
        }
    }

    pub fn handle_developer_input(&mut self, key: KeyEvent, state: &DeveloperView) {
        match state {
            DeveloperView::Menu => self.handle_developer_menu_input(key),
            DeveloperView::ManualAssignment(ma_state) => {
                self.handle_manual_assignment_input(key, ma_state)
            }
        }
    }

    /// Handle async operations for developer state (called during tick).
    pub(crate) async fn tick_developer(&mut self, state: &DeveloperView) {
        match state {
            DeveloperView::Menu => {
                // No async operations for menu
            }
            DeveloperView::ManualAssignment(ma_state) => {
                self.tick_manual_assignment(ma_state).await
            }
        }
    }
}
