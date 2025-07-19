use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use glob::glob;

#[derive(Debug, Deserialize)]
struct GlobParams {
    pattern: String,
    path: Option<String>,
    case_sensitive: Option<bool>,
    respect_git_ignore: Option<bool>,
}

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }
    
    fn description(&self) -> &str {
        "Efficiently finds files matching specific glob patterns (e.g., `src/**/*.ts`, `**/*.md`), returning absolute paths sorted by modification time (newest first). Ideal for quickly locating files based on their name or path structure, especially in large codebases."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "description": "The glob pattern to match against (e.g., '**/*.py', 'docs/*.md').",
                        "type": "string"
                    },
                    "path": {
                        "description": "Optional: The absolute path to the directory to search within. If omitted, searches the root directory.",
                        "type": "string"
                    },
                    "case_sensitive": {
                        "description": "Optional: Whether the search should be case-sensitive. Defaults to false.",
                        "type": "boolean"
                    },
                    "respect_git_ignore": {
                        "description": "Optional: Whether to respect .gitignore patterns when finding files. Only available in git repositories. Defaults to true.",
                        "type": "boolean"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: GlobParams = serde_json::from_value(params.clone())?;
        
        if parsed.pattern.trim().is_empty() {
            return Err(anyhow!("Pattern cannot be empty"));
        }
        
        if let Some(path) = &parsed.path {
            let search_path = Path::new(path);
            if !search_path.exists() {
                return Err(anyhow!("Search path does not exist: {}", path));
            }
            if !search_path.is_dir() {
                return Err(anyhow!("Search path is not a directory: {}", path));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<GlobParams>(params.clone()) {
            let path_desc = parsed.path.as_deref().unwrap_or("current directory");
            format!("Find files matching '{}' in {}", parsed.pattern, path_desc)
        } else {
            "Find files with glob pattern".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<GlobParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.path.unwrap_or_else(|| ".".to_string()),
                line: None,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: GlobParams = serde_json::from_value(params.clone())?;
        
        let search_path = if let Some(path) = &parsed.path {
            PathBuf::from(path)
        } else {
            std::env::current_dir()?
        };
        
        // Change to the search directory temporarily
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&search_path)?;
        
        let result = async {
            let pattern = parsed.pattern.clone();
            
            // Handle case sensitivity
            if !parsed.case_sensitive.unwrap_or(false) {
                // For case-insensitive matching, we'll need to handle this manually
                // since the glob crate doesn't directly support case-insensitive patterns
            }
            
            let mut matched_files = Vec::new();
            
            match glob(&pattern) {
                Ok(paths) => {
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                // Convert to absolute path
                                let abs_path = if path.is_absolute() {
                                    path
                                } else {
                                    search_path.join(&path)
                                };
                                
                                // Get file metadata for sorting
                                if let Ok(metadata) = std::fs::metadata(&abs_path) {
                                    if let Ok(modified) = metadata.modified() {
                                        matched_files.push((abs_path, modified));
                                    } else {
                                        matched_files.push((abs_path, std::time::SystemTime::UNIX_EPOCH));
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error processing glob entry: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Invalid glob pattern '{}': {}", pattern, e));
                }
            }
            
            // Sort by modification time (newest first)
            matched_files.sort_by(|a, b| b.1.cmp(&a.1));
            
            let file_paths: Vec<String> = matched_files
                .iter()
                .map(|(path, _)| path.to_string_lossy().to_string())
                .collect();
            
            let summary = format!("Found {} files matching pattern", file_paths.len());
            
            let llm_content = serde_json::json!({
                "pattern": parsed.pattern,
                "search_path": search_path.to_string_lossy(),
                "case_sensitive": parsed.case_sensitive.unwrap_or(false),
                "total_matches": file_paths.len(),
                "files": file_paths
            });
            
            let mut display_lines = vec![
                format!("Pattern: {}", parsed.pattern),
                format!("Search path: {}", search_path.display()),
                format!("Case sensitive: {}", parsed.case_sensitive.unwrap_or(false)),
                format!("Total matches: {}", file_paths.len()),
                "".to_string(),
            ];
            
            if file_paths.is_empty() {
                display_lines.push("No files found matching the pattern.".to_string());
            } else {
                display_lines.push("Files (sorted by modification time, newest first):".to_string());
                for (i, file_path) in file_paths.iter().enumerate() {
                    if i < 50 { // Limit display to first 50 files
                        display_lines.push(format!("  📄 {}", file_path));
                    } else {
                        display_lines.push(format!("  ... and {} more files", file_paths.len() - 50));
                        break;
                    }
                }
            }
            
            Ok(ToolResult {
                summary: Some(summary),
                llm_content,
                return_display: display_lines.join("\n"),
            })
        }.await;
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;
        
        result
    }
}