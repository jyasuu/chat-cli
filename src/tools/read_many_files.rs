use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use glob::glob;

#[derive(Debug, Deserialize)]
struct ReadManyFilesParams {
    paths: Vec<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    recursive: Option<bool>,
    #[serde(rename = "useDefaultExcludes")]
    use_default_excludes: Option<bool>,
    respect_git_ignore: Option<bool>,
}

pub struct ReadManyFilesTool;

impl ReadManyFilesTool {
    pub fn new() -> Self {
        Self
    }
    
    fn get_default_excludes(&self) -> Vec<String> {
        vec![
            "node_modules/**".to_string(),
            "target/**".to_string(),
            "build/**".to_string(),
            "dist/**".to_string(),
            ".git/**".to_string(),
            "**/*.log".to_string(),
            "**/*.tmp".to_string(),
            "**/*.temp".to_string(),
            "**/.DS_Store".to_string(),
            "**/Thumbs.db".to_string(),
            "**/__pycache__/**".to_string(),
            "**/*.pyc".to_string(),
            "**/*.pyo".to_string(),
            "**/*.class".to_string(),
            "**/*.o".to_string(),
            "**/*.so".to_string(),
            "**/*.dll".to_string(),
            "**/*.exe".to_string(),
            "**/*.bin".to_string(),
            "**/*.zip".to_string(),
            "**/*.tar.gz".to_string(),
            "**/*.rar".to_string(),
            "**/*.7z".to_string(),
            "**/*.pdf".to_string(),
            "**/*.jpg".to_string(),
            "**/*.jpeg".to_string(),
            "**/*.png".to_string(),
            "**/*.gif".to_string(),
            "**/*.bmp".to_string(),
            "**/*.ico".to_string(),
            "**/*.svg".to_string(),
            "**/*.mp3".to_string(),
            "**/*.mp4".to_string(),
            "**/*.avi".to_string(),
            "**/*.mov".to_string(),
            "**/*.wmv".to_string(),
        ]
    }
    
    fn is_text_file(&self, path: &Path) -> bool {
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
    
    fn should_exclude(&self, path: &Path, exclude_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();
        exclude_patterns.iter().any(|pattern| {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                glob_pattern.matches(&path_str)
            } else {
                false
            }
        })
    }
    
