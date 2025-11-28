use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

/// State for the ModelSelector widget.
#[derive(Debug, Clone)]
pub struct ModelSelectorState {
    /// The currently selected index.
    selected: usize,
    /// Offset for scrolling.
    offset: usize,
    /// Scrollbar state.
    scrollbar_state: ScrollbarState,
}

impl Default for ModelSelectorState {
    fn default() -> Self {
        Self {
            selected: 0,
            offset: 0,
            scrollbar_state: ScrollbarState::default(),
        }
    }
}

impl ModelSelectorState {
    /// Create a new ModelSelectorState with the selected index at 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Set the selected index.
    pub fn select(&mut self, index: usize) {
        self.selected = index;
    }

    /// Move selection up with wrap-around.
    pub fn move_up(&mut self, max: usize) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if max > 0 {
            // Wrap to bottom
            self.selected = max - 1;
        }
    }

    /// Move selection down with wrap-around.
    pub fn move_down(&mut self, max: usize) {
        if max > 0 {
            if self.selected < max - 1 {
                self.selected += 1;
            } else {
                // Wrap to top
                self.selected = 0;
            }
        }
    }

    /// Reset selection to 0.
    pub fn reset(&mut self) {
        self.selected = 0;
        self.offset = 0;
        self.scrollbar_state = ScrollbarState::default();
    }

    /// Update the scroll offset based on selected item and viewport height.
    fn update_offset(&mut self, viewport_height: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + viewport_height {
            self.offset = self.selected.saturating_sub(viewport_height - 1);
        }
    }

    /// Update scrollbar state.
    fn update_scrollbar(&mut self, content_length: usize) {
        self.scrollbar_state = self.scrollbar_state
            .content_length(content_length)
            .position(self.selected);
    }
}

/// A widget for selecting items from a list.
///
/// This is a stateful widget that displays a list of items and allows
/// the user to select one using up/down arrow keys.
///
/// ## Example
///
/// ```rust
/// let selector = ModelSelector::new(&model_names)
///     .block(Block::bordered().title("Select a model"));
///
/// frame.render_stateful_widget(
///     selector,
///     area,
///     &mut self.model_selector_state,
/// );
/// ```
#[derive(Debug)]
pub struct ModelSelector<'a> {
    /// The items to display in the list.
    items: &'a [String],
    /// The block to wrap the list in.
    block: Option<Block<'a>>,
    /// Prefix for each item (e.g., "  " for indentation).
    item_prefix: &'a str,
}

const SELECTED_STYLE: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

const UNSELECTED_STYLE: Style = Style::new();

impl<'a> ModelSelector<'a> {
    /// Create a new ModelSelector with the given items.
    pub fn new(items: &'a [String]) -> Self {
        Self {
            items,
            block: None,
            item_prefix: "  ",
        }
    }

    /// Set the block to wrap the list in.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the prefix for each item.
    pub fn item_prefix(mut self, prefix: &'a str) -> Self {
        self.item_prefix = prefix;
        self
    }
}

impl<'a> StatefulWidget for ModelSelector<'a> {
    type State = ModelSelectorState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        // Calculate viewport height (accounting for borders if block is present)
        let viewport_height = if self.block.is_some() {
            area.height.saturating_sub(2) as usize // 2 for top and bottom borders
        } else {
            area.height as usize
        };

        // Update scroll offset and scrollbar state
        state.update_offset(viewport_height);
        state.update_scrollbar(self.items.len());

        // Calculate visible range
        let start = state.offset;
        let end = (start + viewport_height).min(self.items.len());

        // Create list items only for visible items
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start)
            .map(|(i, item)| {
                let style = if i == state.selected {
                    SELECTED_STYLE
                } else {
                    UNSELECTED_STYLE
                };
                ListItem::new(format!("{}{}", self.item_prefix, item)).style(style)
            })
            .collect();

        // Create and render the list
        let mut list = List::new(items);
        if let Some(block) = self.block {
            list = list.block(block);
        }
        Widget::render(list, area, buf);

        // Render scrollbar if needed (only if there are more items than viewport height)
        if self.items.len() > viewport_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            StatefulWidget::render(scrollbar, area, buf, &mut state.scrollbar_state);
        }
    }
}
