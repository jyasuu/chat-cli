use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::function_calling::{FunctionResponse, ToolDefinition};

/// Generic trait for chat clients that can communicate with different LLM providers
#[async_trait]
pub trait ChatClient {
    /// Load a system prompt for the client
    fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()>;
    
    /// Set available tools for function calling
    fn set_available_tools(&mut self, tools: Vec<ToolDefinition>);
    
    /// Add a user message to the conversation history
    fn add_user_message(&mut self, message: &str);
    
    /// Add a function response to the conversation history
    fn add_function_response(&mut self, function_response: &FunctionResponse);
    
    /// Add a model response to the conversation history
    /// The function_call parameter format may vary between providers but should be JSON
    fn add_model_response(&mut self, response: &str, function_calls: Vec<serde_json::Value>);
    
    /// Clear the conversation history
    fn clear_conversation(&mut self);
    
    /// Send a message and get a non-streaming response
    async fn send_message(&self, message: &str) -> Result<String>;
    
    /// Send a message and get a streaming response
    /// Returns a receiver that yields (text_chunk, optional_function_call) tuples
    async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>>;
    
    /// Get the name of the client (for display purposes)
    fn client_name(&self) -> &str;
}

/// Wrapper enum that implements ChatClient for different provider clients
pub enum AnyChatClient {
    Gemini(crate::gemini::GeminiClient),
    OpenAI(crate::openai::OpenAIClient),
    Mock(crate::mock_llm::MockLLMClient),
}

#[async_trait]
impl ChatClient for AnyChatClient {
    fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        match self {
            AnyChatClient::Gemini(client) => client.load_system_prompt(prompt_content),
            AnyChatClient::OpenAI(client) => client.load_system_prompt(prompt_content),
            AnyChatClient::Mock(client) => client.load_system_prompt(prompt_content),
        }
    }
    
    fn set_available_tools(&mut self, tools: Vec<ToolDefinition>) {
        match self {
            AnyChatClient::Gemini(client) => client.set_available_tools(tools),
            AnyChatClient::OpenAI(client) => client.set_available_tools(tools),
            AnyChatClient::Mock(client) => client.set_available_tools(tools),
        }
    }
    
    fn add_user_message(&mut self, message: &str) {
        match self {
            AnyChatClient::Gemini(client) => client.add_user_message(message),
            AnyChatClient::OpenAI(client) => client.add_user_message(message),
            AnyChatClient::Mock(client) => client.add_user_message(message),
        }
    }
    
    fn add_function_response(&mut self, function_response: &FunctionResponse) {
        match self {
            AnyChatClient::Gemini(client) => client.add_function_response(function_response),
            AnyChatClient::OpenAI(client) => client.add_function_response(function_response),
            AnyChatClient::Mock(client) => client.add_function_response(function_response),
        }
    }
    
    fn add_model_response(&mut self, response: &str, function_calls: Vec<serde_json::Value>) {
        match self {
            AnyChatClient::Gemini(client) => client.add_model_response(response, function_calls),
            AnyChatClient::OpenAI(client) => client.add_model_response(response, function_calls),
            AnyChatClient::Mock(client) => client.add_model_response(response, function_calls),
        }
    }
    
    fn clear_conversation(&mut self) {
        match self {
            AnyChatClient::Gemini(client) => client.clear_conversation(),
            AnyChatClient::OpenAI(client) => client.clear_conversation(),
            AnyChatClient::Mock(client) => client.clear_conversation(),
        }
    }
    
    async fn send_message(&self, message: &str) -> Result<String> {
        match self {
            AnyChatClient::Gemini(client) => client.send_message(message).await,
            AnyChatClient::OpenAI(client) => client.send_message(message).await,
            AnyChatClient::Mock(client) => client.send_message(message).await,
        }
    }
    
    async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        match self {
            AnyChatClient::Gemini(client) => client.send_message_stream(message).await,
            AnyChatClient::OpenAI(client) => client.send_message_stream(message).await,
            AnyChatClient::Mock(client) => client.send_message_stream(message).await,
        }
    }
    
    fn client_name(&self) -> &str {
        match self {
            AnyChatClient::Gemini(_) => "Gemini",
            AnyChatClient::OpenAI(_) => "OpenAI",
            AnyChatClient::Mock(_) => "MockLLM",
        }
    }
}

impl AnyChatClient {
    /// Create a new Gemini client
    pub fn new_gemini(api_key: String, model: String) -> Self {
        AnyChatClient::Gemini(crate::gemini::GeminiClient::new(api_key, model))
    }
    
    /// Create a new OpenAI client
    pub fn new_openai(api_key: String, model: String) -> Self {
        AnyChatClient::OpenAI(crate::openai::OpenAIClient::new(api_key, model))
    }
    
    /// Create a new OpenAI client with custom base URL
    pub fn new_openai_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        AnyChatClient::OpenAI(crate::openai::OpenAIClient::new(api_key, model).with_base_url(base_url))
    }
    
    /// Create a new Mock LLM client
    pub fn new_mock() -> Self {
        AnyChatClient::Mock(crate::mock_llm::MockLLMClient::new())
    }
    
    /// Create a new Mock LLM client with custom responses
    pub fn new_mock_with_responses(responses: Vec<String>) -> Self {
        AnyChatClient::Mock(crate::mock_llm::MockLLMClient::with_responses(responses))
    }
}