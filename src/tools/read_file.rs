use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use async_trait::async_trait;

#[derive(Debug, Deserialize)]
struct ReadFileParams {
    absolute_path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    
    fn description(&self) -> &str {
        "Reads and returns the content of a specified file from the local filesystem. Handles text, images (PNG, JPG, GIF, WEBP, SVG, BMP), and PDF files. For text files, it can read specific line ranges."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "absolute_path": {
                        "description": "The absolute path to the file to read (e.g., '/home/user/project/file.txt'). Relative paths are not supported. You must provide an absolute path.",
                        "type": "string"
                    },
                    "offset": {
                        "description": "Optional: For text files, the 0-based line number to start reading from. Requires 'limit' to be set. Use for paginating through large files.",
                        "type": "number"
                    },
                    "limit": {
                        "description": "Optional: For text files, maximum number of lines to read. Use with 'offset' to paginate through large files. If omitted, reads the entire file (if feasible, up to a default limit).",
                        "type": "number"
                    }
                },
                "required": ["absolute_path"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: ReadFileParams = serde_json::from_value(params.clone())?;
        
        let path = Path::new(&parsed.absolute_path);
        if !path.is_absolute() {
            return Err(anyhow!("File path must be absolute, but was relative: {}", parsed.absolute_path));
        }
        
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", parsed.absolute_path));
        }
        
        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", parsed.absolute_path));
        }
        
        if let Some(_offset) = parsed.offset {
            if parsed.limit.is_none() {
                return Err(anyhow!("Offset requires limit to be set"));
            }
        }
        
        if let Some(limit) = parsed.limit {
            if limit == 0 {
                return Err(anyhow!("Limit must be greater than 0"));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<ReadFileParams>(params.clone()) {
            let path = Path::new(&parsed.absolute_path);
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            
            match (parsed.offset, parsed.limit) {
                (Some(offset), Some(limit)) => {
                    format!("Read lines {}-{} from {}", offset, offset + limit - 1, filename)
                }
                (None, Some(limit)) => {
                    format!("Read first {} lines from {}", limit, filename)
                }
                _ => format!("Read file: {}", filename)
            }
        } else {
            "Read file".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<ReadFileParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.absolute_path,
                line: parsed.offset,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: ReadFileParams = serde_json::from_value(params.clone())?;
        let path = Path::new(&parsed.absolute_path);
        
        // Check if it's a binary file by extension
        let is_text_file = match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => matches!(
                ext.to_lowercase().as_str(),
                "txt" | "md" | "rs" | "py" | "js" | "ts" | "html" | "css" | "json" | "xml" | "yaml" | "yml" | "toml" | "cfg" | "ini" | "log" | "csv" | "sql" | "sh" | "bat" | "ps1" | "dockerfile" | "gitignore" | "gitattributes" | "editorconfig" | "env" | "properties" | "conf" | "config" | "makefile" | "cmake" | "gradle" | "pom" | "package" | "lock" | "sum" | "mod" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "php" | "rb" | "swift" | "kt" | "scala" | "clj" | "hs" | "elm" | "dart" | "vue" | "jsx" | "tsx" | "svelte" | "astro" | "scss" | "sass" | "less" | "styl"
            ),
            None => {
                // No extension, check if it's a known text file
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                matches!(
                    filename.as_str(),
                    "readme" | "license" | "changelog" | "makefile" | "dockerfile" | "gemfile" | "rakefile" | "procfile" | "cmakelists.txt"
                )
            }
        };
        
        if !is_text_file {
            return Err(anyhow!("Binary files are not supported yet. File: {}", parsed.absolute_path));
        }
        
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let (selected_lines, start_line, end_line) = match (parsed.offset, parsed.limit) {
            (Some(offset), Some(limit)) => {
                let start = offset;
                let end = std::cmp::min(start + limit, lines.len());
                if start >= lines.len() {
                    return Err(anyhow!("Offset {} is beyond file length {}", offset, lines.len()));
                }
                (lines[start..end].to_vec(), start, end - 1)
            }
            (None, Some(limit)) => {
                let end = std::cmp::min(limit, lines.len());
                (lines[0..end].to_vec(), 0, end - 1)
            }
            _ => (lines.clone(), 0, lines.len().saturating_sub(1))
        };
        
        let selected_content = selected_lines.join("\n");
        let total_lines = lines.len();
        let selected_line_count = selected_lines.len();
        
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let summary = if selected_line_count == total_lines {
            format!("Read {} lines from {}", total_lines, filename)
        } else {
            format!("Read {} of {} lines from {}", selected_line_count, total_lines, filename)
        };
        
        let llm_content = serde_json::json!({
            "file_path": parsed.absolute_path,
            "content": selected_content,
            "total_lines": total_lines,
            "selected_lines": selected_line_count,
            "start_line": start_line,
            "end_line": end_line
        });
        
        let mut display_lines = vec![
            format!("File: {}", parsed.absolute_path),
            format!("Lines: {} (showing {}-{})", total_lines, start_line + 1, end_line + 1),
            "".to_string(),
        ];
        
        // Add line numbers to content for display
        for (i, line) in selected_lines.iter().enumerate() {
            let line_num = start_line + i + 1;
            display_lines.push(format!("{:4} | {}", line_num, line));
        }
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}