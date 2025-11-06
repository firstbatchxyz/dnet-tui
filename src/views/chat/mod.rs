mod utils;
pub use utils::ChatMessage;
use utils::*;

mod styles;
use styles::*;

use crate::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{collections::VecDeque, u16};
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug, Clone, PartialEq)]
pub enum ChatState {
    Active {
        messages: VecDeque<ChatMessage>,
        is_generating: bool,
        current_response: String,
        scroll_offset: u16,
        max_scroll: u16,
        max_tokens: u32,
        model: String,
    },
    Error(String),
}

impl ChatState {
    pub fn new(model: String, max_tokens: u32) -> Self {
        let mut state = ChatState::Active {
            messages: VecDeque::new(),
            model,
            is_generating: false,
            current_response: String::new(),
            scroll_offset: 0,
            max_scroll: 0,
            max_tokens,
        };

        // add welcome message
        state.add_message(ChatMessage::new_system(
            "Welcome to dnet chat! Type your message and press Enter to send.",
        ));

        state
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        if let ChatState::Active { messages, .. } = self {
            messages.push_back(message);
        }
    }
}

impl crate::App {
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
            ChatState::Active {
                model, max_tokens, ..
            } => Line::from(format!(
                "Chatting with {} (max tokens: {})",
                model, max_tokens
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

        match state {
            ChatState::Active {
                messages,
                is_generating,
                current_response,
                scroll_offset,
                ..
            } => {
                // Draw messages
                self.draw_chat_messages(
                    frame,
                    messages_area,
                    messages,
                    current_response,
                    *is_generating,
                    *scroll_offset,
                );

                // Draw input area
                self.draw_input_area(frame, input_area, *is_generating);

                // Footer
                let footer_text = if *is_generating {
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

    fn draw_chat_messages(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        messages: &VecDeque<ChatMessage>,
        current_response: &str,
        is_generating: bool,
        scroll_offset: u16,
    ) {
        let mut lines: Vec<Line> = Vec::new();
        for msg in messages {
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
                let think_lines = parse_think_tags_to_lines(&msg.content);
                lines.extend_from_slice(&think_lines);
            } else {
                lines.push(Line::from(msg.content.clone()));
            }

            // add a space between each message
            lines.push(Line::from(""));
        }

        // add current response if generating (or has content)
        if is_generating || !current_response.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", ChatMessage::now()), TIMESTAMP_STYLE),
                Span::styled("ASSISTANT", ASSISTANT_STYLE),
            ]));

            // parse current response for think tags
            let think_lines = parse_think_tags_to_lines(&current_response);
            lines.extend_from_slice(&think_lines);

            // add typing indicator if still generating
            if is_generating {
                lines.push(Line::from("▌").style(CURSOR_STYLE));
            }
        }

        // create paragraph
        let mut par = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false });

        // determine scroll
        let (width, height) = (area.width, area.height as usize);
        let num_lines = par.line_count(width);
        let max_scroll = if num_lines > height {
            num_lines - height
        } else {
            0
        };

        par = par.scroll((scroll_offset, 0));
        frame.render_widget(par, area);

        // FIXME: !!!
        self.state = match self.state.clone() {
            AppState::Chat(ChatState::Active {
                messages,
                is_generating,
                current_response,
                max_tokens,
                model,
                ..
            }) => AppState::Chat(ChatState::Active {
                messages,
                is_generating,
                current_response,
                scroll_offset,
                max_scroll: max_scroll as u16,
                max_tokens,
                model,
            }),
            other => other,
        };
    }

    fn draw_input_area(&mut self, frame: &mut Frame, area: Rect, is_generating: bool) {
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.chat_input.visual_scroll(width as usize);

        let input = Paragraph::new(self.chat_input.value())
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        if !is_generating {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.chat_input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }

    pub fn handle_chat_input(&mut self, key: KeyEvent, state: &ChatState) {
        if let ChatState::Active {
            messages,
            is_generating,
            current_response,
            scroll_offset,
            max_tokens,
            model,
            max_scroll,
        } = state
        {
            let mut messages = messages.clone();
            let is_generating = *is_generating;
            let mut current_response = current_response.clone();
            let mut scroll_offset = *scroll_offset;
            let max_tokens = *max_tokens;

            if is_generating {
                // Allow stopping generation or exiting
                match (key.modifiers, key.code) {
                    (_, KeyCode::Esc) => {
                        // we allow to exit chat even when generating
                        // eprintln!("ESC pressed during generation - switching to Menu");
                        self.state = AppState::Menu;
                        self.chat_stream_rx = None; // clear the stream
                        self.pending_chat_message = None; // clear any pending message
                        return; // early return to ensure state change takes effect
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        // Stop generation - TODO: would need to implement cancellation
                        // For now, just return to normal state
                        if !current_response.is_empty() {
                            messages.push_back(ChatMessage::new_assistant(&current_response));
                        }
                        current_response.clear();

                        self.state = AppState::Chat(ChatState::Active {
                            messages,
                            is_generating: false,
                            current_response,
                            scroll_offset,
                            max_tokens,
                            max_scroll: *max_scroll,
                            model: model.clone(),
                        });
                        self.chat_stream_rx = None; // Clear the stream
                        return; // Early return to ensure state change takes effect
                    }
                    _ => {}
                }
            } else {
                match (key.modifiers.clone(), key.code.clone()) {
                    (_, KeyCode::Esc) => {
                        self.state = AppState::Menu;
                        return; // early return to prevent state from being overwritten
                    }
                    // scroll up (offset shrinks)
                    (_, KeyCode::Up) => {
                        if scroll_offset > 0 {
                            scroll_offset -= 1;
                        }
                    }
                    // scroll down (offset grows)
                    (_, KeyCode::Down) => {
                        if scroll_offset < *max_scroll {
                            scroll_offset += 1;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => {
                        // Clear chat
                        messages.clear();
                        messages.push_back(ChatMessage::new_system(
                            "Chat cleared. Start a new conversation!",
                        ));
                        scroll_offset = 0;
                    }

                    (_, KeyCode::Enter) => {
                        let input_buffer = self.chat_input.value().trim();
                        if !input_buffer.is_empty() {
                            let user_input = input_buffer.to_string();
                            self.chat_input.reset();

                            // add user message
                            messages.push_back(ChatMessage {
                                role: "user".to_string(),
                                content: user_input.clone(),
                                timestamp: chrono::Local::now().format("%H:%M").to_string(),
                            });

                            // set generating state
                            self.state = AppState::Chat(ChatState::Active {
                                messages: messages.clone(),
                                is_generating: true,
                                current_response: String::new(),
                                scroll_offset,
                                max_tokens,
                                max_scroll: *max_scroll,
                                model: model.clone(),
                            });

                            // store the message for API call
                            self.pending_chat_message = Some(user_input);
                        }
                    }

                    (_, _) => {
                        let event = crossterm::event::Event::Key(key);
                        self.chat_input.handle_event(&event);
                    }
                }

                if !is_generating {
                    self.state = AppState::Chat(ChatState::Active {
                        messages,
                        is_generating,
                        current_response,
                        scroll_offset,
                        max_tokens,
                        max_scroll: *max_scroll,
                        model: model.clone(),
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
impl ChatState {
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
) -> Result<(), Box<dyn std::error::Error>> {
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
    pub(crate) async fn tick_chat(&mut self, state: &ChatState) {
        // Handle pending chat message
        if let Some(_message) = self.pending_chat_message.take() {
            if let ChatState::Active {
                messages,
                max_tokens,
                model,
                ..
            } = state
            {
                match ChatState::send_message(
                    &self.config.api_url(),
                    messages,
                    model,
                    *max_tokens,
                    self.config.temperature,
                )
                .await
                {
                    Ok(rx) => {
                        self.chat_stream_rx = Some(rx);
                    }
                    Err(err) => {
                        self.state = AppState::Chat(ChatState::Error(err));
                    }
                }
            }
        }

        // Process chat stream - but only if we're still in chat state
        if let Some(mut rx) = self.chat_stream_rx.take() {
            // Check if we're still in chat state
            if !matches!(self.state, AppState::Chat(_)) {
                // We've exited chat, don't process the stream
                self.chat_stream_rx = None;
            } else {
                let mut should_clear_rx = false;
                let mut new_error_state = None;

                // Try to receive messages without blocking
                while let Ok(chunk) = rx.try_recv() {
                    if let AppState::Chat(ChatState::Active {
                        messages,
                        is_generating,
                        current_response,
                        max_scroll,
                        scroll_offset,
                        ..
                    }) = &mut self.state
                    {
                        if chunk == "DONE" {
                            // Finalize the response
                            if !current_response.is_empty() {
                                messages.push_back(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: current_response.clone(),
                                    timestamp: chrono::Local::now().format("%H:%M").to_string(),
                                });
                                current_response.clear();
                            }
                            *is_generating = false;
                            should_clear_rx = true;
                            break;
                        } else if chunk.starts_with("ERROR:") {
                            new_error_state = Some(chunk);
                            should_clear_rx = true;
                            break;
                        } else {
                            // Append chunk to current response
                            current_response.push_str(&chunk);
                            // TODO: Auto-scroll during generation to follow the new content
                            // This ensures the user sees the latest tokens being generated
                            *scroll_offset = *max_scroll;
                        }
                    }
                }

                // Handle state changes after processing
                if let Some(error) = new_error_state {
                    self.state = AppState::Chat(ChatState::Error(error));
                } else if !should_clear_rx {
                    // Put the receiver back if we're not done
                    self.chat_stream_rx = Some(rx);
                }
            }
        }
    }
}
