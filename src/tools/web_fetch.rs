use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use regex::Regex;

#[derive(Debug, Deserialize)]
struct WebFetchParams {
    prompt: String,
}

pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
    
    fn extract_urls(&self, prompt: &str) -> Vec<String> {
        let url_regex = Regex::new(r"https?://[^\s]+").unwrap();
        url_regex
            .find_iter(prompt)
            .map(|m| m.as_str().to_string())
            .collect()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }
    
    fn description(&self) -> &str {
        "Processes content from URL(s), including local and private network addresses (e.g., localhost), embedded in a prompt. Include up to 20 URLs and instructions (e.g., summarize, extract specific data) directly in the 'prompt' parameter."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "description": "A comprehensive prompt that includes the URL(s) (up to 20) to fetch and specific instructions on how to process their content (e.g., \"Summarize https://example.com/article and extract key points from https://another.com/data\"). Must contain as least one URL starting with http:// or https://.",
                        "type": "string"
                    }
                },
                "required": ["prompt"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: WebFetchParams = serde_json::from_value(params.clone())?;
        
        let urls = self.extract_urls(&parsed.prompt);
        if urls.is_empty() {
            return Err(anyhow!("Prompt must contain at least one URL starting with http:// or https://"));
        }
        
        if urls.len() > 20 {
            return Err(anyhow!("Maximum 20 URLs allowed, found {}", urls.len()));
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<WebFetchParams>(params.clone()) {
            let urls = self.extract_urls(&parsed.prompt);
            format!("Fetch and process {} URL(s)", urls.len())
        } else {
            "Fetch web content".to_string()
        }
    }
    
    fn tool_locations(&self, _params: &serde_json::Value) -> Vec<ToolLocation> {
        Vec::new() // Web URLs don't have file system locations
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: WebFetchParams = serde_json::from_value(params.clone())?;
        let urls = self.extract_urls(&parsed.prompt);
        
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; ChatCLI/1.0)")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let mut fetched_content = Vec::new();
        let mut successful_fetches = 0;
        let mut failed_fetches = 0;
        
        for url in &urls {
            match client.get(url).send().await {
                Ok(response) => {
                    match response.text().await {
                        Ok(content) => {
                            // Truncate very long content
                            let content_len = content.len();
                            let truncated_content = if content_len > 50000 {
                                format!("{}...\n[Content truncated - original length: {} characters]", 
                                    &content[..50000], content_len)
                            } else {
                                content
                            };
                            
                            fetched_content.push(serde_json::json!({
                                "url": url,
                                "status": "success",
                                "content": truncated_content,
                                "content_length": content_len
                            }));
                            successful_fetches += 1;
                        }
                        Err(e) => {
                            fetched_content.push(serde_json::json!({
                                "url": url,
                                "status": "error",
                                "error": format!("Failed to read response body: {}", e)
                            }));
                            failed_fetches += 1;
                        }
                    }
                }
                Err(e) => {
                    fetched_content.push(serde_json::json!({
                        "url": url,
                        "status": "error",
                        "error": format!("Failed to fetch URL: {}", e)
                    }));
                    failed_fetches += 1;
                }
            }
        }
        
        let summary = format!("Fetched {} URLs ({} successful, {} failed)", 
            urls.len(), successful_fetches, failed_fetches);
        
        let llm_content = serde_json::json!({
            "prompt": parsed.prompt,
            "urls_found": urls,
            "total_urls": urls.len(),
            "successful_fetches": successful_fetches,
            "failed_fetches": failed_fetches,
            "fetched_content": fetched_content
        });
        
        let mut display_lines = vec![
            format!("Web Fetch Results"),
            format!("URLs processed: {}", urls.len()),
            format!("Successful: {}, Failed: {}", successful_fetches, failed_fetches),
            "".to_string(),
        ];
        
        for content in &fetched_content {
            if let Some(url) = content.get("url").and_then(|v| v.as_str()) {
                if let Some(status) = content.get("status").and_then(|v| v.as_str()) {
                    match status {
                        "success" => {
                            if let Some(length) = content.get("content_length").and_then(|v| v.as_u64()) {
                                display_lines.push(format!("✅ {} ({} characters)", url, length));
                            } else {
                                display_lines.push(format!("✅ {}", url));
                            }
                        }
                        "error" => {
                            if let Some(error) = content.get("error").and_then(|v| v.as_str()) {
                                display_lines.push(format!("❌ {}: {}", url, error));
                            } else {
                                display_lines.push(format!("❌ {}", url));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if successful_fetches > 0 {
            display_lines.push("".to_string());
            display_lines.push("Note: Content has been fetched and is available for processing.".to_string());
        }
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}