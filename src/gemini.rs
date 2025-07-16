use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::stream::StreamExt;
use tokio::sync::mpsc;
use std::fs::OpenOptions;
use std::io::Write;
use async_trait::async_trait;

#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    conversation_history: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Content {
    parts: Vec<Part>,
    role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<serde_json::Value>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: Option<GenerationConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "topK")]
    top_k: i32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata", skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<serde_json::Value>,
    #[serde(rename = "modelVersion", skip_serializing_if = "Option::is_none")]
    model_version: Option<String>,
    #[serde(rename = "responseId", skip_serializing_if = "Option::is_none")]
    response_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Candidate {
    content: Content,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<i32>,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            conversation_history: Vec::new(),
            system_instruction: None,
        }
    }

    pub fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        self.system_instruction = Some(SystemInstruction {
            parts: vec![Part {
                text: Some(prompt_content.to_string()),
                function_call: None,
                function_response: None,
            }],
        });
        Ok(())
    }

    pub fn add_user_message(&mut self, message: &str) {
        self.conversation_history.push(Content {
            role: "user".to_string(),
            parts: vec![Part {
                text: Some(message.to_string()),
                function_call: None,
                function_response: None,
            }],
        });
    }

    pub fn add_function_response(&mut self, function_response: &crate::function_calling::FunctionResponse) {
        self.conversation_history.push(Content {
            role: "user".to_string(),
            parts: vec![Part {
                text: None,
                function_call: None,
                function_response: Some(serde_json::json!({
                    "id": function_response.id,
                    "name": function_response.name,
                    "response": function_response.response
                })),
            }],
        });
    }

    pub fn add_model_response(&mut self, response: &str, function_call: Option<serde_json::Value>) {
        let mut parts = Vec::new();
        
        if !response.is_empty() {
            parts.push(Part {
                text: Some(response.to_string()),
                function_call: None,
                function_response: None,
            });
        }
        
        if let Some(fc) = function_call {
            parts.push(Part {
                text: None,
                function_call: Some(fc),
                function_response: None,
            });
        }

        self.conversation_history.push(Content {
            role: "model".to_string(),
            parts,
        });
    }

    pub fn clear_conversation(&mut self) {
        self.conversation_history.clear();
    }

    #[allow(dead_code)]
    pub async fn send_message(&self, message: &str) -> Result<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let mut contents = self.conversation_history.clone();
        
        if !message.is_empty()
        {
            contents.push(Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: Some(message.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            });

        }
        
        let tools = Some(vec![Tool {
            function_declarations: crate::function_calling::FunctionExecutor::get_available_tools()
                .into_iter()
                .map(|tool| FunctionDeclaration {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                })
                .collect(),
        }]);

        let request = GenerateContentRequest {
            contents,
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                top_p: 0.95,
                top_k: 40,
                max_output_tokens: 2048,
            }),
            system_instruction: self.system_instruction.clone(),
            tools,
        };

        // Log the request payload
        if let Ok(request_json) = serde_json::to_string_pretty(&request) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("tmp_rovodev_gemini_request_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] GEMINI REQUEST PAYLOAD:", timestamp);
                let _ = writeln!(file, "{}", request_json);
                let _ = writeln!(file, "---");
            }
        }

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let api_response: GenerateContentResponse = response.json().await?;

        if api_response.candidates.is_empty() {
            return Err(anyhow!("No response from API"));
        }

        let candidate = &api_response.candidates[0];
        if candidate.content.parts.is_empty() {
            return Err(anyhow!("Empty response from API"));
        }

        // Extract text from the first part that has text
        for part in &candidate.content.parts {
            if let Some(text) = &part.text {
                return Ok(text.clone());
            }
        }

        Err(anyhow!("No text content in response"))
    }

    pub async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, self.model, self.api_key
        );

        let mut contents = self.conversation_history.clone();
        if !message.is_empty()
        {
            contents.push(Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: Some(message.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            });
        }

        let tools = Some(vec![Tool {
            function_declarations: crate::function_calling::FunctionExecutor::get_available_tools()
                .into_iter()
                .map(|tool| FunctionDeclaration {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                })
                .collect(),
        }]);

        let request = GenerateContentRequest {
            contents,
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                top_p: 0.95,
                top_k: 40,
                max_output_tokens: 2048,
            }),
            system_instruction: self.system_instruction.clone(),
            tools,
        };

        // Log the request payload
        if let Ok(request_json) = serde_json::to_string_pretty(&request) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("tmp_rovodev_gemini_request_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] GEMINI STREAMING REQUEST PAYLOAD:", timestamp);
                let _ = writeln!(file, "{}", request_json);
                let _ = writeln!(file, "---");
            }
        }

        let response = self
            .client
            .post(&url)
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
                .open("tmp_rovodev_streaming_debug.log") 
            {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] {}", timestamp, msg);
            }
        };
        
        log_debug("Starting streaming task");
        
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut chunk_count = 0;
            
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
                            // Handle both \r\n\r\n and \n\n patterns
                            while let Some(event_end) = buffer.find("\r\n\r\n").or_else(|| buffer.find("\n\n")) {
                                let event = buffer[..event_end].to_string();
                                let skip_len = if buffer[event_end..].starts_with("\r\n\r\n") { 4 } else { 2 };
                                buffer = buffer[event_end + skip_len..].to_string();
                                
                                log_debug(&format!("Processing SSE event: {:?}", event));
                                
                                // Parse SSE event - look for data lines
                                for line in event.lines() {
                                    let line = line.trim(); // Remove any whitespace/carriage returns
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
                                        match serde_json::from_str::<GenerateContentResponse>(json_data) {
                                            Ok(response_data) => {
                                                log_debug(&format!("Successfully parsed JSON, candidates: {}", response_data.candidates.len()));
                                                if !response_data.candidates.is_empty() {
                                                    let candidate = &response_data.candidates[0];
                                                    if !candidate.content.parts.is_empty() {
                                                        for part in &candidate.content.parts {
                                                            let mut text_content = String::new();
                                                            let mut function_call = None;
                                                            
                                                            if let Some(text) = &part.text {
                                                                text_content = text.clone();
                                                            }
                                                            
                                                            if let Some(fc) = &part.function_call {
                                                                function_call = Some(fc.clone());
                                                            }
                                                            
                                                            if !text_content.is_empty() || function_call.is_some() {
                                                                log_debug(&format!("GEMINI: Sending content to channel: text={:?}, function_call={:?}", text_content, function_call));
                                                                match tx.send((text_content, function_call)).await {
                                                                    Ok(_) => {
                                                                        log_debug("GEMINI: Content sent successfully to channel");
                                                                    }
                                                                    Err(e) => {
                                                                        log_debug(&format!("GEMINI: Receiver dropped, stopping stream: {}", e));
                                                                        return; // Receiver dropped
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        log_debug("No parts in candidate content");
                                                    }
                                                } else {
                                                    log_debug("No candidates in response");
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
impl crate::chat_client::ChatClient for GeminiClient {
    fn load_system_prompt(&mut self, prompt_content: &str) -> Result<()> {
        self.load_system_prompt(prompt_content)
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
        self.send_message(message).await
    }
    
    async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<(String, Option<serde_json::Value>)>> {
        self.send_message_stream(message).await
    }
    
    fn client_name(&self) -> &str {
        "Gemini"
    }
}