use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::stream::StreamExt;
use tokio::sync::mpsc;
use std::fs::OpenOptions;
use std::io::Write;
use async_trait::async_trait;
use crate::function_calling::ToolDefinition;

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    conversation_history: Vec<Message>,
    system_message: Option<String>,
    available_tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Array(Vec<ContentPart>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ToolFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    index: i32,
    message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

// Streaming response structures
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionChunk {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChunkChoice {
    index: i32,
    delta: Delta,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeltaToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<DeltaFunction>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeltaFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<String>,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
            conversation_history: Vec::new(),
            system_message: None,
            available_tools: Vec::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        self.system_message = Some(prompt_content.to_string());
        Ok(())
    }
    
    pub fn set_available_tools(&mut self, tools: Vec<ToolDefinition>) {
        self.available_tools = tools;
    }

    pub fn add_user_message(&mut self, message: &str) {
        self.conversation_history.push(Message {
            role: "user".to_string(),
            content: MessageContent::Text(message.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    pub fn add_function_response(&mut self, function_response: &crate::function_calling::FunctionResponse) {
        self.conversation_history.push(Message {
            role: "tool".to_string(),
            content: MessageContent::Text(serde_json::to_string(&function_response.response).unwrap_or_else(|_| "Error serializing response".to_string())),
            name: Some(function_response.name.clone()),
            tool_calls: None,
            tool_call_id: Some(function_response.id.clone()),
        });
    }

    pub fn add_model_response(&mut self, response: &str, tool_calls: Option<Vec<ToolCall>>) {
        self.conversation_history.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::Text(response.to_string()),
            name: None,
            tool_calls,
            tool_call_id: None,
        });
    }

    pub fn clear_conversation(&mut self) {
        self.conversation_history.clear();
    }

    fn build_messages(&self, user_message: Option<&str>) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(system_msg) = &self.system_message {
            messages.push(Message {
                role: "system".to_string(),
                content: MessageContent::Text(system_msg.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Add conversation history
        messages.extend(self.conversation_history.clone());

        // Add new user message if provided
        if let Some(msg) = user_message {
            messages.push(Message {
                role: "user".to_string(),
                content: MessageContent::Text(msg.to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages
    }

    fn build_tools(&self) -> Option<Vec<Tool>> {
        if self.available_tools.is_empty() {
            return None;
        }

        Some(
            self.available_tools
                .iter()
                .map(|tool| Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect(),
        )
    }

    #[allow(dead_code)]
    pub async fn send_message(&self, message: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let messages = self.build_messages(if message.is_empty() {None} else{Some(message)});
        let tools = self.build_tools();

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.7),
            top_p: Some(0.95),
            max_tokens: Some(2048),
            stream: Some(false),
            tools,
            tool_choice: None,
        };

        // Log the request payload
        if let Ok(request_json) = serde_json::to_string_pretty(&request) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("tmp_rovodev_openai_request_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] OPENAI REQUEST PAYLOAD:", timestamp);
                let _ = writeln!(file, "{}", request_json);
                let _ = writeln!(file, "---");
            }
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let api_response: ChatCompletionResponse = response.json().await?;

        if api_response.choices.is_empty() {
            return Err(anyhow!("No response from API"));
        }

        let choice = &api_response.choices[0];
        match &choice.message.content {
            MessageContent::Text(text) => Ok(text.clone()),
            MessageContent::Array(parts) => {
                // Concatenate all text parts
                let text = parts
                    .iter()
                    .map(|part| part.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                Ok(text)
            }
        }
    }

    pub async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        let url = format!("{}/chat/completions", self.base_url);

        let messages = self.build_messages(if message.is_empty() {None} else{Some(message)});
        let tools = self.build_tools();

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.7),
            top_p: Some(0.95),
            max_tokens: Some(2048),
            stream: Some(true),
            tools,
            tool_choice: None,
        };

        // Log the request payload
        if let Ok(request_json) = serde_json::to_string_pretty(&request) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("tmp_rovodev_openai_request_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] OPENAI STREAMING REQUEST PAYLOAD:", timestamp);
                let _ = writeln!(file, "{}", request_json);
                let _ = writeln!(file, "---");
            }
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let (tx, rx) = mpsc::channel::<(String, Option<serde_json::Value>)>(1000);
        
        // Add logging function
        let log_debug = |msg: &str| {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("tmp_rovodev_openai_streaming_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] {}", timestamp, msg);
            }
        };
        
        log_debug("Starting OpenAI streaming task");
        
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut chunk_count = 0;
            let mut current_tool_calls: Vec<ToolCall> = Vec::new();
            
            log_debug("Stream created, starting to read chunks");
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        chunk_count += 1;
                        log_debug(&format!("Received chunk #{}: {} bytes", chunk_count, chunk.len()));
                        
                        if let Ok(chunk_str) = String::from_utf8(chunk.to_vec()) {
                            log_debug(&format!("Chunk content: {:?}", chunk_str));
                            buffer.push_str(&chunk_str);
                            
                            // Process complete SSE events (looking for double newlines)
                            while let Some(event_end) = buffer.find("\n\n") {
                                let event = buffer[..event_end].to_string();
                                buffer = buffer[event_end + 2..].to_string();
                                
                                log_debug(&format!("Processing SSE event: {:?}", event));
                                
                                // Parse SSE event - look for data lines
                                for line in event.lines() {
                                    let line = line.trim();
                                    if line.starts_with("data: ") {
                                        let json_data = &line[6..]; // Remove "data: " prefix
                                        log_debug(&format!("Found data line: {:?}", json_data));
                                        
                                        // Skip empty data or [DONE] messages
                                        if json_data.trim().is_empty() || json_data.trim() == "[DONE]" {
                                            log_debug("Skipping empty or [DONE] message");
                                            continue;
                                        }
                                        
                                        // Try to parse the JSON response
                                        log_debug(&format!("Attempting to parse JSON: {}", json_data));
                                        match serde_json::from_str::<ChatCompletionChunk>(json_data) {
                                            Ok(chunk_data) => {
                                                log_debug(&format!("Successfully parsed JSON, choices: {}", chunk_data.choices.len()));
                                                if !chunk_data.choices.is_empty() {
                                                    let choice = &chunk_data.choices[0];
                                                    let delta = &choice.delta;
                                                    
                                                    let mut text_content = String::new();
                                                    let mut function_call = None;
                                                    
                                                    // Handle text content
                                                    if let Some(content) = &delta.content {
                                                        text_content = content.clone();
                                                    }
                                                    
                                                    // Handle tool calls
                                                    if let Some(tool_calls) = &delta.tool_calls {
                                                        for delta_tool_call in tool_calls {
                                                            let index = delta_tool_call.index.unwrap_or(0) as usize;
                                                            
                                                            // Ensure we have enough space in the vector
                                                            while current_tool_calls.len() <= index {
                                                                current_tool_calls.push(ToolCall {
                                                                    id: format!("call_{}", chrono::Utc::now().timestamp_millis()),
                                                                    call_type: "function".to_string(),
                                                                    function: FunctionCall {
                                                                        name: String::new(),
                                                                        arguments: String::new(),
                                                                    },
                                                                });
                                                            }
                                                            
                                                            let tool_call = &mut current_tool_calls[index];
                                                            
                                                            if let Some(id) = &delta_tool_call.id {
                                                                if !id.is_empty() {
                                                                    tool_call.id = id.clone();
                                                                }
                                                            }
                                                            
                                                            if let Some(call_type) = &delta_tool_call.call_type {
                                                                tool_call.call_type = call_type.clone();
                                                            }
                                                            
                                                            if let Some(function) = &delta_tool_call.function {
                                                                if let Some(name) = &function.name {
                                                                    tool_call.function.name = name.clone();
                                                                }
                                                                if let Some(arguments) = &function.arguments {
                                                                    tool_call.function.arguments.push_str(arguments);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Check if we have complete tool calls to send
                                                    if choice.finish_reason.as_deref() == Some("tool_calls") {
                                                        // Convert to the format expected by the function calling system
                                                        for tool_call in &current_tool_calls {
                                                            if !tool_call.function.name.is_empty() {
                                                                // Parse the arguments JSON string
                                                                let args = if tool_call.function.arguments.is_empty() {
                                                                    serde_json::json!({})
                                                                } else {
                                                                    match serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
                                                                        Ok(parsed) => parsed,
                                                                        Err(_) => {
                                                                            log_debug(&format!("Failed to parse tool call arguments: {}", tool_call.function.arguments));
                                                                            serde_json::json!({})
                                                                        }
                                                                    }
                                                                };
                                                                
                                                                function_call = Some(serde_json::json!({
                                                                    "name": tool_call.function.name,
                                                                    "args": args
                                                                }));
                                                                
                                                                log_debug(&format!("OPENAI: Created function call: {:?}", function_call));
                                                                break; // Only send the first tool call for now
                                                            }
                                                        }
                                                    }
                                                    
                                                    if !text_content.is_empty() || function_call.is_some() {
                                                        log_debug(&format!("OPENAI: Sending content to channel: text={:?}, function_call={:?}", text_content, function_call));
                                                        match tx.send((text_content, function_call)).await {
                                                            Ok(_) => {
                                                                log_debug("OPENAI: Content sent successfully to channel");
                                                            }
                                                            Err(e) => {
                                                                log_debug(&format!("OPENAI: Receiver dropped, stopping stream: {}", e));
                                                                return; // Receiver dropped
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    log_debug("No choices in chunk");
                                                }
                                            }
                                            Err(e) => {
                                                // Send error message for debugging
                                                let error_msg = format!("JSON parse error: {} - Data: {}", e, json_data);
                                                log_debug(&format!("JSON parse error: {}", error_msg));
                                                if tx.send((error_msg, None)).await.is_err() {
                                                    log_debug("Failed to send error message, receiver dropped");
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Send error message
                        let error_msg = format!("Stream error: {}", e);
                        log_debug(&format!("Stream error: {}", error_msg));
                        let _ = tx.send((error_msg, None)).await;
                        break;
                    }
                }
            }
            
            log_debug("Stream processing completed");
            // Explicitly drop the sender to signal completion
            drop(tx);
        });

        Ok(rx)
    }
}

#[async_trait]
impl crate::chat_client::ChatClient for OpenAIClient {
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
        // Convert function call format for OpenAI
        let tool_calls = function_call.map(|fc| {
            vec![ToolCall {
                id: format!("call_{}", chrono::Utc::now().timestamp_millis()),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: fc.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                    arguments: fc.get("args").map(|v| v.to_string()).unwrap_or_default(),
                },
            }]
        });
        self.add_model_response(response, tool_calls)
    }
    
    fn clear_conversation(&mut self) {
        self.clear_conversation()
    }
    
    async fn send_message(&self, message: &str) -> Result<String> {
        self.send_message(message).await
    }
    
    async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        self.send_message_stream(message).await
    }
    
    fn client_name(&self) -> &str {
        "OpenAI"
    }
}