use anyhow::{anyhow, Result};
use rmcp::{
    RoleClient, ServiceExt, service::RunningService,
    model::{CallToolRequestParam, Tool as McpTool},
    transport::ConfigureCommandExt,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Stdio};
use tokio::process::Command;

use crate::function_calling::{ToolDefinition, FunctionCall, FunctionResponse};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    pub inputs: Option<Vec<McpInput>>,
    pub servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpInput {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub password: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

pub struct McpClientManager {
    clients: HashMap<String, RunningService<RoleClient, ()>>,
    tools: HashMap<String, (String, McpTool)>, // tool_name -> (server_name, tool)
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    pub async fn load_from_config(&mut self, config: McpConfig) -> Result<()> {
        // Process inputs (for now, we'll use environment variables)
        let mut resolved_env = HashMap::new();
        if let Some(inputs) = &config.inputs {
            for input in inputs {
                if let Ok(value) = std::env::var(&input.id.to_uppercase()) {
                    resolved_env.insert(input.id.clone(), value);
                } else {
                    println!("Warning: Environment variable {} not found for input {}", 
                            input.id.to_uppercase(), input.id);
                }
            }
        }

        // Start MCP servers
        for (server_name, server_config) in config.servers {
            println!("Starting MCP server: {}", server_name);
            
            // Resolve environment variables in server config
            let mut env_vars = server_config.env.clone();
            for (_key, value) in &mut env_vars {
                if value.starts_with("${input:") && value.ends_with("}") {
                    let input_id = &value[8..value.len()-1];
                    if let Some(resolved_value) = resolved_env.get(input_id) {
                        *value = resolved_value.clone();
                    }
                }
            }

            // Create and start the MCP client
            let transport = rmcp::transport::child_process::TokioChildProcess::new(
                Command::new(&server_config.command).configure(|cmd| {
                    cmd.args(&server_config.args)
                        .envs(&env_vars)
                        .stderr(Stdio::inherit());
                }),
            )?;

            let client = ().serve(transport).await?;
            
            // Get available tools from this server
            match client.list_all_tools().await {
                Ok(tools) => {
                    println!("Found {} tools from server {}", tools.len(), server_name);
                    for tool in tools {
                        let tool_name = tool.name.to_string();
                        self.tools.insert(tool_name.clone(), (server_name.clone(), tool));
                        println!("  - {}", tool_name);
                    }
                }
                Err(e) => {
                    println!("Warning: Failed to list tools from server {}: {}", server_name, e);
                }
            }

            self.clients.insert(server_name, client);
        }

        Ok(())
    }

    pub fn get_available_tools(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|(tool_name, (_, mcp_tool))| {
            ToolDefinition {
                name: tool_name.clone(),
                description: mcp_tool.description.as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| format!("MCP tool: {}", tool_name)),
                parameters: serde_json::to_value(&mcp_tool.input_schema)
                    .unwrap_or_else(|_| serde_json::json!({})),
            }
        }).collect()
    }

    pub async fn execute_tool(&self, function_call: &FunctionCall) -> Result<FunctionResponse> {
        let function_id = format!("{}-{}", function_call.name, chrono::Utc::now().timestamp_millis());
        
        if let Some((server_name, _)) = self.tools.get(&function_call.name) {
            if let Some(client) = self.clients.get(server_name) {
                println!("🔧 Executing MCP tool: {} on server: {}", function_call.name, server_name);
                
                // Convert args to the format expected by MCP
                let arguments = match &function_call.args {
                    serde_json::Value::Object(map) => Some(map.clone()),
                    _ => None,
                };

                let result = client.call_tool(CallToolRequestParam {
                    name: function_call.name.clone().into(),
                    arguments,
                }).await?;

                // Convert MCP result to our function response format
                let response_data = if result.is_error.unwrap_or(false) {
                    serde_json::json!({
                        "success": false,
                        "error": "Tool execution failed",
                        "content": result.content
                    })
                } else {
                    serde_json::json!({
                        "success": true,
                        "content": result.content
                    })
                };

                return Ok(FunctionResponse {
                    id: function_id,
                    name: function_call.name.clone(),
                    response: response_data,
                });
            }
        }

        Err(anyhow!("MCP tool not found: {}", function_call.name))
    }

    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.tools.contains_key(tool_name)
    }
}