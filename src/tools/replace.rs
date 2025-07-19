use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ReplaceParams {
    file_path: String,
    old_string: String,
    new_string: String,
    expected_replacements: Option<usize>,
}

pub struct ReplaceTool;

impl ReplaceTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReplaceTool {
    fn name(&self) -> &str {
        "replace"
    }
    
    fn description(&self) -> &str {
        "Replaces text within a file. By default, replaces a single occurrence, but can replace multiple occurrences when `expected_replacements` is specified. This tool requires providing significant context around the change to ensure precise targeting."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "description": "The absolute path to the file to modify. Must start with '/'.",
                        "type": "string"
                    },
                    "old_string": {
                        "description": "The exact literal text to replace, preferably unescaped. For single replacements (default), include at least 3 lines of context BEFORE and AFTER the target text, matching whitespace and indentation precisely. For multiple replacements, specify expected_replacements parameter. If this string is not the exact literal text (i.e. you escaped it) or does not match exactly, the tool will fail.",
                        "type": "string"
                    },
                    "new_string": {
                        "description": "The exact literal text to replace `old_string` with, preferably unescaped. Provide the EXACT text. Ensure the resulting code is correct and idiomatic.",
                        "type": "string"
                    },
                    "expected_replacements": {
                        "description": "Number of replacements expected. Defaults to 1 if not specified. Use when you want to replace multiple occurrences.",
                        "type": "number",
                        "minimum": 1
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: ReplaceParams = serde_json::from_value(params.clone())?;
        
        let path = Path::new(&parsed.file_path);
        if !path.is_absolute() {
            return Err(anyhow!("File path must be absolute, but was relative: {}", parsed.file_path));
        }
        
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", parsed.file_path));
        }
        
        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", parsed.file_path));
        }
        
        if parsed.old_string.is_empty() {
            return Err(anyhow!("old_string cannot be empty"));
        }
        
        if let Some(expected) = parsed.expected_replacements {
            if expected == 0 {
                return Err(anyhow!("expected_replacements must be greater than 0"));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<ReplaceParams>(params.clone()) {
            let path = Path::new(&parsed.file_path);
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let expected = parsed.expected_replacements.unwrap_or(1);
            
            if expected == 1 {
                format!("Replace text in {}", filename)
            } else {
                format!("Replace {} occurrences in {}", expected, filename)
            }
        } else {
            "Replace text in file".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<ReplaceParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.file_path,
                line: None,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: ReplaceParams = serde_json::from_value(params.clone())?;
        let path = Path::new(&parsed.file_path);
        
        let original_content = fs::read_to_string(path)?;
        let expected_replacements = parsed.expected_replacements.unwrap_or(1);
        
        // Count actual occurrences
        let actual_count = original_content.matches(&parsed.old_string).count();
        
        if actual_count == 0 {
            return Err(anyhow!(
                "String '{}' not found in file. Please check the exact text including whitespace and line endings.",
                parsed.old_string
            ));
        }
        
        if actual_count != expected_replacements {
            return Err(anyhow!(
                "Expected {} occurrences of '{}' but found {}. Please verify the text and expected_replacements parameter.",
                expected_replacements,
                parsed.old_string,
                actual_count
            ));
        }
        
        // Perform replacement
        let new_content = if expected_replacements == 1 {
            original_content.replacen(&parsed.old_string, &parsed.new_string, 1)
        } else {
            original_content.replace(&parsed.old_string, &parsed.new_string)
        };
        
        // Write back to file
        fs::write(path, &new_content)?;
        
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let summary = if expected_replacements == 1 {
            format!("Replaced 1 occurrence in {}", filename)
        } else {
            format!("Replaced {} occurrences in {}", expected_replacements, filename)
        };
        
        // Calculate some basic diff info
        let original_lines = original_content.lines().count();
        let new_lines = new_content.lines().count();
        let size_change = new_content.len() as i64 - original_content.len() as i64;
        
        let llm_content = serde_json::json!({
            "file_path": parsed.file_path,
            "replacements_made": expected_replacements,
            "old_string": parsed.old_string,
            "new_string": parsed.new_string,
            "original_size": original_content.len(),
            "new_size": new_content.len(),
            "size_change": size_change,
            "original_lines": original_lines,
            "new_lines": new_lines
        });
        
        let display_lines = vec![
            format!("File: {}", parsed.file_path),
            format!("Replacements made: {}", expected_replacements),
            format!("Size change: {} bytes", size_change),
            format!("Lines: {} → {}", original_lines, new_lines),
            "".to_string(),
            "Replaced:".to_string(),
            format!("  {}", parsed.old_string.replace('\n', "\\n")),
            "With:".to_string(),
            format!("  {}", parsed.new_string.replace('\n', "\\n")),
        ];
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}