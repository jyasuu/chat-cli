use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct MockLLMClient {
    conversation_history: Vec<MockMessage>,
    system_prompt: Option<String>,
    responses: Vec<String>,
    response_index: usize,
    streaming_enabled: bool,
    delay_ms: u64,
    function_calls: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
struct MockMessage {
    role: String,
    content: String,
    function_call: Option<serde_json::Value>,
}

impl MockLLMClient {
    /// Create a new mock LLM client with default responses
    pub fn new() -> Self {
        Self {
            conversation_history: Vec::new(),
            system_prompt: None,
            responses: vec![
                "Hello! I'm a mock LLM for testing purposes.".to_string(),
                "This is a simulated response from the mock LLM.".to_string(),
                "I can help you test your application without making real API calls.".to_string(),
                "Mock response: Your request has been processed successfully.".to_string(),
                "Testing mode: This is an automated response.".to_string(),
            ],
            response_index: 0,
            streaming_enabled: true,
            delay_ms: 50, // Small delay to simulate network latency
            function_calls: HashMap::new(),
        }
    }

    /// Create a mock client with custom responses
    pub fn with_responses(responses: Vec<String>) -> Self {
        let mut client = Self::new();
        client.responses = responses;
        client
    }

    /// Set whether streaming should be enabled
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.streaming_enabled = enabled;
        self
    }

    /// Set the delay between chunks in streaming mode
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Add a predefined function call response
    pub fn add_function_call_response(&mut self, trigger_text: &str, function_call: serde_json::Value) {
        self.function_calls.insert(trigger_text.to_lowercase(), function_call);
    }

    /// Get the next response (cycles through available responses)
    fn get_next_response(&mut self, message: &str) -> (String, Option<serde_json::Value>) {
        // Check if message should trigger a function call
        let message_lower = message.to_lowercase();
        for (trigger, function_call) in &self.function_calls {
            if message_lower.contains(trigger) {
                let response = format!("I need to call a function to help with: {}", message);
                return (response, Some(function_call.clone()));
            }
        }

        // Return regular response
        let response = if self.responses.is_empty() {
            "Mock LLM: No responses configured".to_string()
        } else {
            let response = self.responses[self.response_index % self.responses.len()].clone();
            self.response_index += 1;
            
            // Add some context based on the message
            if message.to_lowercase().contains("test") {
                format!("Mock LLM (Test Mode): {}", response)
            } else if message.to_lowercase().contains("error") {
                "Mock LLM: Simulating error handling scenario".to_string()
            } else {
                format!("Mock LLM: {}", response)
            }
        };

        (response, None)
    }

    /// Split response into chunks for streaming
    fn split_into_chunks(&self, text: &str) -> Vec<String> {
        // Split by words to simulate realistic streaming
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        let chunk_size = 3; // 3 words per chunk

        for chunk_words in words.chunks(chunk_size) {
            let chunk = chunk_words.join(" ");
            chunks.push(if chunks.is_empty() { chunk } else { format!(" {}", chunk) });
        }

        if chunks.is_empty() {
            chunks.push(text.to_string());
        }

        chunks
    }

    pub fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        self.system_prompt = Some(prompt_content.to_string());
        Ok(())
    }
    
    pub fn set_available_tools(&mut self, _tools: Vec<crate::function_calling::ToolDefinition>) {
        // Mock client doesn't need to store tools, but we implement the interface
    }

    pub fn add_user_message(&mut self, message: &str) {
        self.conversation_history.push(MockMessage {
            role: "user".to_string(),
            content: message.to_string(),
            function_call: None,
        });
    }

    pub fn add_function_response(&mut self, function_response: &crate::function_calling::FunctionResponse) {
        self.conversation_history.push(MockMessage {
            role: "function".to_string(),
            content: function_response.response.to_string(),
            function_call: None,
        });
    }

    pub fn add_model_response(&mut self, response: &str, function_call: Option<serde_json::Value>) {
        self.conversation_history.push(MockMessage {
            role: "assistant".to_string(),
            content: response.to_string(),
            function_call,
        });
    }

    pub fn clear_conversation(&mut self) {
        self.conversation_history.clear();
        self.response_index = 0;
    }

    pub async fn send_message(&mut self, message: &str) -> Result<String> {
        // Add user message to history
        self.add_user_message(message);

        // Simulate some processing delay
        tokio::time::sleep(Duration::from_millis(self.delay_ms * 2)).await;

        // Get response
        let (response, function_call) = self.get_next_response(message);

        // Add model response to history
        self.add_model_response(&response, function_call);

        Ok(response)
    }

    pub async fn send_message_stream(&mut self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        // Add user message to history
        self.add_user_message(message);

        let (response, function_call) = self.get_next_response(message);
        let chunks = self.split_into_chunks(&response);
        let delay_ms = self.delay_ms;

        let (tx, rx) = mpsc::channel(100);

        // Clone function_call for the async task
        let function_call_for_task = function_call.clone();

        // Spawn task to send chunks
        tokio::spawn(async move {
            for (i, chunk) in chunks.iter().enumerate() {
                // Add delay between chunks
                if i > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }

                // Send function call with the last chunk
                let fc = if i == chunks.len() - 1 { function_call_for_task.clone() } else { None };

                if tx.send((chunk.clone(), fc)).await.is_err() {
                    break; // Receiver dropped
                }
            }
        });

        // Add the complete response to history
        self.add_model_response(&response, function_call);

        Ok(rx)
    }

    /// Get conversation history for debugging
    pub fn get_conversation_history(&self) -> &[MockMessage] {
        &self.conversation_history
    }

    /// Get current system prompt
    pub fn get_system_prompt(&self) -> Option<&String> {
        self.system_prompt.as_ref()
    }
}

