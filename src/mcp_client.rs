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
use crate::input_manager::{InputManager, McpInput};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    pub inputs: Option<Vec<McpInput>>,
    pub servers: HashMap<String, McpServerConfig>,
}

// McpInput is now defined in input_manager.rs

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
    input_manager: InputManager,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            tools: HashMap::new(),
            input_manager: InputManager::new(),
        }
    }

    pub async fn load_from_config(&mut self, config: McpConfig) -> Result<()> {
        // Register input definitions
        if let Some(inputs) = config.inputs {
            self.input_manager.register_inputs(inputs);
        }

        // Start MCP servers
        for (server_name, server_config) in config.servers {
            // Check if this server needs any inputs that aren't resolved yet
            let missing_inputs = self.input_manager.check_dependencies(&server_config.env);
            
            if !missing_inputs.is_empty() {
                println!("Server '{}' requires the following inputs:", server_name);
                for input_id in &missing_inputs {
                    println!("  - {}", input_id);
                }
                
                // Prompt for missing inputs
                for input_id in missing_inputs {
                    if let Err(e) = self.input_manager.get_input_value(&input_id) {
                        println!("Error getting input '{}': {}", input_id, e);
                        continue;
                    }
                }
            }
            
            println!("Starting MCP server: {}", server_name);
            
            // Resolve environment variables in server config
            let mut env_vars = server_config.env.clone();
            for (_key, value) in &mut env_vars {
                match self.input_manager.resolve_env_vars(value) {
                    Ok(resolved) => *value = resolved,
                    Err(e) => {
                        println!("Error resolving environment variable '{}': {}", value, e);
                        return Err(anyhow!("Failed to resolve required input for server '{}'", server_name));
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