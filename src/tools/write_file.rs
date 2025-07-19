use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use async_trait::async_trait;

#[derive(Debug, Deserialize)]
struct WriteFileParams {
    file_path: String,
    content: String,
}

pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    
    fn description(&self) -> &str {
        "Writes content to a specified file in the local filesystem."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "description": "The absolute path to the file to write to (e.g., '/home/user/project/file.txt'). Relative paths are not supported.",
                        "type": "string"
                    },
                    "content": {
                        "description": "The content to write to the file.",
                        "type": "string"
                    }
                },
                "required": ["file_path", "content"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: WriteFileParams = serde_json::from_value(params.clone())?;
        
        let path = Path::new(&parsed.file_path);
        if !path.is_absolute() {
            return Err(anyhow!("File path must be absolute, but was relative: {}", parsed.file_path));
        }
        
        // Check if parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(anyhow!("Parent directory does not exist: {}", parent.display()));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<WriteFileParams>(params.clone()) {
            let path = Path::new(&parsed.file_path);
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let content_size = parsed.content.len();
            let line_count = parsed.content.lines().count();
            
            format!("Write {} bytes ({} lines) to {}", content_size, line_count, filename)
        } else {
            "Write file".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<WriteFileParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.file_path,
                line: None,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: WriteFileParams = serde_json::from_value(params.clone())?;
        let path = Path::new(&parsed.file_path);
        
        let file_existed = path.exists();
        let original_size = if file_existed {
            fs::metadata(path)?.len()
        } else {
            0
        };
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, &parsed.content)?;
        
        let new_size = parsed.content.len() as u64;
        let line_count = parsed.content.lines().count();
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        
        let summary = if file_existed {
            format!("Updated {} ({} bytes, {} lines)", filename, new_size, line_count)
        } else {
            format!("Created {} ({} bytes, {} lines)", filename, new_size, line_count)
        };
        
        let llm_content = serde_json::json!({
            "file_path": parsed.file_path,
            "operation": if file_existed { "updated" } else { "created" },
            "bytes_written": new_size,
            "lines_written": line_count,
            "original_size": original_size
        });
        
        let display_lines = vec![
            format!("File: {}", parsed.file_path),
            format!("Operation: {}", if file_existed { "Updated" } else { "Created" }),
            format!("Size: {} bytes ({} lines)", new_size, line_count),
            if file_existed {
                format!("Previous size: {} bytes", original_size)
            } else {
                "New file created".to_string()
            },
        ];
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}