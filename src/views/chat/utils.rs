use ratatui::text::Line;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ApiMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

impl ChatMessage {
    /// Returns the current local time formatted as "HH:MM".
    #[inline]
    pub fn now() -> String {
        chrono::Local::now().format("%H:%M").to_string()
    }

    pub fn new_user(content: &str) -> Self {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: Self::now(),
        }
    }

    pub fn new_assistant(content: &str) -> Self {
        ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: Self::now(),
        }
    }

    pub fn new_system(content: &str) -> Self {
        ChatMessage {
            role: "system".to_string(),
            content: content.to_string(),
            timestamp: Self::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for ApiMessage {
    fn from(msg: &ChatMessage) -> Self {
        ApiMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreamChoice {
    index: usize,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

/// Helper function to parse text with `<think>` tags,
/// returning a triple of `(before_think, thinking, after_think)`.
pub fn parse_think_tags(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut before_think = None;
    let mut thinking = None;
    let mut after_think = None;
    let mut remaining = text;
    if let Some(think_start) = remaining.find("<think>") {
        // get text before <think>
        if think_start > 0 {
            let before = &remaining[..think_start];
            if !before.is_empty() {
                before_think = Some(before.to_string());
            }
        }

        // find the closing tag
        remaining = &remaining[think_start + 7..]; // skip "<think>"
        if let Some(think_end) = remaining.find("</think>") {
            // get the content between tags
            let think_content = &remaining[..think_end];
            if !think_content.is_empty() {
                thinking = Some(think_content.to_string());
            }

            remaining = &remaining[think_end + 8..]; // skip "</think>"
            if !remaining.is_empty() {
                after_think = Some(remaining.to_string());
            }
        } else {
            // no closing tag found, treat rest as thinking text
            if !remaining.is_empty() {
                thinking = Some(remaining.to_string());
            }
        }
    } else {
        // no <think> tag found, entire text is after_think
        if !remaining.is_empty() {
            after_think = Some(remaining.to_string());
        }
    }

    (before_think, thinking, after_think)
}

pub fn parse_think_tags_to_lines(text: &str) -> Vec<Line> {
    use super::THINK_STYLE;

    let (before_think, thinking, after_think) = parse_think_tags(text);
    let mut lines = vec![];
    if let Some(before_think) = before_think {
        lines.push(Line::raw(before_think));
    };
    if let Some(thinking) = thinking.clone() {
        lines.push(Line::styled(thinking, THINK_STYLE));
    }
    if let Some(after_think) = after_think {
        if thinking.is_some() {
            lines.push(Line::raw(""));
            lines.push(Line::styled("---end thinking---", THINK_STYLE));
            lines.push(Line::raw(""))
        }
        lines.push(Line::raw(after_think));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_think_tags() {
        let input = "This is a test. <think>Thinking...</think> This is after.";
        let (before, thinking, after) = parse_think_tags(input);
        assert_eq!(before.unwrap(), "This is a test. ");
        assert_eq!(thinking.unwrap(), "Thinking...");
        assert_eq!(after.unwrap(), " This is after.");

        let input_no_think = "This is a test with no think tags.";
        let (before, thinking, after) = parse_think_tags(input_no_think);
        assert_eq!(before, None);
        assert_eq!(thinking, None);
        assert_eq!(after.unwrap(), "This is a test with no think tags.");

        let input_unclosed = "Start <think>Unclosed thinking...";
        let (before, thinking, after) = parse_think_tags(input_unclosed);
        assert_eq!(before.unwrap(), "Start ");
        assert_eq!(thinking, None);
        assert_eq!(after.unwrap(), "Unclosed thinking...");
    }
}
