use crate::app::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum ChatState {
    Active {
        messages: VecDeque<ChatMessage>,
        input_buffer: String,
        cursor_position: usize,
        is_generating: bool,
        current_response: String,
        scroll_offset: usize,
        max_tokens: u32,
    },
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    messages: Vec<ApiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    index: usize,
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamDelta {
    role: Option<String>,
    content: Option<String>,
}

impl ChatState {
    pub fn new() -> Self {
        let mut messages = VecDeque::new();

        // Add welcome message
        messages.push_back(ChatMessage {
            role: "system".to_string(),
            content: "Welcome to dnet chat! Type your message and press Enter to send.".to_string(),
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        });

        ChatState::Active {
            messages,
            input_buffer: String::new(),
            cursor_position: 0,
            is_generating: false,
            current_response: String::new(),
            scroll_offset: 0,
            max_tokens: 2000,
        }
    }
}

impl App {
    pub fn draw_chat(&mut self, frame: &mut Frame, state: &ChatState) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Messages
            Constraint::Length(4), // Input
            Constraint::Length(2), // Footer
        ]);
        let [title_area, messages_area, input_area, footer_area] = vertical.areas(area);

        // Title with max tokens info
        let title = match state {
            ChatState::Active { max_tokens, .. } => {
                Line::from(format!("Chat with Model (Max Tokens: {})", max_tokens))
                    .bold()
                    .cyan()
                    .centered()
            }
            _ => Line::from("Chat with Model").bold().cyan().centered(),
        };
        frame.render_widget(
            Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        match state {
            ChatState::Active {
                messages,
                input_buffer,
                cursor_position,
                is_generating,
                current_response,
                scroll_offset,
                max_tokens: _,
            } => {
                // Draw messages
                self.draw_messages(frame, messages_area, messages, current_response, *is_generating, *scroll_offset);

                // Draw input area
                self.draw_input_area(frame, input_area, input_buffer, *cursor_position, *is_generating);

                // Footer
                let footer_text = if *is_generating {
                    "Generating... | Ctrl+C: Stop generation | Esc: Exit chat"
                } else {
                    "Enter: Send | ↑↓: Scroll | Ctrl+t: +500 tokens | Ctrl+T: -500 tokens | Ctrl+L: Clear | Esc: Exit"
                };
                frame.render_widget(
                    Paragraph::new(footer_text)
                        .style(Style::default().fg(Color::DarkGray))
                        .centered(),
                    footer_area,
                );
            }
            ChatState::Error(err) => {
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

    fn draw_messages(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        messages: &VecDeque<ChatMessage>,
        current_response: &str,
        is_generating: bool,
        scroll_offset: usize,
    ) {
        let mut lines: Vec<Line> = Vec::new();

        // Convert messages to lines
        for msg in messages {
            // Add role header
            let role_style = match msg.role.as_str() {
                "user" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                "assistant" => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(Color::Gray),
            };

            let role_text = msg.role.to_uppercase();
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                Span::styled(role_text, role_style),
            ]));

            // Add message content with word wrapping and think tag parsing
            if msg.role == "assistant" {
                // For assistant messages, parse think tags for the entire content
                let parsed_spans = parse_think_tags(&msg.content);

                // Now we need to wrap these spans into lines
                let width = area.width.saturating_sub(4) as usize;
                let wrapped_lines = wrap_spans(parsed_spans, width);
                for line in wrapped_lines {
                    lines.push(line);
                }
            } else {
                // For other messages, just wrap normally
                for line in msg.content.lines() {
                    if line.is_empty() {
                        lines.push(Line::from(""));
                    } else {
                        let width = area.width.saturating_sub(4) as usize;
                        let wrapped = wrap_text(line, width);
                        for wrapped_line in wrapped {
                            lines.push(Line::from(wrapped_line));
                        }
                    }
                }
            }
            lines.push(Line::from("")); // Add spacing between messages
        }

        // Add current response if generating (or has content)
        if is_generating || !current_response.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", chrono::Local::now().format("%H:%M")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("ASSISTANT", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ]));

            // Parse current response for think tags
            let parsed_spans = parse_think_tags(current_response);
            let width = area.width.saturating_sub(4) as usize;
            let wrapped_lines = wrap_spans(parsed_spans, width);
            for line in wrapped_lines {
                lines.push(line);
            }

            // Add typing indicator if still generating
            if is_generating {
                lines.push(Line::from(Span::styled("▌", Style::default().fg(Color::Blue).add_modifier(Modifier::SLOW_BLINK))));
            }
        }

        // Calculate visible lines based on scroll
        let total_lines = lines.len();
        let visible_height = area.height as usize;
        let scroll = if total_lines > visible_height {
            // If scroll_offset is very large (usize::MAX), scroll to bottom
            // This happens during generation to auto-follow new content
            if scroll_offset == usize::MAX {
                total_lines.saturating_sub(visible_height)
            } else {
                scroll_offset.min(total_lines.saturating_sub(visible_height))
            }
        } else {
            0
        };

        // Take only visible lines
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(scroll)
            .take(visible_height)
            .collect();

        let messages_widget = Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false });

        frame.render_widget(messages_widget, area);

        // Draw scroll indicator if needed
        if total_lines > visible_height {
            let scroll_percent = (scroll as f32 / (total_lines - visible_height) as f32 * 100.0) as u16;
            let scroll_indicator = format!(" {}% ", scroll_percent);
            frame.render_widget(
                Paragraph::new(scroll_indicator)
                    .style(Style::default().fg(Color::Yellow)),
                Rect::new(
                    area.x + area.width - 6,
                    area.y,
                    5,
                    1,
                ),
            );
        }
    }

    fn draw_input_area(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        input_buffer: &str,
        cursor_position: usize,
        is_generating: bool,
    ) {
        let input_style = if is_generating {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let input_text = if is_generating {
            format!("{} (waiting for response...)", input_buffer)
        } else {
            input_buffer.to_string()
        };

        let input = Paragraph::new(input_text)
            .style(input_style)
            .block(Block::default().borders(Borders::ALL).title("Your Message"));

        frame.render_widget(input, area);

        // Show cursor when not generating
        if !is_generating {
            // cursor_position is now in characters, not bytes
            // We need to calculate the visual position considering multi-byte characters
            let visual_position = input_buffer
                .chars()
                .take(cursor_position)
                .map(|ch| if ch.is_ascii() { 1 } else { 2 }) // Rough approximation for wide chars
                .sum::<usize>();

            let cursor_x = area.x + 1 + visual_position as u16;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    pub fn handle_chat_input(&mut self, key: KeyEvent, state: &ChatState) {
        if let ChatState::Active {
            messages,
            input_buffer,
            cursor_position,
            is_generating,
            current_response,
            scroll_offset,
            max_tokens,
        } = state {
            let mut messages = messages.clone();
            let mut input_buffer = input_buffer.clone();
            let mut cursor_position = *cursor_position;
            let is_generating = *is_generating;
            let mut current_response = current_response.clone();
            let mut scroll_offset = *scroll_offset;
            let mut max_tokens = *max_tokens;

            if is_generating {
                // Allow stopping generation or exiting
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        // Exit chat even when generating
                        eprintln!("ESC pressed during generation - switching to Menu");
                        self.state = AppState::Menu;
                        self.chat_stream_rx = None; // Clear the stream
                        self.pending_chat_message = None; // Clear any pending message
                        return; // Early return to ensure state change takes effect
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        // Stop generation - would need to implement cancellation
                        // For now, just return to normal state
                        if !current_response.is_empty() {
                            messages.push_back(ChatMessage {
                                role: "assistant".to_string(),
                                content: current_response.clone(),
                                timestamp: chrono::Local::now().format("%H:%M").to_string(),
                            });
                        }
                        current_response.clear();

                        self.state = AppState::Chat(ChatState::Active {
                            messages,
                            input_buffer,
                            cursor_position,
                            is_generating: false,
                            current_response,
                            scroll_offset,
                            max_tokens,
                        });
                        self.chat_stream_rx = None; // Clear the stream
                        return; // Early return to ensure state change takes effect
                    }
                    _ => {}
                }
                // Don't update state if we didn't handle the key
                return;
            } else {
                // Normal input handling
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        self.state = AppState::Menu;
                        return; // Early return to prevent state from being overwritten
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => {
                        // Clear chat
                        messages.clear();
                        messages.push_back(ChatMessage {
                            role: "system".to_string(),
                            content: "Chat cleared. Start a new conversation!".to_string(),
                            timestamp: chrono::Local::now().format("%H:%M").to_string(),
                        });
                        scroll_offset = 0;
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                        // Increase max tokens by 500 (cap at 10000)
                        max_tokens = (max_tokens + 500).min(10000);
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('T')) => {
                        // Decrease max tokens by 500 (minimum 100) with Shift+T (uppercase T)
                        max_tokens = max_tokens.saturating_sub(500).max(100);
                    }
                    (_, KeyCode::Enter) => {
                        if !input_buffer.trim().is_empty() {
                            // Add user message
                            messages.push_back(ChatMessage {
                                role: "user".to_string(),
                                content: input_buffer.clone(),
                                timestamp: chrono::Local::now().format("%H:%M").to_string(),
                            });

                            // Prepare for generation
                            let user_input = input_buffer.clone();
                            input_buffer.clear();
                            cursor_position = 0;

                            // Auto-scroll to bottom
                            scroll_offset = messages.len().saturating_sub(10);

                            // Set generating state
                            self.state = AppState::Chat(ChatState::Active {
                                messages: messages.clone(),
                                input_buffer: input_buffer.clone(),
                                cursor_position,
                                is_generating: true,
                                current_response: String::new(),
                                scroll_offset,
                                max_tokens,
                            });

                            // Store the message for API call
                            self.pending_chat_message = Some(user_input);
                        }
                    }
                    (_, KeyCode::Backspace) => {
                        if cursor_position > 0 {
                            // Find the character boundary before cursor
                            let mut chars: Vec<char> = input_buffer.chars().collect();
                            if cursor_position <= chars.len() {
                                chars.remove(cursor_position - 1);
                                input_buffer = chars.into_iter().collect();
                                cursor_position -= 1;
                            }
                        }
                    }
                    (_, KeyCode::Delete) => {
                        let chars: Vec<char> = input_buffer.chars().collect();
                        if cursor_position < chars.len() {
                            let mut chars = chars;
                            chars.remove(cursor_position);
                            input_buffer = chars.into_iter().collect();
                        }
                    }
                    (_, KeyCode::Left) => {
                        if cursor_position > 0 {
                            cursor_position -= 1;
                        }
                    }
                    (_, KeyCode::Right) => {
                        let char_count = input_buffer.chars().count();
                        if cursor_position < char_count {
                            cursor_position += 1;
                        }
                    }
                    (_, KeyCode::Home) => {
                        cursor_position = 0;
                    }
                    (_, KeyCode::End) => {
                        cursor_position = input_buffer.chars().count();
                    }
                    (_, KeyCode::Up) => {
                        if scroll_offset > 0 {
                            scroll_offset -= 1;
                        }
                    }
                    (_, KeyCode::Down) => {
                        scroll_offset += 1;
                    }
                    (_, KeyCode::Char(c)) => {
                        // Convert to chars, insert at character position, then convert back
                        let mut chars: Vec<char> = input_buffer.chars().collect();
                        if cursor_position <= chars.len() {
                            chars.insert(cursor_position, c);
                            input_buffer = chars.into_iter().collect();
                            cursor_position += 1;
                        }
                    }
                    _ => {}
                }

                if !is_generating {
                    self.state = AppState::Chat(ChatState::Active {
                        messages,
                        input_buffer,
                        cursor_position,
                        is_generating,
                        current_response,
                        scroll_offset,
                        max_tokens,
                    });
                }
            }
        } else if let ChatState::Error(_) = state {
            if key.code == KeyCode::Esc {
                self.state = AppState::Menu;
            }
        }
    }
}

