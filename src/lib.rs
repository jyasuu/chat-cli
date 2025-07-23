pub mod chat_client;
pub mod function_calling;
pub mod gemini;
pub mod openai;
pub mod mock_llm;
pub mod mcp_client;
pub mod tools;
pub mod ui;

// Re-export commonly used types
pub use chat_client::{ChatClient, AnyChatClient};
pub use function_calling::{FunctionCall, FunctionResponse, FunctionExecutor};
pub use mock_llm::MockLLMClient;
pub use mcp_client::{McpClientManager, McpConfig};
pub use tools::{ToolRegistry, Tool, ToolResult};