/// Shard-viewer.
mod shard;
pub use shard::ShardView;

/// Ring topology viewer.
mod ring;
pub use ring::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyView {
    Shard(String, ShardView),
    Ring(TopologyRingView),
}

#[derive(Default, Debug)]
pub struct TopologyState {
    /// Selected device index in topology view.
    pub selected_device: usize,
}

impl crate::App {
    /// Handle async operations for topology state (called during tick).
    pub(crate) async fn tick_topology(&mut self, view: &TopologyView) {
        match view {
            TopologyView::Ring(view) => self.tick_topology_ring(view).await,
            TopologyView::Shard(device, view) => self.tick_topology_shard(device, view).await,
        }
    }

    /// Draw topology state.
    pub(crate) fn draw_topology(&mut self, frame: &mut ratatui::Frame, view: &TopologyView) {
        match view {
            TopologyView::Ring(view) => self.draw_topology_ring_view(frame, view),
            TopologyView::Shard(device, view) => self.draw_shard_interaction(frame, device, view),
        }
    }

    /// Handle input for topology state.
    pub(crate) fn handle_topology_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        view: &TopologyView,
    ) {
        match view {
            TopologyView::Ring(_) => self.handle_topology_ring_input(key),
            TopologyView::Shard(_, _) => self.handle_shard_interaction_input(key),
        }
    }
}
