use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;

#[derive(Debug, Deserialize)]
struct SearchFileContentParams {
    pattern: String,
    path: Option<String>,
    include: Option<String>,
}

pub struct SearchFileContentTool;

impl SearchFileContentTool {
    pub fn new() -> Self {
        Self
    }
    
    fn is_text_file(path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => matches!(
                ext.to_lowercase().as_str(),
                "txt" | "md" | "rs" | "py" | "js" | "ts" | "html" | "css" | "json" | "xml" | "yaml" | "yml" | "toml" | "cfg" | "ini" | "log" | "csv" | "sql" | "sh" | "bat" | "ps1" | "dockerfile" | "gitignore" | "gitattributes" | "editorconfig" | "env" | "properties" | "conf" | "config" | "makefile" | "cmake" | "gradle" | "pom" | "package" | "lock" | "sum" | "mod" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "php" | "rb" | "swift" | "kt" | "scala" | "clj" | "hs" | "elm" | "dart" | "vue" | "jsx" | "tsx" | "svelte" | "astro" | "scss" | "sass" | "less" | "styl"
            ),
            None => {
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                matches!(
                    filename.as_str(),
                    "readme" | "license" | "changelog" | "makefile" | "dockerfile" | "gemfile" | "rakefile" | "procfile" | "cmakelists.txt"
                )
            }
        }
    }
    
    fn collect_files(&self, search_path: &Path, include_pattern: Option<&str>) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        if search_path.is_file() {
            if Self::is_text_file(search_path) {
                files.push(search_path.to_path_buf());
            }
            return Ok(files);
        }
        
        let include_regex = if let Some(pattern) = include_pattern {
            Some(glob_to_regex(pattern)?)
        } else {
            None
        };
        
        fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>, include_regex: &Option<Regex>) -> Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.is_dir() {
                        // Skip hidden directories and common ignore patterns
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with('.') || matches!(name, "node_modules" | "target" | "build" | "dist" | "__pycache__") {
                                continue;
                            }
                        }
                        visit_dir(&path, files, include_regex)?;
                    } else if SearchFileContentTool::is_text_file(&path) {
                        if let Some(regex) = include_regex {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                if regex.is_match(filename) {
                                    files.push(path);
                                }
                            }
                        } else {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        }
        
        visit_dir(search_path, &mut files, &include_regex)?;
        Ok(files)
    }
}

fn glob_to_regex(pattern: &str) -> Result<Regex> {
    let mut regex_pattern = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    regex_pattern.push_str(".*");
                    i += 2;
                } else {
                    regex_pattern.push_str("[^/]*");
                    i += 1;
                }
            }
            '?' => {
                regex_pattern.push_str("[^/]");
                i += 1;
            }
            '[' => {
                regex_pattern.push('[');
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    regex_pattern.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    regex_pattern.push(']');
                    i += 1;
                }
            }
            c if "(){}+^$|\\".contains(c) => {
                regex_pattern.push('\\');
                regex_pattern.push(c);
                i += 1;
            }
            c => {
                regex_pattern.push(c);
                i += 1;
            }
        }
    }
    
    Regex::new(&regex_pattern).map_err(|e| anyhow!("Invalid regex pattern: {}", e))
}

#[async_trait]
impl Tool for SearchFileContentTool {
    fn name(&self) -> &str {
        "search_file_content"
    }
    
    fn description(&self) -> &str {
        "Searches for a regular expression pattern within the content of files in a specified directory (or current working directory). Can filter files by a glob pattern. Returns the lines containing matches, along with their file paths and line numbers."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "description": "The regular expression (regex) pattern to search for within file contents (e.g., 'function\\s+myFunction', 'import\\s+\\{.*\\}\\s+from\\s+.*').",
                        "type": "string"
                    },
                    "path": {
                        "description": "Optional: The absolute path to the directory to search within. If omitted, searches the current working directory.",
                        "type": "string"
                    },
                    "include": {
                        "description": "Optional: A glob pattern to filter which files are searched (e.g., '*.js', '*.{ts,tsx}', 'src/**'). If omitted, searches all files (respecting potential global ignores).",
                        "type": "string"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: SearchFileContentParams = serde_json::from_value(params.clone())?;
        
        // Validate regex pattern
        Regex::new(&parsed.pattern).map_err(|e| anyhow!("Invalid regex pattern '{}': {}", parsed.pattern, e))?;
        
        if let Some(path) = &parsed.path {
            let search_path = Path::new(path);
            if !search_path.exists() {
                return Err(anyhow!("Search path does not exist: {}", path));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<SearchFileContentParams>(params.clone()) {
            let path_desc = parsed.path.as_deref().unwrap_or("current directory");
            let include_desc = parsed.include.as_deref().unwrap_or("all files");
            format!("Search for '{}' in {} ({})", parsed.pattern, path_desc, include_desc)
        } else {
            "Search file content".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<SearchFileContentParams>(params.clone()) {
            vec![ToolLocation {
                path: parsed.path.unwrap_or_else(|| ".".to_string()),
                line: None,
            }]
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: SearchFileContentParams = serde_json::from_value(params.clone())?;
        
        let search_path = if let Some(path) = &parsed.path {
            Path::new(path)
        } else {
            Path::new(".")
        };
        
        let regex = Regex::new(&parsed.pattern)?;
        let files = self.collect_files(search_path, parsed.include.as_deref())?;
        
        let mut matches = Vec::new();
        let mut total_matches = 0;
        
        for file_path in files {
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let mut file_matches = Vec::new();
                    
                    for (line_num, line) in lines.iter().enumerate() {
                        if regex.is_match(line) {
                            file_matches.push(serde_json::json!({
                                "line_number": line_num + 1,
                                "content": line
                            }));
                            total_matches += 1;
                        }
                    }
                    
                    if !file_matches.is_empty() {
                        matches.push(serde_json::json!({
                            "file_path": file_path.to_string_lossy(),
                            "matches": file_matches
                        }));
                    }
                }
                Err(_) => {
                    // Skip files that can't be read as text
                    continue;
                }
            }
        }
        
        let summary = format!("Found {} matches in {} files", total_matches, matches.len());
        
        let llm_content = serde_json::json!({
            "pattern": parsed.pattern,
            "search_path": search_path.to_string_lossy(),
            "include_pattern": parsed.include,
            "total_matches": total_matches,
            "files_with_matches": matches.len(),
            "matches": matches
        });
        
        let mut display_lines = vec![
            format!("Search pattern: {}", parsed.pattern),
            format!("Search path: {}", search_path.display()),
            if let Some(include) = &parsed.include {
                format!("Include pattern: {}", include)
            } else {
                "Include pattern: all files".to_string()
            },
            format!("Total matches: {} in {} files", total_matches, matches.len()),
            "".to_string(),
        ];
        
        for match_info in &matches {
            if let (Some(file_path), Some(file_matches)) = (
                match_info.get("file_path").and_then(|v| v.as_str()),
                match_info.get("matches").and_then(|v| v.as_array())
            ) {
                display_lines.push(format!("📄 {}", file_path));
                for file_match in file_matches {
                    if let (Some(line_num), Some(content)) = (
                        file_match.get("line_number").and_then(|v| v.as_u64()),
                        file_match.get("content").and_then(|v| v.as_str())
                    ) {
                        display_lines.push(format!("  {:4}: {}", line_num, content));
                    }
                }
                display_lines.push("".to_string());
            }
        }
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}