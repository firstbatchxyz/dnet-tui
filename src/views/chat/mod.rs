mod utils;
pub use utils::ChatMessage; // used by tests
use utils::*;

mod styles;
use styles::*;

use crate::AppView;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use std::{collections::VecDeque, u16};
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug)]
pub struct ChatState {
    pub messages: VecDeque<ChatMessage>,
    /// Whether a response is currently being generated (streamed).
    pub is_generating: bool,
    pub current_response: String,
    pub scroll_cur: u16,
    /// Maximum scroll position, be careful about this as it may crash the app
    /// if set incorrectly.
    pub scroll_max: u16,
    /// Whether the scroll is locked, i.e. auto-scrolls to the bottom
    /// as new tokens are arriving. If the user scrolls manually while
    /// generating, this is set to false.
    pub scroll_locked: bool,
    // pub model: String,
    /// Chat message receiver for streaming responses
    pub stream_rx: Option<mpsc::UnboundedReceiver<String>>,
    /// Chat input area.
    pub input: tui_input::Input,
    /// Scrollbar for chat messages.
    pub scroll_bar: ScrollbarState,
    /// Pending chat message to send
    pub pending_chat_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatView {
    Active,
    Error(String),
}

impl Default for ChatState {
    fn default() -> Self {
        let mut state = ChatState {
            messages: VecDeque::new(),
            is_generating: false,
            current_response: String::new(),
            scroll_cur: 0,
            scroll_max: 0,
            scroll_locked: false,
            scroll_bar: ScrollbarState::default(),
            stream_rx: None,
            input: tui_input::Input::default(),
            pending_chat_message: None,
        };

        // add welcome message
        state.messages.push_back(ChatMessage::new_system(
            "Welcome to dnet chat! Type your message and press Enter to send.",
        ));

        state
    }
}

impl crate::App {
    pub fn draw_chat(&mut self, frame: &mut Frame, view: &ChatView) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Messages
            Constraint::Length(4), // Input
            Constraint::Length(2), // Footer
        ]);
        let [title_area, messages_area, input_area, footer_area] = vertical.areas(area);

        // Title with max tokens info
        let title = match view {
            ChatView::Active => Line::from(format!(
                "Chatting with {} (max tokens: {})",
                self.topology
                    .as_ref()
                    .and_then(|t| t.model.clone())
                    .unwrap_or_default(),
                self.config.max_tokens
            ))
            .bold()
            .cyan()
            .centered(),
            _ => Line::from("Chatting with Model").bold().cyan().centered(),
        };
        frame.render_widget(
            Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        match view {
            ChatView::Active => {
                // Draw messages
                self.draw_chat_messages(frame, messages_area);

                // Draw input area
                self.draw_input_area(frame, input_area, self.state.chat.is_generating);

                // Footer
                let footer_text = if self.state.chat.is_generating {
                    "Generating... | Ctrl+C: Stop generation | Esc: Exit chat"
                } else {
                    "Enter: Send | ↑↓: Scroll | Ctrl+L: Clear | Esc: Exit"
                };
                frame.render_widget(
                    Paragraph::new(footer_text)
                        .style(Style::default().fg(Color::DarkGray))
                        .centered(),
                    footer_area,
                );
            }
            ChatView::Error(err) => {
                frame.render_widget(
                    Paragraph::new(format!("Error: {}", err))
                        .block(Block::default().borders(Borders::ALL))
                        .style(Style::default().fg(Color::Red))
                        .wrap(Wrap { trim: true }),
                    messages_area,
                );

                frame.render_widget(
                    Paragraph::new("Press Esc to go back")
                        .style(Style::default().fg(Color::DarkGray))
                        .centered(),
                    footer_area,
                );
            }
        }
    }

    fn draw_chat_messages(&mut self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for msg in &self.state.chat.messages {
            // role & timestamp header
            let role_text = msg.role.to_uppercase();
            let role_style = match msg.role.as_str() {
                "user" => USER_STYLE,
                "assistant" => ASSISTANT_STYLE,
                _ => THINK_STYLE,
            };

            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", msg.timestamp), TIMESTAMP_STYLE),
                Span::styled(role_text, role_style),
            ]));

            // Add message content with word wrapping and think tag parsing
            if msg.role == "assistant" {
                // for assistant messages, parse think tags for the entire content
                let think_lines = parse_think_tags_to_lines(&msg.content, false);
                lines.extend_from_slice(&think_lines);
            } else {
                lines.push(Line::from(msg.content.clone()));
            }

            // add a space between each message
            lines.push(Line::from(""));
        }

        // add current response if generating (or has content)
        if self.state.chat.is_generating || !self.state.chat.current_response.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", ChatMessage::now()), TIMESTAMP_STYLE),
                Span::styled("ASSISTANT", ASSISTANT_STYLE),
            ]));

            // parse current response for think tags
            let think_lines = parse_think_tags_to_lines(&self.state.chat.current_response, true);
            lines.extend_from_slice(&think_lines);
        }

        // create paragraph
        let mut par = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false });

        // update max scroll
        let (width, height) = (area.width, area.height as usize);
        let num_lines = par.line_count(width - 2); // account for borders
        let max_scroll = if num_lines > height {
            num_lines - height
        } else {
            0
        };

        self.state.chat.scroll_max = max_scroll as u16;

        // sanity check, not needed for our case though
        self.state.chat.scroll_cur = self.state.chat.scroll_cur.min(self.state.chat.scroll_max);
        par = par.scroll((self.state.chat.scroll_cur, 0));
        frame.render_widget(par, area);

        // update scrollbar
        self.state.chat.scroll_bar = self
            .state
            .chat
            .scroll_bar
            .content_length(self.state.chat.scroll_max as usize)
            .position(self.state.chat.scroll_cur as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut self.state.chat.scroll_bar,
        );
    }

    fn draw_input_area(&mut self, frame: &mut Frame, area: Rect, is_generating: bool) {
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.state.chat.input.visual_scroll(width as usize);

        let input = Paragraph::new(self.state.chat.input.value())
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        if !is_generating {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.state.chat.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }

    pub fn handle_chat_input(&mut self, key: KeyEvent, view: &ChatView) {
        if let ChatView::Active = view {
            if self.state.chat.is_generating {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        // we allow to exit chat even when generating
                        // the stream may continue in the background
                        self.view = AppView::Menu;
                        return;
                    }
                    // scroll up (offset shrinks)
                    (_, KeyCode::Up) => {
                        if self.state.chat.scroll_cur > 0 {
                            self.state.chat.scroll_cur -= 1;
                            self.state.chat.scroll_locked = false;
                        }
                    }
                    // scroll down (offset grows)
                    (_, KeyCode::Down) => {
                        if self.state.chat.scroll_cur < self.state.chat.scroll_max {
                            self.state.chat.scroll_cur += 1;
                            self.state.chat.scroll_locked = false;

                            // lock anyways if we are back at the bottom
                            if self.state.chat.scroll_cur == self.state.chat.scroll_max {
                                self.state.chat.scroll_locked = true;
                            }
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        // stop generation - TODO: would need to implement cancellation
                        // For now, just return to normal state
                        if !self.state.chat.current_response.is_empty() {
                            self.state
                                .chat
                                .messages
                                .push_back(ChatMessage::new_assistant(
                                    &self.state.chat.current_response,
                                ));
                        }
                        self.state.chat.current_response.clear();

                        self.state.chat.is_generating = false;
                        self.state.chat.current_response.clear();
                        self.state.chat.stream_rx = None; // clear the stream
                        self.view = AppView::Menu;
                        return;
                    }
                    _ => {}
                }
            } else {
                match (key.modifiers.clone(), key.code.clone()) {
                    (_, KeyCode::Esc) => {
                        self.view = AppView::Menu;
                        return; // early return to prevent state from being overwritten
                    }
                    // scroll up (offset shrinks)
                    (_, KeyCode::Up) => {
                        if self.state.chat.scroll_cur > 0 {
                            self.state.chat.scroll_cur -= 1;
                        }
                    }
                    // scroll down (offset grows)
                    (_, KeyCode::Down) => {
                        if self.state.chat.scroll_cur < self.state.chat.scroll_max {
                            self.state.chat.scroll_cur += 1;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => {
                        self.state.chat.messages.clear();
                        self.state.chat.messages.push_back(ChatMessage::new_system(
                            "Chat cleared. Start a new conversation!",
                        ));
                        self.state.chat.scroll_cur = 0;
                    }

                    (_, KeyCode::Enter) => {
                        let input_buffer = self.state.chat.input.value().trim();
                        if !input_buffer.is_empty() {
                            let user_input = input_buffer.to_string();
                            self.state.chat.input.reset();

                            // add user message
                            self.state
                                .chat
                                .messages
                                .push_back(ChatMessage::new_user(&user_input));

                            // set generating state
                            self.state.chat.is_generating = true;
                            self.state.chat.scroll_locked = true;
                            self.state.chat.current_response.clear();

                            // store the message for API call
                            self.state.chat.pending_chat_message = Some(user_input);
                        }
                    }

                    (_, _) => {
                        let event = crossterm::event::Event::Key(key);
                        self.state.chat.input.handle_event(&event);
                    }
                }
            }
        } else if let ChatView::Error(_) = view {
            if key.code == KeyCode::Esc {
                self.view = AppView::Menu;
            }
        }
    }
}

/// Helper function to clean model-specific special tokens from streaming content
fn clean_model_tokens(content: &str) -> String {
    let mut cleaned = content.to_string();
    let tokens_to_remove = [
        "<|im_start|>",  // Qwen models (shouldn't appear but just in case)
        "<|im_end|>",    // Qwen models
        "<|endoftext|>", // GPT models
        "</s>",          // Llama models
        "<s>",           // Llama models
        "[INST]",        // Instruction models
        "[/INST]",       // Instruction models
        "�",             // Unicode replacement character (malformed UTF-8)
    ];
    for token in &tokens_to_remove {
        cleaned = cleaned.replace(token, "");
    }

    cleaned
}

// API functions for chat
impl ChatView {
    pub async fn send_message(
        api_url: &str,
        messages: &VecDeque<ChatMessage>,
        model: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<mpsc::UnboundedReceiver<String>, String> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Build message history for API
        let mut api_messages = Vec::new();

        // Add conversation
        // Skip the system message and don't duplicate the new message
        for msg in messages.iter() {
            if msg.role != "system" {
                api_messages.push(msg.into());
            }
        }

        // The new message is already added to messages in handle_chat_input,
        // so we don't add it again here
        let request = ChatRequest {
            model: model.to_string(),
            messages: api_messages,
            max_tokens: Some(max_tokens),
            temperature: Some(temperature),
            stream: true,
        };

        let api_url = api_url.to_string();

        // Spawn async task to handle streaming
        tokio::spawn(async move {
            if let Err(e) = stream_chat_response(api_url, request, tx).await {
                eprintln!("Stream error: {}", e);
            }
        });

        Ok(rx)
    }
}

async fn stream_chat_response(
    api_url: String,
    request: ChatRequest,
    tx: mpsc::UnboundedSender<String>,
) -> color_eyre::Result<()> {
    use futures::StreamExt;

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", api_url);

    let response = client.post(&url).json(&request).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        tx.send(format!("ERROR: {}", error_text)).ok();
        return Ok(());
    }

    // Stream the response bytes
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process all complete lines (SSE lines end with \n)
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].to_string();
            buffer.drain(..=line_end); // Remove the line including the \n

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Check if this is a data line
            if line.starts_with("data: ") {
                let json_str = &line[6..];

                if json_str.trim() == "[DONE]" {
                    tx.send("DONE".to_string()).ok();
                    return Ok(());
                }

                // Try to parse as JSON
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            // Filter out model-specific special tokens
                            let cleaned_content = clean_model_tokens(content);

                            // Only send if there's actual content after cleaning
                            if !cleaned_content.is_empty() {
                                tx.send(cleaned_content).ok();
                            }
                        }
                        if choice.finish_reason.is_some() {
                            tx.send("DONE".to_string()).ok();
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // Send DONE if not already sent
    tx.send("DONE".to_string()).ok();
    Ok(())
}

