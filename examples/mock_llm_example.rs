use chat_cli::{ChatClient, AnyChatClient, MockLLMClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Mock LLM Example");
    println!("================");

    // Example 1: Basic mock client
    println!("\n1. Basic Mock Client:");
    let mut basic_client = AnyChatClient::new_mock();
    
    let response1 = basic_client.send_message("Hello!").await?;
    println!("User: Hello!");
    println!("Mock: {}", response1);

    let response2 = basic_client.send_message("How are you?").await?;
    println!("User: How are you?");
    println!("Mock: {}", response2);

    // Example 2: Custom responses
    println!("\n2. Custom Responses:");
    let custom_responses = vec![
        "Welcome to the test environment!".to_string(),
        "All systems are operational.".to_string(),
        "Test completed successfully.".to_string(),
    ];
    let mut custom_client = AnyChatClient::new_mock_with_responses(custom_responses);

    for i in 1..=4 {
        let response = custom_client.send_message(&format!("Test message {}", i)).await?;
        println!("User: Test message {}", i);
        println!("Mock: {}", response);
    }

    // Example 3: Streaming responses
    println!("\n3. Streaming Response:");
    let mut streaming_client = AnyChatClient::new_mock();
    let mut rx = streaming_client.send_message_stream("Tell me about Rust programming").await?;
    
    print!("User: Tell me about Rust programming\nMock: ");
    while let Some((chunk, function_call)) = rx.recv().await {
        print!("{}", chunk);
        if let Some(fc) = function_call {
            println!("\n[Function Call: {}]", fc);
        }
    }
    println!();

    // Example 4: Function calls
    println!("\n4. Function Calls:");
    let mut function_client = MockLLMClient::new();
    
    // Add a function call response
    let weather_function = serde_json::json!({
        "name": "get_weather",
        "args": {"location": "San Francisco", "units": "celsius"}
    });
    function_client.add_function_call_response("weather", weather_function);

    let mut any_client = AnyChatClient::Mock(function_client);
    let response = any_client.send_message("What's the weather like in San Francisco?").await?;
    println!("User: What's the weather like in San Francisco?");
    println!("Mock: {}", response);

    // Example 5: System prompt
    println!("\n5. System Prompt:");
    let mut system_client = AnyChatClient::new_mock();
    system_client.load_system_prompt("You are a helpful testing assistant. Always mention that you're in test mode.")?;
    
    let response = system_client.send_message("Hello").await?;
    println!("User: Hello");
    println!("Mock: {}", response);

    // Example 6: Conversation history
    println!("\n6. Conversation History:");
    let mut history_client = AnyChatClient::new_mock();
    
    history_client.add_user_message("My name is Alice");
    let response1 = history_client.send_message("What's my name?").await?;
    println!("User: My name is Alice");
    println!("User: What's my name?");
    println!("Mock: {}", response1);

    println!("\nExample completed!");
    Ok(())
}