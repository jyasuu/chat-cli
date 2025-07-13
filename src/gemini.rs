use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::stream::StreamExt;
use tokio::sync::mpsc;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: Option<GenerationConfig>,
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
}

#[derive(Debug, Serialize, Deserialize)]
struct Candidate {
    content: Content,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: message.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                top_p: 0.95,
                top_k: 40,
                max_output_tokens: 2048,
            }),
        };

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

        Ok(candidate.content.parts[0].text.clone())
    }

    pub async fn send_message_stream(&self, message: &str) -> Result<mpsc::Receiver<String>> {
        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, self.model, self.api_key
        );

        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: message.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                top_p: 0.95,
                top_k: 40,
                max_output_tokens: 2048,
            }),
        };

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

        let (tx, rx) = mpsc::channel(100);
        
        // Add logging function
        let log_debug = |msg: &str| {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("/workspace/chat-cli/tmp_rovodev_streaming_debug.log") 
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
                            while let Some(event_end) = buffer.find("\n\n") {
                                let event = buffer[..event_end].to_string();
                                buffer = buffer[event_end + 2..].to_string();
                                
                                log_debug(&format!("Processing SSE event: {:?}", event));
                                
                                // Parse SSE event - look for data lines
                                for line in event.lines() {
                                    if line.starts_with("data: ") {
                                        let json_data = &line[6..]; // Remove "data: " prefix
                                        log_debug(&format!("Found data line: {:?}", json_data));
                                        
                                        // Skip empty data or [DONE] messages
                                        if json_data.trim().is_empty() || json_data.trim() == "[DONE]" {
                                            log_debug("Skipping empty or [DONE] message");
                                            continue;
                                        }
                                        
                                        // Try to parse the JSON response
                                        match serde_json::from_str::<GenerateContentResponse>(json_data) {
                                            Ok(response_data) => {
                                                log_debug(&format!("Successfully parsed JSON, candidates: {}", response_data.candidates.len()));
                                                if !response_data.candidates.is_empty() {
                                                    let candidate = &response_data.candidates[0];
                                                    if !candidate.content.parts.is_empty() {
                                                        let text = &candidate.content.parts[0].text;
                                                        log_debug(&format!("Extracted text: {:?}", text));
                                                        if !text.is_empty() {
                                                            log_debug(&format!("Sending text to channel: {:?}", text));
                                                            if tx.send(text.clone()).await.is_err() {
                                                                log_debug("Receiver dropped, stopping stream");
                                                                return; // Receiver dropped
                                                            }
                                                            log_debug("Text sent successfully");
                                                        } else {
                                                            log_debug("Text is empty, skipping");
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
                                                if tx.send(error_msg).await.is_err() {
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
                        let _ = tx.send(error_msg).await;
                        break;
                    }
                }
            }
            
            log_debug("Stream processing completed");
        });

        Ok(rx)
    }
}