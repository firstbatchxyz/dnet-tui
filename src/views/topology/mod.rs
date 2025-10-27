/// Shard-viewer.
mod shard;
pub use shard::ShardViewState;

/// Ring topology viewer.
mod ring;
pub use ring::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyState {
    Shard(String, ShardViewState),
    Ring(TopologyRingState),
}

impl crate::App {
    /// Handle async operations for topology state (called during tick).
    pub(crate) async fn tick_topology(&mut self, state: &TopologyState) {
        match state {
            TopologyState::Ring(ring_state) => self.tick_topology_ring(ring_state).await,
            TopologyState::Shard(device, shard_state) => {
                self.tick_topology_shard(device, shard_state).await
            }
        }
    }

    /// Draw topology state.
    pub(crate) fn draw_topology(&mut self, frame: &mut ratatui::Frame, state: &TopologyState) {
        match state {
            TopologyState::Ring(ring_state) => self.draw_topology_ring_view(frame, ring_state),
            TopologyState::Shard(device, shard_state) => {
                self.draw_shard_interaction(frame, device, shard_state)
            }
        }
    }

    /// Handle input for topology state.
    pub(crate) fn handle_topology_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: &TopologyState,
    ) {
        match state {
            TopologyState::Ring(_ring_state) => self.handle_topology_ring_input(key),
            TopologyState::Shard(_, _) => self.handle_shard_interaction_input(key),
        }
    }
}
