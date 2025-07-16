use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub id: String,
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct FunctionExecutor;

impl FunctionExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "shell_command".to_string(),
                description: "Execute a shell command in the current working directory".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }),
            }
        ]
    }

    pub async fn execute_function(&self, function_call: &FunctionCall) -> Result<FunctionResponse> {
        let function_id = format!("{}-{}", function_call.name, chrono::Utc::now().timestamp_millis());
        
        match function_call.name.as_str() {
            "shell_command" => {
                let command = function_call.args.get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' parameter"))?;
                
                self.execute_shell_command(command, &function_id).await
            }
            _ => Err(anyhow!("Unknown function: {}", function_call.name))
        }
    }

    async fn execute_shell_command(&self, command: &str, function_id: &str) -> Result<FunctionResponse> {
        println!("🔧 Executing shell command: {}", command);
        
        // Use shell to execute the command properly
        let output = if cfg!(target_os = "windows") {
            AsyncCommand::new("cmd")
                .args(["/C", command])
                .output()
                .await?
        } else {
            AsyncCommand::new("sh")
                .args(["-c", command])
                .output()
                .await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let success = output.status.success();

        let response_data = serde_json::json!({
            "success": success,
            "exit_code": output.status.code(),
            "stdout": stdout.to_string(),
            "stderr": stderr.to_string(),
            "output": if success {
                format!("Command executed successfully.\nOutput:\n{}", stdout)
            } else {
                format!("Command failed with exit code {:?}.\nStdout:\n{}\nStderr:\n{}", 
                    output.status.code(), stdout, stderr)
            }
        });

        Ok(FunctionResponse {
            id: function_id.to_string(),
            name: "shell_command".to_string(),
            response: response_data,
        })
    }
}

pub fn parse_function_call_from_text(text: &str) -> Option<FunctionCall> {
    // Look for function call patterns in the text
    // This is a simple implementation - in practice, you might want more sophisticated parsing
    if let Ok(parsed) = serde_json::from_str::<FunctionCall>(text) {
        Some(parsed)
    } else {
        None
    }
}