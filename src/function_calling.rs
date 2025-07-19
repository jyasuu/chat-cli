use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;
use crate::mcp_client::McpClientManager;
use crate::tools::ToolRegistry;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct FunctionExecutor {
    mcp_manager: Option<McpClientManager>,
    tool_registry: ToolRegistry,
}

impl FunctionExecutor {
    pub fn new() -> Self {
        Self {
            mcp_manager: None,
            tool_registry: ToolRegistry::new(),
        }
    }

    pub fn with_mcp_manager(mut self, mcp_manager: McpClientManager) -> Self {
        self.mcp_manager = Some(mcp_manager);
        self
    }

    pub fn get_available_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = vec![
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
        ];

        // Add built-in tools from registry
        tools.extend(self.tool_registry.get_available_tools());

        // Add MCP tools if available
        if let Some(mcp_manager) = &self.mcp_manager {
            tools.extend(mcp_manager.get_available_tools());
        }

        tools
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
            _ => {
                // Check if it's a built-in tool
                if self.tool_registry.get_tool(&function_call.name).is_some() {
                    return self.tool_registry.execute_tool(function_call).await;
                }
                
                // Check if it's an MCP tool
                if let Some(mcp_manager) = &self.mcp_manager {
                    if mcp_manager.has_tool(&function_call.name) {
                        return mcp_manager.execute_tool(function_call).await;
                    }
                }
                Err(anyhow!("Unknown function: {}", function_call.name))
            }
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