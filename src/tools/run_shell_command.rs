use super::{Tool, ToolResult, ToolLocation};
use crate::function_calling::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct RunShellCommandParams {
    command: String,
    description: Option<String>,
    directory: Option<String>,
}

pub struct RunShellCommandTool;

impl RunShellCommandTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RunShellCommandTool {
    fn name(&self) -> &str {
        "run_shell_command"
    }
    
    fn description(&self) -> &str {
        "This tool executes a given shell command as `bash -c <command>`. Command can start background processes using `&`. Command is executed as a subprocess that leads its own process group."
    }
    
    fn get_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "description": "Exact bash command to execute as `bash -c <command>`",
                        "type": "string"
                    },
                    "description": {
                        "description": "Brief description of the command for the user. Be specific and concise. Ideally a single sentence. Can be up to 3 sentences for clarity. No line breaks.",
                        "type": "string"
                    },
                    "directory": {
                        "description": "(OPTIONAL) Directory to run the command in, if not the project root directory. Must be relative to the project root directory and must already exist.",
                        "type": "string"
                    }
                },
                "required": ["command"]
            }),
        }
    }
    
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let parsed: RunShellCommandParams = serde_json::from_value(params.clone())?;
        
        if parsed.command.trim().is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }
        
        if let Some(directory) = &parsed.directory {
            let dir_path = Path::new(directory);
            if dir_path.is_absolute() {
                return Err(anyhow!("Directory must be relative to project root, but was absolute: {}", directory));
            }
            
            if !dir_path.exists() {
                return Err(anyhow!("Directory does not exist: {}", directory));
            }
            
            if !dir_path.is_dir() {
                return Err(anyhow!("Path is not a directory: {}", directory));
            }
        }
        
        Ok(())
    }
    
    fn get_description(&self, params: &serde_json::Value) -> String {
        if let Ok(parsed) = serde_json::from_value::<RunShellCommandParams>(params.clone()) {
            if let Some(description) = parsed.description {
                description
            } else {
                format!("Execute: {}", parsed.command)
            }
        } else {
            "Execute shell command".to_string()
        }
    }
    
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation> {
        if let Ok(parsed) = serde_json::from_value::<RunShellCommandParams>(params.clone()) {
            if let Some(directory) = parsed.directory {
                vec![ToolLocation {
                    path: directory,
                    line: None,
                }]
            } else {
                vec![ToolLocation {
                    path: ".".to_string(),
                    line: None,
                }]
            }
        } else {
            Vec::new()
        }
    }
    
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult> {
        let parsed: RunShellCommandParams = serde_json::from_value(params.clone())?;
        
        let working_dir = parsed.directory.as_deref().unwrap_or(".");
        let current_dir = std::env::current_dir()?;
        let execution_dir = current_dir.join(working_dir);
        
        println!("🔧 Executing shell command: {}", parsed.command);
        if working_dir != "." {
            println!("📁 Working directory: {}", working_dir);
        }
        
        let mut command = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", &parsed.command]);
            cmd
        } else {
            let mut cmd = Command::new("bash");
            cmd.args(["-c", &parsed.command]);
            cmd
        };
        
        command
            .current_dir(&execution_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let start_time = std::time::Instant::now();
        let output = command.output().await?;
        let duration = start_time.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let success = output.status.success();
        let exit_code = output.status.code();
        
        let summary = if success {
            format!("Command executed successfully in {:.2}s", duration.as_secs_f64())
        } else {
            format!("Command failed with exit code {:?} in {:.2}s", exit_code, duration.as_secs_f64())
        };
        
        let llm_content = serde_json::json!({
            "command": parsed.command,
            "directory": working_dir,
            "success": success,
            "exit_code": exit_code,
            "stdout": stdout.to_string(),
            "stderr": stderr.to_string(),
            "duration_seconds": duration.as_secs_f64()
        });
        
        let mut display_lines = vec![
            format!("Command: {}", parsed.command),
            format!("Directory: {}", if working_dir == "." { "(root)" } else { working_dir }),
            format!("Exit code: {:?}", exit_code),
            format!("Duration: {:.2}s", duration.as_secs_f64()),
            "".to_string(),
        ];
        
        if !stdout.is_empty() {
            display_lines.push("Stdout:".to_string());
            display_lines.push(stdout.to_string());
            display_lines.push("".to_string());
        }
        
        if !stderr.is_empty() {
            display_lines.push("Stderr:".to_string());
            display_lines.push(stderr.to_string());
        }
        
        Ok(ToolResult {
            summary: Some(summary),
            llm_content,
            return_display: display_lines.join("\n"),
        })
    }
}