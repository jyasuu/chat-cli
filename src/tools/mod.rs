use anyhow::Result;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

pub mod list_directory;
pub mod read_file;
pub mod search_file_content;
pub mod glob;
pub mod replace;
pub mod write_file;
pub mod web_fetch;
pub mod read_many_files;
pub mod run_shell_command;
pub mod save_memory;
pub mod google_web_search;

use crate::function_calling::{FunctionCall, FunctionResponse, ToolDefinition};

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub summary: Option<String>,
    pub llm_content: serde_json::Value,
    pub return_display: String,
}

#[derive(Debug, Clone)]
pub struct ToolLocation {
    pub path: String,
    pub line: Option<usize>,
}

#[async_trait]
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn get_tool_definition(&self) -> ToolDefinition;
    fn validate_params(&self, params: &serde_json::Value) -> Result<()>;
    fn get_description(&self, params: &serde_json::Value) -> String;
    fn tool_locations(&self, params: &serde_json::Value) -> Vec<ToolLocation>;
    async fn execute(&self, params: &serde_json::Value) -> Result<ToolResult>;
}

pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: std::collections::HashMap::new(),
        };
        
        // Register all built-in tools
        registry.register_tool(Box::new(list_directory::ListDirectoryTool::new()));
        registry.register_tool(Box::new(read_file::ReadFileTool::new()));
        registry.register_tool(Box::new(search_file_content::SearchFileContentTool::new()));
        registry.register_tool(Box::new(glob::GlobTool::new()));
        registry.register_tool(Box::new(replace::ReplaceTool::new()));
        registry.register_tool(Box::new(write_file::WriteFileTool::new()));
        registry.register_tool(Box::new(web_fetch::WebFetchTool::new()));
        registry.register_tool(Box::new(read_many_files::ReadManyFilesTool::new()));
        registry.register_tool(Box::new(run_shell_command::RunShellCommandTool::new()));
        registry.register_tool(Box::new(save_memory::SaveMemoryTool::new()));
        registry.register_tool(Box::new(google_web_search::GoogleWebSearchTool::new()));
        
        registry
    }
    
    pub fn register_tool(&mut self, tool: Box<dyn Tool + Send + Sync>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub fn get_available_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.get_tool_definition()).collect()
    }
    
    pub fn get_tool(&self, name: &str) -> Option<&Box<dyn Tool + Send + Sync>> {
        self.tools.get(name)
    }
    
    pub async fn execute_tool(&self, function_call: &FunctionCall) -> Result<FunctionResponse> {
        let tool = self.get_tool(&function_call.name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", function_call.name))?;
        
        tool.validate_params(&function_call.args)?;
        let result = tool.execute(&function_call.args).await?;
        
        let function_id = format!("{}-{}", function_call.name, chrono::Utc::now().timestamp_millis());
        
        Ok(FunctionResponse {
            id: function_id,
            name: function_call.name.clone(),
            response: serde_json::to_value(result)?,
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}