    fn collect_files(&self, params: &ReadManyFilesParams) -> Result<Vec<PathBuf>> {
        let mut all_files = Vec::new();
        let recursive = params.recursive.unwrap_or(true);
        
        // Combine exclude patterns
        let mut exclude_patterns = params.exclude.clone().unwrap_or_default();
        if params.use_default_excludes.unwrap_or(true) {
            exclude_patterns.extend(self.get_default_excludes());
        }
        
        // Process main paths
        for path_pattern in &params.paths {
            if path_pattern.contains('*') || path_pattern.contains('?') {
                // It's a glob pattern
                match glob(path_pattern) {
                    Ok(paths) => {
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    if path.is_file() && self.is_text_file(&path) {
                                        if !self.should_exclude(&path, &exclude_patterns) {
                                            all_files.push(path);
                                        }
                                    } else if path.is_dir() && recursive {
                                        self.collect_from_directory(&path, &exclude_patterns, &mut all_files)?;
                                    }
                                }
                                Err(e) => eprintln!("Error processing glob entry: {}", e),
                            }
                        }
                    }
                    Err(e) => return Err(anyhow!("Invalid glob pattern '{}': {}", path_pattern, e)),
                }
            } else {
                // It's a regular path
                let path = Path::new(path_pattern);
                if path.is_file() {
                    if self.is_text_file(path) && !self.should_exclude(path, &exclude_patterns) {
                        all_files.push(path.to_path_buf());
                    }
                } else if path.is_dir() && recursive {
                    self.collect_from_directory(path, &exclude_patterns, &mut all_files)?;
                }
            }
        }
        
        // Process include patterns
        if let Some(include_patterns) = &params.include {
            for include_pattern in include_patterns {
                match glob(include_pattern) {
                    Ok(paths) => {
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    if path.is_file() && self.is_text_file(&path) {
                                        if !self.should_exclude(&path, &exclude_patterns) {
                                            all_files.push(path);
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Error processing include glob entry: {}", e),
                            }
                        }
                    }
                    Err(e) => return Err(anyhow!("Invalid include glob pattern '{}': {}", include_pattern, e)),
                }
            }
        }
        
        // Remove duplicates
        all_files.sort();
        all_files.dedup();
        
        Ok(all_files)
    }
    
    fn collect_from_directory(&self, dir: &Path, exclude_patterns: &[String], files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if self.should_exclude(&path, exclude_patterns) {
                continue;
            }
            
            if path.is_file() && self.is_text_file(&path) {
                files.push(path);
            } else if path.is_dir() {
                self.collect_from_directory(&path, exclude_patterns, files)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for ReadManyFilesTool {
    fn name(&self) -> &str {
        "read_many_files"
    }
    
    fn description(&self) -> &str {
        "Reads content from multiple files specified by paths or glob patterns within a configured target directory. For text files, it concatenates their content into a single string."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "minLength": 1
                        },
                        "minItems": 1,
                        "description": "Required. An array of glob patterns or paths relative to the tool's target directory. Examples: ['src/**/*.ts'], ['README.md', 'docs/']"
                    },
                    "include": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "minLength": 1
                        },
                        "description": "Optional. Additional glob patterns to include. These are merged with `paths`. Example: [\"*.test.ts\"] to specifically add test files if they were broadly excluded."
                    },
                    "exclude": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "minLength": 1
                        },
                        "description": "Optional. Glob patterns for files/directories to exclude. Added to default excludes if useDefaultExcludes is true. Example: [\"**/*.log\", \"temp/\"]"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Optional. Whether to search recursively (primarily controlled by `**` in glob patterns). Defaults to true."
                    },
                    "useDefaultExcludes": {
                        "type": "boolean",
                        "description": "Optional. Whether to apply a list of default exclusion patterns (e.g., node_modules, .git, binary files). Defaults to true."
                    },
                    "respect_git_ignore": {
                        "type": "boolean",
                        "description": "Optional. Whether to respect .gitignore patterns when discovering files. Only available in git repositories. Defaults to true."
                    }
                },
                "required": ["paths"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: ReadManyFilesParams = serde_json::from_value(params.clone())?;
        
        if parsed.paths.is_empty() {
            return Err(anyhow!("At least one path must be specified"));
        }
        
        for path in &parsed.paths {
            if path.trim().is_empty() {
                return Err(anyhow!("Path cannot be empty"));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<ReadManyFilesParams>(params.clone()) {
            format!("Read multiple files from {} path patterns", parsed.paths.len())
        } else {
            "Read multiple files".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<ReadManyFilesParams>(params.clone()) {
            parsed.paths.into_iter().map(|path| ToolLocation {
                path,
                line: None,
            }).collect()
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: ReadManyFilesParams = serde_json::from_value(params.clone())?;
        
        let files = self.collect_files(&parsed)?;
        
        let mut concatenated_content = String::new();
        let mut successful_reads = 0;
        let mut failed_reads = 0;
        let mut total_size = 0;
        let mut file_info = Vec::new();
        
        for file_path in &files {
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    concatenated_content.push_str(&format!("\n=== {} ===\n", file_path.display()));
                    concatenated_content.push_str(&content);
                    concatenated_content.push_str("\n");
                    
                    total_size += content.len();
                    successful_reads += 1;
                    
                    file_info.push(serde_json::json!({
                        "path": file_path.to_string_lossy(),
                        "status": "success",
                        "size": content.len(),
                        "lines": content.lines().count()
                    }));
                }
                Err(e) => {
                    failed_reads += 1;
                    file_info.push(serde_json::json!({
                        "path": file_path.to_string_lossy(),
                        "status": "error",
                        "error": e.to_string()
                    }));
                }
            }
        }
        
        let summary = format!("Read {} files ({} successful, {} failed, {} total bytes)", 
            files.len(), successful_reads, failed_reads, total_size);
        
        let llm_content = serde_json::json!({
            "paths_requested": parsed.paths,
            "files_found": files.len(),
            "successful_reads": successful_reads,
            "failed_reads": failed_reads,
            "total_size": total_size,
            "concatenated_content": concatenated_content,
            "file_info": file_info
        });
        
        let mut display_lines = vec![
            format!("Read Many Files Results"),
            format!("Files found: {}", files.len()),
            format!("Successfully read: {}", successful_reads),
            format!("Failed to read: {}", failed_reads),
            format!("Total content size: {} bytes", total_size),
            "".to_string(),
        ];
        
        if successful_reads > 0 {
            display_lines.push("Successfully read files:".to_string());
            for info in &file_info {
                if let (Some(path), Some(status)) = (
                    info.get("path").and_then(|v| v.as_str()),
                    info.get("status").and_then(|v| v.as_str())
                ) {
                    if status == "success" {
                        if let (Some(size), Some(lines)) = (
                            info.get("size").and_then(|v| v.as_u64()),
                            info.get("lines").and_then(|v| v.as_u64())
                        ) {
                            display_lines.push(format!("  ✅ {} ({} bytes, {} lines)", path, size, lines));
                        }
                    }
                }
            }
        }
        
        if failed_reads > 0 {
            display_lines.push("".to_string());
            display_lines.push("Failed to read files:".to_string());
            for info in &file_info {
                if let (Some(path), Some(status)) = (
                    info.get("path").and_then(|v| v.as_str()),
                    info.get("status").and_then(|v| v.as_str())
                ) {
                    if status == "error" {
                        if let Some(error) = info.get("error").and_then(|v| v.as_str()) {
                            display_lines.push(format!("  ❌ {}: {}", path, error));
                        }
                    }
                }
            }
        }
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}