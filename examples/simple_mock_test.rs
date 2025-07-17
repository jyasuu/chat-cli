use chat_cli::{ChatClient, AnyChatClient, MockLLMClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Simple Mock LLM Test");
    println!("====================");

    // Test 1: Basic mock client usage
    println!("\n1. Basic Mock Client:");
    let client = AnyChatClient::new_mock();
    let response = client.send_message("Hello, mock LLM!").await?;
    println!("User: Hello, mock LLM!");
    println!("Mock: {}", response);

    // Test 2: Custom responses
    println!("\n2. Custom Responses:");
    let custom_responses = vec![
        "Welcome to testing mode!".to_string(),
        "All systems operational.".to_string(),
        "Test completed successfully.".to_string(),
    ];
    let client = AnyChatClient::new_mock_with_responses(custom_responses);
    
    for i in 1..=3 {
        let response = client.send_message(&format!("Test {}", i)).await?;
        println!("User: Test {}", i);
        println!("Mock: {}", response);
    }

    // Test 3: Streaming
    println!("\n3. Streaming Test:");
    let client = AnyChatClient::new_mock();
    let mut rx = client.send_message_stream("Tell me about testing").await?;
    
    print!("User: Tell me about testing\nMock: ");
    while let Some((chunk, _function_call)) = rx.recv().await {
        print!("{}", chunk);
    }
    println!();

    // Test 4: Function calls
    println!("\n4. Function Call Test:");
    let mut mock_client = MockLLMClient::new();
    
    // Add a function call response
    let weather_function = serde_json::json!({
        "name": "get_weather",
        "args": {"location": "San Francisco"}
    });
    mock_client.add_function_call_response("weather", weather_function);
    
    let client = AnyChatClient::Mock(mock_client);
    let response = client.send_message("What's the weather like?").await?;
    println!("User: What's the weather like?");
    println!("Mock: {}", response);

    println!("\nAll tests completed successfully!");
    Ok(())
}