use anyhow::Result;
use chat_cli::chat_client::{ChatClient, AnyChatClient};

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Using the trait with dynamic dispatch (Box<dyn ChatClient>)
    let mut client: Box<dyn ChatClient> = Box::new(
        AnyChatClient::new_gemini("your-api-key".to_string(), "gemini-2.0-flash-exp".to_string())
    );
    
    // All clients implement the same interface
    client.add_user_message("Hello, how are you?");
    println!("Client: {}", client.client_name());
    
    // Example 2: Using the trait with generics
    async fn send_message_to_any_client<T: ChatClient>(client: &T, message: &str) -> Result<String> {
        client.send_message(message).await
    }
    
    // Works with any client that implements ChatClient
    let gemini_client = AnyChatClient::new_gemini("api-key".to_string(), "model".to_string());
    let openai_client = AnyChatClient::new_openai("api-key".to_string(), "gpt-4".to_string());
    
    // Both can be used with the same generic function
    let _response1 = send_message_to_any_client(&gemini_client, "Test message").await;
    let _response2 = send_message_to_any_client(&openai_client, "Test message").await;
    
    // Example 3: Switching between clients at runtime
    let use_openai = std::env::var("USE_OPENAI").is_ok();
    
    let mut runtime_client: Box<dyn ChatClient> = if use_openai {
        Box::new(AnyChatClient::new_openai("key".to_string(), "gpt-4".to_string()))
    } else {
        Box::new(AnyChatClient::new_gemini("key".to_string(), "gemini-2.0-flash-exp".to_string()))
    };
    
    runtime_client.add_user_message("This works with any client!");
    println!("Using: {}", runtime_client.client_name());
    
    Ok(())
}