impl crate::App {
    /// Handle async operations for chat state (called during tick).
    pub(crate) async fn tick_chat(&mut self, view: &ChatView) {
        // Handle pending chat message
        if let Some(_message) = self.state.chat.pending_chat_message.take() {
            if let ChatView::Active = view {
                let Some(model) = self.topology.as_ref().and_then(|t| t.model.clone()) else {
                    self.view = AppView::Chat(ChatView::Error(
                        // we dont expect to get there at all without a model,
                        // but it still shall be handled
                        "No model configured in topology.".to_string(),
                    ));
                    return;
                };

                match ChatView::send_message(
                    &self.config.api_url(),
                    &self.state.chat.messages,
                    &model,
                    self.config.max_tokens,
                    self.config.temperature,
                )
                .await
                {
                    Ok(rx) => {
                        self.state.chat.stream_rx = Some(rx);
                    }
                    Err(err) => {
                        self.view = AppView::Chat(ChatView::Error(err));
                    }
                }
            }
        }

        // Process chat stream - but only if we're still in chat state
        if let Some(mut rx) = self.state.chat.stream_rx.take() {
            // Check if we're still in chat state
            if !matches!(self.view, AppView::Chat(_)) {
                // We've exited chat, don't process the stream
                // FIXME: ??
                self.state.chat.stream_rx = None;
            } else {
                let mut should_clear_rx = false;
                let mut new_error_state = None;

                // Try to receive messages without blocking
                while let Ok(chunk) = rx.try_recv() {
                    if let AppView::Chat(ChatView::Active) = &mut self.view {
                        if chunk == "DONE" {
                            // Finalize the response
                            if !self.state.chat.current_response.is_empty() {
                                self.state.chat.messages.push_back(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: self.state.chat.current_response.clone(),
                                    // TODO: store time itself here, convert to string later?
                                    // or use std instead of chrono?
                                    timestamp: chrono::Local::now().format("%H:%M").to_string(),
                                });
                                self.state.chat.current_response.clear();
                            }
                            self.state.chat.is_generating = false;
                            should_clear_rx = true;
                            break;
                        } else if chunk.starts_with("ERROR:") {
                            new_error_state = Some(chunk);
                            should_clear_rx = true;
                            break;
                        } else {
                            self.state.chat.current_response.push_str(&chunk);

                            // auto-scroll during generation to follow the new content
                            if self.state.chat.scroll_locked {
                                self.state.chat.scroll_cur = self.state.chat.scroll_max;
                            }
                        }
                    }
                }

                // Handle state changes after processing
                if let Some(error) = new_error_state {
                    self.view = AppView::Chat(ChatView::Error(error));
                } else if !should_clear_rx {
                    // put the receiver back if we're not done
                    self.state.chat.stream_rx = Some(rx);
                }
            }
        }
    }
}
