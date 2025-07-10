use chrono::{DateTime, Utc};
use crate::gemini::GeminiClient;

#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    User,
    Assistant,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
    pub is_loading: bool,
    pub gemini_client: GeminiClient,
}

impl App {
    pub fn new(gemini_client: GeminiClient) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: vec![
                Message {
                    content: "Welcome to Gemini Chat CLI! Press 'i' to start typing, 'q' to quit, 'c' to clear chat.".to_string(),
                    message_type: MessageType::Assistant,
                    timestamp: Utc::now(),
                }
            ],
            scroll_offset: 0,
            is_loading: false,
            gemini_client,
        }
    }
}