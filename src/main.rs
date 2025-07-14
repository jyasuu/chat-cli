mod gemini;
mod response_card;

use anyhow::Result;
use dotenv::dotenv;
use gemini::GeminiClient;
use response_card::ResponseCard;
use std::{
    env,
    io::{self, Write},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable not set. Please copy .env.example to .env and set your API key.");
    
    // Initialize Gemini client
    let client = GeminiClient::new(api_key, "gemini-2.0-flash-exp".to_string());
    
    println!("Gemini Chat CLI");
    println!("===============");
    println!("Type your message and press Enter to send.");
    println!("Commands:");
    println!("  /help    - Show this help");
    println!("  /clear   - Clear the screen");
    println!("  /quit    - Exit the chat");
    println!("  /stream  - Toggle streaming mode (default: on)");
    println!();
    
    let mut streaming_mode = true;
    
    loop {
        // Get user input
        print!("You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Handle commands
        match input {
            "/quit" | "/exit" | "/q" => {
                println!("Goodbye!");
                break;
            }
            "/help" | "/h" => {
                println!("Commands:");
                println!("  /help    - Show this help");
                println!("  /clear   - Clear the screen");
                println!("  /quit    - Exit the chat");
                println!("  /stream  - Toggle streaming mode");
                println!();
                continue;
            }
            "/clear" | "/cls" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush()?;
                continue;
            }
            "/stream" => {
                streaming_mode = !streaming_mode;
                println!("Streaming mode: {}", if streaming_mode { "ON" } else { "OFF" });
                continue;
            }
            _ => {}
        }
        
        // Send message to Gemini
        if streaming_mode {
            // Streaming response with response card
            match client.send_message_stream(input).await {
                Ok(mut rx) => {
                    let card = ResponseCard::new();
                    let mut response_text = String::new();
                    
                    // Start the response card
                    card.start_streaming()?;
                    
                    while let Some(chunk) = rx.recv().await {
                        card.stream_content(&chunk)?;
                        response_text.push_str(&chunk);
                    }
                    
                    if response_text.is_empty() {
                        print!("No response received");
                    }
                    
                    // End the response card
                    card.end_streaming()?;
                }
                Err(e) => {
                    let error_card = ResponseCard::with_title("Error");
                    error_card.display_complete(&format!("Failed to get response: {}", e))?;
                }
            }
        } else {
            // Non-streaming response with response card
            match client.send_message(input).await {
                Ok(response) => {
                    let card = ResponseCard::new();
                    card.display_complete(&response)?;
                }
                Err(e) => {
                    let error_card = ResponseCard::with_title("Error");
                    error_card.display_complete(&format!("Failed to get response: {}", e))?;
                }
            }
        }
        
        println!(); // Extra line for spacing
    }
    
    Ok(())
}