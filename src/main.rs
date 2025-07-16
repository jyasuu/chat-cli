mod gemini;
mod response_card;
mod prompt_input;
mod loading_animation;
mod function_calling;

use anyhow::Result;
use dotenv::dotenv;
use gemini::GeminiClient;
use response_card::ResponseCard;
use prompt_input::PromptInput;
use loading_animation::{LoadingAnimation, AnimationStyle, show_loading_in_response_box};
use function_calling::FunctionExecutor;
use std::{
    env,
    io::{self, Write},
    fs,
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
    let mut client = GeminiClient::new(api_key, "gemini-2.0-flash-exp".to_string());
    
    // Load system prompt from markdown file
    match fs::read_to_string("system_prompt.md") {
        Ok(system_prompt) => {
            client.load_system_prompt(&system_prompt)?;
            println!("System prompt loaded from system_prompt.md");
        }
        Err(_) => {
            println!("No system_prompt.md found, continuing without system prompt");
        }
    }
    
    // Initialize function executor
    let function_executor = FunctionExecutor::new();
    
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
        
        // Add user message to conversation history
        client.add_user_message(input);
        
        // Send message to Gemini with loading animation
        if streaming_mode {
            // Use ResponseCard for proper streaming with loading animation
            let response_card = ResponseCard::with_title("Response");
            
            // Show initial loading in response box with interrupt hint
            response_card.start_streaming()?;
            
            // Show interrupt hint during loading
            println!();
            println!();
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
                    let mut function_calls = Vec::new();
                    let mut is_first_chunk = true;
                    
                    while let Some((text_chunk, function_call)) = rx.recv().await {
                        if is_first_chunk {
                            // Clear interrupt hint lines
                            execute!(io::stdout(), cursor::MoveDown(2))?;
                            println!("{}", " ".repeat(120)); // Clear the interrupt hint line
                            execute!(io::stdout(), cursor::MoveUp(3), cursor::MoveToColumn(3))?;
                            is_first_chunk = false;
                        }
                        
                        if !text_chunk.is_empty() {
                            response_card.stream_content(&text_chunk)?;
                            response_text.push_str(&text_chunk);
                        }
                        
                        if let Some(fc) = function_call {
                            function_calls.push(fc);
                        }
                    }
                    
                    if response_text.is_empty() && function_calls.is_empty() {
                        response_card.stream_content("No response received")?;
                    }
                    
                    // Complete the response box
                    response_card.end_streaming()?;
                    
                    // Add model response to conversation history
                    let function_call_json = if function_calls.len() == 1 {
                        Some(function_calls[0].clone())
                    } else if function_calls.len() > 1 {
                        Some(serde_json::json!(function_calls))
                    } else {
                        None
                    };
                    client.add_model_response(&response_text, function_call_json.clone());
                    
                    // Handle function calls
                    for fc in function_calls {
                        if let Ok(function_call) = serde_json::from_value::<function_calling::FunctionCall>(fc) {
                            println!("\n🔧 Executing function: {}", function_call.name);
                            match function_executor.execute_function(&function_call).await {
                                Ok(function_response) => {
                                    let result_card = ResponseCard::with_title("Function Result");
                                    if let Some(output) = function_response.response.get("output") {
                                        result_card.display_complete(&output.as_str().unwrap_or("No output"))?;
                                    } else {
                                        result_card.display_complete(&serde_json::to_string_pretty(&function_response.response)?)?;
                                    }
                                    
                                    // Add function response to conversation history
                                    client.add_function_response(&function_response);
                                }
                                Err(e) => {
                                    let error_card = ResponseCard::with_title("Function Error");
                                    error_card.display_complete(&format!("Failed to execute function: {}", e))?;
                                }
                            }
                        }
                    }
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
                    
                    // Add model response to conversation history (non-streaming doesn't support function calls yet)
                    client.add_model_response(&response, None);
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