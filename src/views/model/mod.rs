mod load;
pub use load::*;

mod unload;
pub use unload::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelState {
    Load(LoadModelState),
    Unload(UnloadModelState),
}

impl crate::App {
    /// Handle async operations for model state (called during tick).
    pub(crate) async fn tick_model(&mut self, state: &ModelState) {
        match state {
            ModelState::Load(load_state) => self.tick_load_model(load_state).await,
            ModelState::Unload(unload_state) => self.tick_unload_model(unload_state).await,
        }
    }

    /// Draw model state.
    pub(crate) fn draw_model(&mut self, frame: &mut ratatui::Frame, state: &ModelState) {
        match state {
            ModelState::Load(load_state) => self.draw_load_model(frame, load_state),
            ModelState::Unload(unload_state) => self.draw_unload_model(frame, unload_state),
        }
    }

    /// Handle input for model state.
    pub(crate) fn handle_model_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: &ModelState,
    ) {
        match state {
            ModelState::Load(load_state) => self.handle_load_model_input(key, load_state),
            ModelState::Unload(unload_state) => self.handle_unload_model_input(key, unload_state),
        }
    }
}
