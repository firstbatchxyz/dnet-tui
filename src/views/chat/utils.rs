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
    #[inline]
    fn now() -> String {
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
pub fn parse_think_tags(text: &str) -> (String, String, String) {
    let mut before_think = String::new();
    let mut thinking = String::new();
    let mut after_think = String::new();
    let mut remaining = text;
    if let Some(think_start) = remaining.find("<think>") {
        // get text before <think>
        if think_start > 0 {
            let before = &remaining[..think_start];
            if !before.is_empty() {
                before_think = before.to_string();
            }
        }

        // find the closing tag
        remaining = &remaining[think_start + 7..]; // Skip "<think>"
        if let Some(think_end) = remaining.find("</think>") {
            // Get the content between tags
            let think_content = &remaining[..think_end];
            if !think_content.is_empty() {
                thinking = think_content.to_string();
                // Use a lighter gray with dim modifier to simulate transparency
                // spans.push(Span::styled(
                //     think_content.to_string(),
                //     Style::default()
                //         .fg(Color::Rgb(255, 246, 229)) // #FFF6E5
                //         .add_modifier(Modifier::DIM), // Simulates transparency
                // ));
            }

            // Add the "--end thinking--" marker
            // spans.push(Span::styled(
            //     "\n\n--end thinking--\n\n".to_string(),
            //     Style::default()
            //         .fg(Color::Yellow)
            //         .add_modifier(Modifier::DIM | Modifier::ITALIC),
            // ));

            remaining = &remaining[think_end + 8..]; // Skip "</think>"
            if !remaining.is_empty() {
                after_think = remaining.to_string();
            }
        } else {
            // No closing tag found, treat rest as thinking text
            if !remaining.is_empty() {
                after_think = remaining.to_string();
                // spans.push(Span::styled(
                //     remaining.to_string(),
                //     Style::default()
                //         .fg(Color::Rgb(255, 246, 229))
                //         .add_modifier(Modifier::DIM),
                // ));
            }
        }
    } else {
        // No more <think> tags, add the rest as normal text
        if !remaining.is_empty() {
            after_think = remaining.to_string();
        }
    }

    // If no spans were created, return the original text
    // if spans.is_empty() {
    //     spans.push(Span::raw(text.to_string()));
    // }

    (before_think, thinking, after_think)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_think_tags() {
        let input = "This is a test. <think>Thinking...</think> This is after.";
        let (before, thinking, after) = parse_think_tags(input);
        assert_eq!(before, "This is a test. ");
        assert_eq!(thinking, "Thinking...");
        assert_eq!(after, " This is after.");

        let input_no_think = "This is a test with no think tags.";
        let (before, thinking, after) = parse_think_tags(input_no_think);
        assert_eq!(before, "");
        assert_eq!(thinking, "");
        assert_eq!(after, "This is a test with no think tags.");

        let input_unclosed = "Start <think>Unclosed thinking...";
        let (before, thinking, after) = parse_think_tags(input_unclosed);
        assert_eq!(before, "Start ");
        assert_eq!(thinking, "");
        assert_eq!(after, "Unclosed thinking...");
    }
}
