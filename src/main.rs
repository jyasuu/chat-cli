mod gemini;
mod response_card;
mod prompt_input;
mod loading_animation;

use anyhow::Result;
use dotenv::dotenv;
use gemini::GeminiClient;
use response_card::ResponseCard;
use prompt_input::PromptInput;
use loading_animation::{LoadingAnimation, AnimationStyle, show_loading_in_response_box};
use std::{
    env,
    io::{self, Write},
};
use crossterm::{
    execute,
    terminal::{self, ClearType},
    cursor,
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
    
    // Clear screen and show welcome
    execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    println!("Gemini Chat CLI");
    println!("===============");
    println!("Enhanced with fancy input and Docker-style loading animations!");
    println!();
    
    let mut streaming_mode = true;
    let prompt_input = PromptInput::new().with_width(120);
    
    loop {
        // Get user input with fancy prompt
        let input = prompt_input.get_input()?;
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
                execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                println!("Gemini Chat CLI - Help");
                println!("======================");
                println!("Commands:");
                println!("  /help    - Show this help");
                println!("  /clear   - Clear the screen");
                println!("  /quit    - Exit the chat");
                println!("  /stream  - Toggle streaming mode (current: {})", if streaming_mode { "ON" } else { "OFF" });
                println!();
                println!("Features:");
                println!("  • Fancy bordered input interface");
                println!("  • Docker-style loading animations");
                println!("  • Real-time streaming responses");
                println!("  • Beautiful response cards");
                continue;
            }
            "/clear" | "/cls" => {
                execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                println!("Gemini Chat CLI");
                println!("===============");
                println!("Screen cleared! Ready for new conversation.");
                continue;
            }
            "/stream" => {
                streaming_mode = !streaming_mode;
                println!("Streaming mode: {}", if streaming_mode { "ON" } else { "OFF" });
                continue;
            }
            _ => {}
        }
        
        // Send message to Gemini with loading animation
        if streaming_mode {
            // Use ResponseCard for proper streaming with loading animation
            let response_card = ResponseCard::with_title("Response");
            
            // Show initial loading in response box with interrupt hint
            response_card.start_streaming()?;
            
            // Show interrupt hint during loading
            println!();
            println!("╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯");
            println!(" ctrl+c to interrupt");
            
            // Move cursor back up to spinner position
            execute!(io::stdout(), cursor::MoveUp(3), cursor::MoveToColumn(3))?;
            
            // Start spinner for connection
            let loading = LoadingAnimation::new("Connecting...")
                .with_style(AnimationStyle::Spinner);
            let loading_handle = loading.start();
            
            match client.send_message_stream(input).await {
                Ok(mut rx) => {
                    loading_handle.stop().await;
                    
                    // Clear the loading line and interrupt hint, start streaming content
                    print!("\r│ ");
                    io::stdout().flush()?;
                    
                    let mut response_text = String::new();
                    let mut is_first_chunk = true;
                    
                    while let Some(chunk) = rx.recv().await {
                        if is_first_chunk {
                            // Clear interrupt hint lines
                            execute!(io::stdout(), cursor::MoveDown(2))?;
                            println!("{}", " ".repeat(120)); // Clear the interrupt hint line
                            execute!(io::stdout(), cursor::MoveUp(3), cursor::MoveToColumn(3))?;
                            is_first_chunk = false;
                        }
                        
                        response_card.stream_content(&chunk)?;
                        response_text.push_str(&chunk);
                    }
                    
                    if response_text.is_empty() {
                        response_card.stream_content("No response received")?;
                    }
                    
                    // Complete the response box
                    response_card.end_streaming()?;
                }
                Err(e) => {
                    loading_handle.stop().await;
                    // Clear interrupt hint lines
                    execute!(io::stdout(), cursor::MoveDown(2))?;
                    println!("{}", " ".repeat(120)); // Clear the interrupt hint line
                    execute!(io::stdout(), cursor::MoveUp(3), cursor::MoveToColumn(3))?;
                    
                    response_card.stream_content(&format!("Error: Failed to get response: {}", e))?;
                    response_card.end_streaming()?;
                }
            }
        } else {
            // Non-streaming response with boxed loading animation
            let response_result = show_loading_in_response_box(
                client.send_message(input)
            ).await;
            
            match response_result {
                Ok(response) => {
                    let card = ResponseCard::with_title("Gemini Response");
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