use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, StatefulWidget, Widget},
};

/// State for the ModelSelector widget.
#[derive(Debug, Default, Clone)]
pub struct ModelSelectorState {
    /// The currently selected index.
    selected: usize,
    /// Offset for scrolling.
    offset: usize,
}

impl ModelSelectorState {
    /// Create a new ModelSelectorState with the selected index at 0.
    pub fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
        }
    }

    /// Get the currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Set the selected index.
    pub fn select(&mut self, index: usize) {
        self.selected = index;
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self, max: usize) {
        if max > 0 && self.selected < max - 1 {
            self.selected += 1;
        }
    }

    /// Reset selection to 0.
    pub fn reset(&mut self) {
        self.selected = 0;
        self.offset = 0;
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
    .fg(Color::Cyan)
    .bg(Color::Black)
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
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == state.selected {
                    SELECTED_STYLE
                } else {
                    UNSELECTED_STYLE
                };
                ListItem::new(format!("{}{}", self.item_prefix, item)).style(style)
            })
            .collect();

        let mut list = List::new(items);
        if let Some(block) = self.block {
            list = list.block(block);
        }

        Widget::render(list, area, buf);
    }
}
