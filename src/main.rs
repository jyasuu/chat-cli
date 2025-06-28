use reqwest;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tokio;

#[derive(Serialize,Deserialize, Debug,Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize,Deserialize, Debug,Clone)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize,Deserialize, Debug,Clone)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Serialize,Deserialize, Debug,Clone)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

struct ChatClient {
    client: reqwest::Client,
    api_key: String,
    api_url: String,
    model: String,
    conversation: Vec<ChatMessage>,
}

impl ChatClient {
    fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            api_url: "https://gateway.ai.cloudflare.com/v1/0177dfd3fc04f0bb51d422b49f2dad20/jyasu-demo/openrouter/v1/chat/completions".to_string(),
            model: "deepseek/deepseek-chat:free".to_string(),
            conversation: Vec::new(),
        }
    }

    fn add_message(&mut self, role: &str, content: &str) {
        self.conversation.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    async fn send_message(&mut self, user_input: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Add user message to conversation
        self.add_message("user", user_input);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: self.conversation.clone(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("API error: {}", error_text).into());
        }

        let chat_response: ChatResponse = response.json().await?;
        
        if let Some(choice) = chat_response.choices.first() {
            let assistant_message = &choice.message.content;
            // Add assistant response to conversation
            self.add_message("assistant", assistant_message);
            Ok(assistant_message.clone())
        } else {
            Err("No response from API".into())
        }
    }

    fn clear_conversation(&mut self) {
        self.conversation.clear();
    }

    fn show_conversation(&self) {
        println!("\n=== Conversation History ===");
        for (i, msg) in self.conversation.iter().enumerate() {
            println!("{}. {}: {}", i + 1, msg.role.to_uppercase(), msg.content);
        }
        println!("============================\n");
    }
}

fn print_help() {
    println!("\n=== AI Chat CLI Help ===");
    println!("Commands:");
    println!("  /help    - Show this help message");
    println!("  /clear   - Clear conversation history");
    println!("  /history - Show conversation history");
    println!("  /quit    - Exit the chat");
    println!("  Just type your message to chat with the AI!");
    println!("========================\n");
}

fn get_api_key() -> Result<String, Box<dyn std::error::Error>> {
    // Try to get from environment variable first
    if let Ok(key) = std::env::var("API_KEY") {
        return Ok(key);
    }
    
    // If not found, prompt user
    print!("Enter your API key: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 AI Chat CLI");
    println!("===============");
    println!("Welcome to the AI Chat CLI! Type /help for commands.\n");

    // Get API key
    let api_key = match get_api_key() {
        Ok(key) if !key.is_empty() => key,
        _ => {
            eprintln!("Error: No API key provided. Set API_KEY environment variable or enter when prompted.");
            return Ok(());
        }
    };

    let mut client = ChatClient::new(api_key);
    
    loop {
        print!("You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input {
            "/quit" | "/exit" => {
                println!("👋 Goodbye!");
                break;
            }
            "/help" => {
                print_help();
                continue;
            }
            "/clear" => {
                client.clear_conversation();
                println!("🧹 Conversation cleared!");
                continue;
            }
            "/history" => {
                client.show_conversation();
                continue;
            }
            _ => {
                print!("AI: ");
                io::stdout().flush()?;
                
                match client.send_message(input).await {
                    Ok(response) => {
                        println!("{}\n", response);
                    }
                    Err(e) => {
                        eprintln!("❌ Error: {}\n", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}