// Helper function to wrap text
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() + 1 <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

// Helper function to wrap styled spans into lines with proper word wrapping
fn wrap_spans(spans: Vec<Span<'_>>, width: usize) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_line_spans: Vec<Span> = Vec::new();

    for span in spans {
        let text = span.content.to_string();
        let style = span.style;

        // Split by newlines first
        let parts: Vec<&str> = text.split('\n').collect();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                // We hit a newline, finish current line
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                    current_line.clear();
                } else if !current_line.is_empty() {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
            }

            // Now wrap this part if needed
            if part.is_empty() {
                continue;
            }

            // Simple word wrapping for this span part
            for word in part.split_whitespace() {
                let word_with_space = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!(" {}", word)
                };

                if current_line.len() + word_with_space.len() > width && !current_line.is_empty() {
                    // Need to wrap
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                    current_line.clear();

                    current_line_spans.push(Span::styled(word.to_string(), style));
                    current_line = word.to_string();
                } else {
                    // Add to current line
                    current_line_spans.push(Span::styled(word_with_space.clone(), style));
                    current_line.push_str(&word_with_space);
                }
            }
        }
    }

    // Add any remaining content
    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    lines
}

// Helper function to parse text with <think> tags and return styled spans
fn parse_think_tags(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();

    // Simple regex-like approach: find <think> and </think> tags
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(think_start) = remaining.find("<think>") {
            // Add text before <think>
            if think_start > 0 {
                let before = &remaining[..think_start];
                if !before.is_empty() {
                    spans.push(Span::raw(before.to_string()));
                }
            }

            // Find the closing tag
            remaining = &remaining[think_start + 7..]; // Skip "<think>"

            if let Some(think_end) = remaining.find("</think>") {
                // Get the content between tags
                let think_content = &remaining[..think_end];
                if !think_content.is_empty() {
                    // Use a lighter gray with dim modifier to simulate transparency
                    spans.push(Span::styled(
                        think_content.to_string(),
                        Style::default()
                            .fg(Color::Rgb(255, 246, 229))  // #FFF6E5
                            .add_modifier(Modifier::DIM)     // Simulates transparency
                    ));
                }

                // Add the "--end thinking--" marker
                spans.push(Span::styled(
                    "\n\n--end thinking--\n\n".to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM | Modifier::ITALIC)
                ));

                remaining = &remaining[think_end + 8..]; // Skip "</think>"
            } else {
                // No closing tag found, treat rest as thinking text
                if !remaining.is_empty() {
                    spans.push(Span::styled(
                        remaining.to_string(),
                        Style::default()
                            .fg(Color::Rgb(255, 246, 229))
                            .add_modifier(Modifier::DIM)
                    ));
                }
                break;
            }
        } else {
            // No more <think> tags, add the rest as normal text
            if !remaining.is_empty() {
                spans.push(Span::raw(remaining.to_string()));
            }
            break;
        }
    }

    // If no spans were created, return the original text
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }

    spans
}

