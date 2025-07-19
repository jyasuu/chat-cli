use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::Result;
use serde::Deserialize;
use async_trait::async_trait;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct SaveMemoryParams {
    fact: String,
}

pub struct SaveMemoryTool;

impl SaveMemoryTool {
    pub fn new() -> Self {
        Self
    }
    
    fn get_memory_file_path(&self) -> &'static str {
        "chat_memory.txt"
    }
}

#[async_trait]
impl Tool for SaveMemoryTool {
    fn name(&self) -> &str {
        "save_memory"
    }
    
    fn description(&self) -> &str {
        "Saves a specific piece of information or fact to your long-term memory. Use this tool when the user explicitly asks you to remember something or when the user states a clear, concise fact about themselves, their preferences, or their environment that seems important for you to retain for future interactions."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "fact": {
                        "description": "The specific fact or piece of information to remember. Should be a clear, self-contained statement.",
                        "type": "string"
                    }
                },
                "required": ["fact"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: SaveMemoryParams = serde_json::from_value(params.clone())?;
        
        if parsed.fact.trim().is_empty() {
            return Err(anyhow::anyhow!("Fact cannot be empty"));
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<SaveMemoryParams>(params.clone()) {
            let preview = if parsed.fact.len() > 50 {
                format!("{}...", &parsed.fact[..50])
            } else {
                parsed.fact.clone()
            };
            format!("Save to memory: {}", preview)
        } else {
            "Save fact to memory".to_string()
        }
    }
    
    fn tool_locations(&self, _params: &serde_json::Value) -> Vec<ToolLocation> {
        vec![ToolLocation {
            path: self.get_memory_file_path().to_string(),
            line: None,
        }]
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: SaveMemoryParams = serde_json::from_value(params.clone())?;
        let memory_file = self.get_memory_file_path();
        
        // Create timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        
        // Prepare the memory entry
        let memory_entry = format!("[{}] {}\n", timestamp, parsed.fact);
        
        // Check if file exists to determine if this is the first entry
        let file_exists = Path::new(memory_file).exists();
        let existing_size = if file_exists {
            fs::metadata(memory_file)?.len()
        } else {
            0
        };
        
        // Append to memory file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(memory_file)?;
        
        file.write_all(memory_entry.as_bytes())?;
        file.flush()?;
        
        let new_size = fs::metadata(memory_file)?.len();
        
        let summary = format!("Saved fact to memory ({})", 
            if file_exists { "appended" } else { "created new file" });
        
        let llm_content = serde_json::json!({
            "fact": parsed.fact,
            "timestamp": timestamp.to_string(),
            "memory_file": memory_file,
            "file_existed": file_exists,
            "previous_size": existing_size,
            "new_size": new_size,
            "bytes_added": new_size - existing_size
        });
        
        let display_lines = vec![
            format!("Memory saved successfully"),
            format!("File: {}", memory_file),
            format!("Timestamp: {}", timestamp),
            format!("Fact: {}", parsed.fact),
            format!("File size: {} bytes (was {} bytes)", new_size, existing_size),
        ];
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}