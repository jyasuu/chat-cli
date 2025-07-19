use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use glob::Pattern;
use async_trait::async_trait;

#[derive(Debug, Deserialize)]
struct ListDirectoryParams {
    path: String,
    ignore: Option<Vec<String>>,
    respect_git_ignore: Option<bool>,
}

pub struct ListDirectoryTool;

impl ListDirectoryTool {
    pub fn new() -> Self {
        Self
    }
    
    fn should_ignore(&self, entry_name: &str, ignore_patterns: &[Pattern]) -> bool {
        ignore_patterns.iter().any(|pattern| pattern.matches(entry_name))
    }
    
    fn load_gitignore_patterns(&self, dir_path: &Path) -> Vec<Pattern> {
        let gitignore_path = dir_path.join(".gitignore");
        if !gitignore_path.exists() {
            return Vec::new();
        }
        
        match fs::read_to_string(&gitignore_path) {
            Ok(content) => {
                content
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                    .filter_map(|line| Pattern::new(line.trim()).ok())
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }
}

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }
    
    fn description(&self) -> &str {
        "Lists the names of files and subdirectories directly within a specified directory path. Can optionally ignore entries matching provided glob patterns."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "description": "The absolute path to the directory to list (must be absolute, not relative)",
                        "type": "string"
                    },
                    "ignore": {
                        "description": "List of glob patterns to ignore",
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "respect_git_ignore": {
                        "description": "Optional: Whether to respect .gitignore patterns when listing files. Only available in git repositories. Defaults to true.",
                        "type": "boolean"
                    }
                },
                "required": ["path"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: ListDirectoryParams = serde_json::from_value(params.clone())?;
        
        let path = Path::new(&parsed.path);
        if !path.is_absolute() {
            return Err(anyhow!("Path must be absolute, but was relative: {}", parsed.path));
        }
        
        if !path.exists() {
            return Err(anyhow!("Directory does not exist: {}", parsed.path));
        }
        
        if !path.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", parsed.path));
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<ListDirectoryParams>(params.clone()) {
            format!("List contents of directory: {}", parsed.path)
        } else {
            "List directory contents".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<ListDirectoryParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.path,
                line: None,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: ListDirectoryParams = serde_json::from_value(params.clone())?;
        let dir_path = Path::new(&parsed.path);
        
        // Compile ignore patterns
        let mut ignore_patterns = Vec::new();
        if let Some(ignore) = &parsed.ignore {
            for pattern_str in ignore {
                match Pattern::new(pattern_str) {
                    Ok(pattern) => ignore_patterns.push(pattern),
                    Err(e) => return Err(anyhow!("Invalid glob pattern '{}': {}", pattern_str, e)),
                }
            }
        }
        
        // Add gitignore patterns if requested
        if parsed.respect_git_ignore.unwrap_or(true) {
            ignore_patterns.extend(self.load_gitignore_patterns(dir_path));
        }
        
        // Read directory contents
        let entries = fs::read_dir(dir_path)?;
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // Skip if matches ignore patterns
            if self.should_ignore(&file_name, &ignore_patterns) {
                continue;
            }
            
            if entry.file_type()?.is_dir() {
                directories.push(file_name);
            } else {
                files.push(file_name);
            }
        }
        
        // Sort alphabetically
        files.sort();
        directories.sort();
        
        let total_count = files.len() + directories.len();
        let summary = format!("Listed {} items in directory", total_count);
        
        let mut display_lines = Vec::new();
        display_lines.push(format!("Directory: {}", parsed.path));
        display_lines.push(format!("Total items: {}", total_count));
        display_lines.push("".to_string());
        
        if !directories.is_empty() {
            display_lines.push("Directories:".to_string());
            for dir in &directories {
                display_lines.push(format!("  📁 {}/", dir));
            }
            display_lines.push("".to_string());
        }
        
        if !files.is_empty() {
            display_lines.push("Files:".to_string());
            for file in &files {
                display_lines.push(format!("  📄 {}", file));
            }
        }
        
        let llm_content = serde_json::json!({
            "directory": parsed.path,
            "total_items": total_count,
            "directories": directories,
            "files": files
        });
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}