// API functions for chat
impl ChatState {
    pub async fn send_message(
        api_url: &str,
        messages: &VecDeque<ChatMessage>,
        max_tokens: u32,
    ) -> Result<mpsc::UnboundedReceiver<String>, String> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Build message history for API
        let mut api_messages = Vec::new();

        // Add conversation history (limit to last 10 messages for context)
        // Skip the system message and don't duplicate the new message
        for msg in messages.iter() {
            if msg.role != "system" {
                api_messages.push(ApiMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
        }

        // The new message is already added to messages in handle_chat_input,
        // so we don't add it again here

        let request = ChatRequest {
            messages: api_messages,
            max_tokens: Some(max_tokens),
            temperature: Some(0.7),
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
) -> Result<(), Box<dyn std::error::Error>> {
    use futures::StreamExt;

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", api_url);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await?;

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
                            // Send each token immediately
                            tx.send(content.clone()).ok();
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

// Check if model is loaded
pub async fn is_model_loaded(api_url: &str) -> bool {
    // Try to get topology to see if model is loaded
    let url = format!("{}/v1/topology", api_url);
    if let Ok(response) = reqwest::get(&url).await {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            // Check if model field exists and is not empty
            if let Some(model) = json.get("model") {
                return model.as_str().map_or(false, |s| !s.is_empty());
            }
        }
    }
    false
}