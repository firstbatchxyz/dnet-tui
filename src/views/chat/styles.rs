use ratatui::style::{Color, Modifier, Style};

/// [`Style`] for thinking text, dimmed & transparent-like.
pub const THINK_STYLE: Style = Style::new()
    .fg(Color::Rgb(255, 246, 229)) // rgba(255, 246, 229, 1)
    .add_modifier(Modifier::DIM);

pub const ASSISTANT_STYLE: Style = Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD);

pub const CURSOR_STYLE: Style = Style::new()
    .fg(Color::Blue)
    .add_modifier(Modifier::SLOW_BLINK);

pub const USER_STYLE: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);

pub const TIMESTAMP_STYLE: Style = Style::new().fg(Color::DarkGray);
