mod gemini;
mod openai;
mod response_card;
mod prompt_input;
mod loading_animation;
mod function_calling;

use anyhow::Result;
use dotenv::dotenv;
use gemini::GeminiClient;
use openai::OpenAIClient;
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

// Enum to handle different client types
enum ChatClient {
    Gemini(GeminiClient),
    OpenAI(OpenAIClient),
}

impl ChatClient {
    fn add_user_message(&mut self, message: &str) {
        match self {
            ChatClient::Gemini(client) => client.add_user_message(message),
            ChatClient::OpenAI(client) => client.add_user_message(message),
        }
    }

    fn add_function_response(&mut self, function_response: &function_calling::FunctionResponse) {
        match self {
            ChatClient::Gemini(client) => client.add_function_response(function_response),
            ChatClient::OpenAI(client) => client.add_function_response(function_response),
        }
    }

    fn add_model_response(&mut self, response: &str, function_call: Option<serde_json::Value>) {
        match self {
            ChatClient::Gemini(client) => client.add_model_response(response, function_call),
            ChatClient::OpenAI(client) => {
                // Convert function call format for OpenAI
                let tool_calls = function_call.map(|fc| {
                    vec![openai::ToolCall {
                        id: format!("call_{}", chrono::Utc::now().timestamp_millis()),
                        call_type: "function".to_string(),
                        function: openai::FunctionCall {
                            name: fc.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                            arguments: fc.get("args").map(|v| v.to_string()).unwrap_or_default(),
                        },
                    }]
                });
                client.add_model_response(response, tool_calls);
            }
        }
    }

    async fn send_message_stream(&self, message: &str) -> Result<tokio::sync::mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        match self {
            ChatClient::Gemini(client) => client.send_message_stream(message).await,
            ChatClient::OpenAI(client) => client.send_message_stream(message).await,
        }
    }

    #[allow(dead_code)]
    async fn send_message(&self, message: &str) -> Result<String> {
        match self {
            ChatClient::Gemini(client) => client.send_message(message).await,
            ChatClient::OpenAI(client) => client.send_message(message).await,
        }
    }

    fn clear_conversation(&mut self) {
        match self {
            ChatClient::Gemini(client) => client.clear_conversation(),
            ChatClient::OpenAI(client) => client.clear_conversation(),
        }
    }

    fn client_name(&self) -> &str {
        match self {
            ChatClient::Gemini(_) => "Gemini",
            ChatClient::OpenAI(_) => "OpenAI",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Determine which client to use based on environment variables
    let mut client = if let Ok(openai_key) = env::var("OPENAI_API_KEY") {
        // Check if user wants to use OpenAI specifically
        let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string());
        let base_url = env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        
        let mut openai_client = OpenAIClient::new(openai_key, model).with_base_url(base_url);
        
        // Load system prompt
        if let Ok(system_prompt) = fs::read_to_string("system_prompt.md") {
            openai_client.load_system_prompt(&system_prompt)?;
            println!("System prompt loaded from system_prompt.md");
        } else {
            println!("No system_prompt.md found, continuing without system prompt");
        }
        
        ChatClient::OpenAI(openai_client)
    } else if let Ok(gemini_key) = env::var("GEMINI_API_KEY") {
        let model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash-exp".to_string());
        let mut gemini_client = GeminiClient::new(gemini_key, model);
        
        // Load system prompt
        if let Ok(system_prompt) = fs::read_to_string("system_prompt.md") {
            gemini_client.load_system_prompt(&system_prompt)?;
            println!("System prompt loaded from system_prompt.md");
        } else {
            println!("No system_prompt.md found, continuing without system prompt");
        }
        
        ChatClient::Gemini(gemini_client)
    } else {
        return Err(anyhow::anyhow!(
            "No API key found. Please set either OPENAI_API_KEY or GEMINI_API_KEY environment variable.\n\
             You can also set OPENAI_MODEL, GEMINI_MODEL, and OPENAI_BASE_URL for customization."
        ));
    };
    
    // Initialize function executor
    let function_executor = FunctionExecutor::new();
    
    // Clear screen and show welcome
    execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    println!("{} Chat CLI", client.client_name());
    println!("===============");
    println!("Enhanced with fancy input and Docker-style loading animations!");
    println!("Using: {}", client.client_name());
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
                println!("{} Chat CLI - Help", client.client_name());
                println!("======================");
                println!("Commands:");
                println!("  /help    - Show this help");
                println!("  /clear   - Clear the screen");
                println!("  /quit    - Exit the chat");
                println!("  /stream  - Toggle streaming mode (current: {})", if streaming_mode { "ON" } else { "OFF" });
                println!("  /switch  - Clear conversation history");
                println!();
                println!("Features:");
                println!("  * Fancy bordered input interface");
                println!("  * Docker-style loading animations");
                println!("  * Real-time streaming responses");
                println!("  * Beautiful response cards");
                println!("  * Function calling support");
                println!("  * Multi-provider support (OpenAI/Gemini)");
                continue;
            }
            "/clear" | "/cls" => {
                execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                println!("{} Chat CLI", client.client_name());
                println!("===============");
                println!("Screen cleared! Ready for new conversation.");
                continue;
            }
            "/stream" => {
                streaming_mode = !streaming_mode;
                println!("Streaming mode: {}", if streaming_mode { "ON" } else { "OFF" });
                continue;
            }
            "/switch" | "/reset" => {
                client.clear_conversation();
                println!("Conversation history cleared!");
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
                                    
                                    // CRITICAL: Continue conversation with function result - send back to LLM
                                    println!("\n[LLM] Getting LLM response to function result...");
                                    match client.send_message_stream("").await {
                                        Ok(mut follow_up_rx) => {
                                            let follow_up_card = ResponseCard::with_title("LLM Response");
                                            follow_up_card.start_streaming()?;
                                            
                                            let mut follow_up_response = String::new();
                                            
                                            while let Some((text_chunk, _function_call)) = follow_up_rx.recv().await {
                                                if !text_chunk.is_empty() {
                                                    follow_up_card.stream_content(&text_chunk)?;
                                                    follow_up_response.push_str(&text_chunk);
                                                }
                                            }
                                            
                                            if follow_up_response.is_empty() {
                                                follow_up_card.stream_content("No response received")?;
                                            }
                                            
                                            follow_up_card.end_streaming()?;
                                            
                                            // Add the follow-up response to conversation history
                                            client.add_model_response(&follow_up_response, None);
                                        }
                                        Err(e) => {
                                            println!("\n[ERROR] Failed to get LLM response: {}", e);
                                        }
                                    }
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
                    let card = ResponseCard::with_title(&format!("{} Response", client.client_name()));
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