impl Default for MockLLMClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::chat_client::ChatClient for MockLLMClient {
    fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        self.load_system_prompt(prompt_content)
    }
    
    fn set_available_tools(&mut self, tools: Vec<crate::function_calling::ToolDefinition>) {
        self.set_available_tools(tools)
    }

    fn add_user_message(&mut self, message: &str) {
        self.add_user_message(message)
    }

    fn add_function_response(&mut self, function_response: &crate::function_calling::FunctionResponse) {
        self.add_function_response(function_response)
    }

    fn add_model_response(&mut self, response: &str, function_call: Option<serde_json::Value>) {
        self.add_model_response(response, function_call)
    }

    fn clear_conversation(&mut self) {
        self.clear_conversation()
    }

    async fn send_message(&self, message: &str) -> Result<String> {
        // Clone self to make it mutable for the mock
        let mut mock_self = self.clone();
        
        // Add user message to history
        mock_self.add_user_message(message);

        // Simulate some processing delay
        tokio::time::sleep(Duration::from_millis(mock_self.delay_ms * 2)).await;

        // Get response
        let (response, function_call) = mock_self.get_next_response(message);

        // Add model response to history
        mock_self.add_model_response(&response, function_call);

        Ok(response)
    }

    async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        // Clone self to make it mutable for the mock
        let mut mock_self = self.clone();
        
        // Add user message to history
        mock_self.add_user_message(message);

        let (response, function_call) = mock_self.get_next_response(message);
        let chunks = mock_self.split_into_chunks(&response);
        let delay_ms = mock_self.delay_ms;

        let (tx, rx) = mpsc::channel(100);

        // Clone function_call for the async task
        let function_call_for_task = function_call.clone();

        // Spawn task to send chunks
        tokio::spawn(async move {
            for (i, chunk) in chunks.iter().enumerate() {
                // Add delay between chunks
                if i > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }

                // Send function call with the last chunk
                let fc = if i == chunks.len() - 1 { function_call_for_task.clone() } else { None };

                if tx.send((chunk.clone(), fc)).await.is_err() {
                    break; // Receiver dropped
                }
            }
        });

        // Add the complete response to history
        mock_self.add_model_response(&response, function_call);

        Ok(rx)
    }

    fn client_name(&self) -> &str {
        "MockLLM"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_client::ChatClient;

    #[tokio::test]
    async fn test_mock_llm_basic_response() {
        let client = MockLLMClient::new();
        let response = client.send_message("Hello").await.unwrap();
        assert!(response.contains("Mock LLM"));
    }

    #[tokio::test]
    async fn test_mock_llm_custom_responses() {
        let responses = vec![
            "Custom response 1".to_string(),
            "Custom response 2".to_string(),
        ];
        let client = MockLLMClient::with_responses(responses);
        
        let response1 = client.send_message("Test 1").await.unwrap();
        let response2 = client.send_message("Test 2").await.unwrap();
        
        assert!(response1.contains("Custom response 1"));
        assert!(response2.contains("Custom response 2"));
    }

    #[tokio::test]
    async fn test_mock_llm_streaming() {
        let client = MockLLMClient::new().with_delay(1); // Fast for testing
        let mut rx = client.send_message_stream("Hello").await.unwrap();
        
        let mut chunks = Vec::new();
        while let Some((chunk, _)) = rx.recv().await {
            chunks.push(chunk);
        }
        
        assert!(!chunks.is_empty());
        let full_response = chunks.join("");
        assert!(full_response.contains("Mock LLM"));
    }

    #[tokio::test]
    async fn test_mock_llm_function_calls() {
        let mut client = MockLLMClient::new();
        
        // Add a function call response
        let function_call = serde_json::json!({
            "name": "get_weather",
            "args": {"location": "San Francisco"}
        });
        client.add_function_call_response("weather", function_call.clone());
        
        let _response = client.send_message("What's the weather like?").await.unwrap();
        
        // Check that the last message in history has the function call
        let history = client.get_conversation_history();
        let last_message = history.last().unwrap();
        assert_eq!(last_message.role, "assistant");
        assert!(last_message.function_call.is_some());
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let client = MockLLMClient::new();
        
        let mut client_mut = client.clone();
        client_mut.add_user_message("Hello");
        let _ = client_mut.send_message("How are you?").await.unwrap();
        
        let history = client_mut.get_conversation_history();
        assert_eq!(history.len(), 3); // "Hello", "How are you?", and response
        assert_eq!(history[0].role, "user");
        assert_eq!(history[1].role, "user");
        assert_eq!(history[2].role, "assistant");
    }
}