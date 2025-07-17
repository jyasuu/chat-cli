# Gemini Chat CLI

A command-line interface for chatting with Google's Gemini AI model and OpenAI, built in Rust with MCP (Model Context Protocol) integration.

## Features

- **Interactive Chat**: Real-time conversation with Gemini AI or OpenAI
- **Streaming Responses**: See responses as they're generated (default mode)
- **Non-streaming Mode**: Get complete responses at once
- **Built-in Commands**: Help, clear screen, quit, toggle streaming
- **Function Calling**: Execute shell commands and MCP tools
- **MCP Integration**: Connect to Model Context Protocol servers for extended functionality
- **Multi-Provider Support**: Works with OpenAI, Gemini, and custom endpoints
- **Error Handling**: Robust error handling for API issues
- **Environment Configuration**: Secure API key management

## Setup

1. **Clone and build the project:**
   ```bash
   cargo build
   ```

2. **Set up your API key:**
   ```bash
   cp .env.example .env
   # Edit .env and add your API key (Gemini or OpenAI)
   ```

3. **Get an API key:**
   - **Gemini**: Go to [Google AI Studio](https://aistudio.google.com/app/apikey)
   - **OpenAI**: Go to [OpenAI API Keys](https://platform.openai.com/api-keys)
   - Create a new API key and copy it to your `.env` file

4. **Optional - Configure MCP servers:**
   ```bash
   # Example: Connect to Git MCP server
   export MCP_SERVERS="git|stdio|uvx|mcp-server-git"
   
   # Multiple servers
   export MCP_SERVERS="git|stdio|uvx|mcp-server-git,everything|stdio|npx|-y @modelcontextprotocol/server-everything"
   ```

## Usage

### Start the chat:
```bash
cargo run
```

### Available Commands:
- `/help` or `/h` - Show help message
- `/clear` or `/cls` - Clear the screen
- `/quit`, `/exit`, or `/q` - Exit the chat
- `/stream` - Toggle between streaming and non-streaming mode
- `/tools` - List available function tools (including MCP tools)
- `/switch` or `/reset` - Clear conversation history

### Example Session:
```
Gemini Chat CLI
===============
Type your message and press Enter to send.
Commands:
  /help    - Show this help
  /clear   - Clear the screen
  /quit    - Exit the chat
  /stream  - Toggle streaming mode (default: on)
  /tools   - List available tools

You: /tools
Available tools (3):
  - shell_command: Execute a shell command in the current working directory
  - git:git_status: Check the status of the git repository
  - git:git_diff: Show differences in the repository

You: What's the status of my git repository?
Gemini: I'll check the git status for you.

[TOOL] Executing function: git:git_status
Function Result:
On branch main
Your branch is up to date with 'origin/main'.
Changes not staged for commit:
  modified:   README.md

[LLM] Getting LLM response to function result...
LLM Response:
I can see your git repository status. You have modified README.md that hasn't been staged yet.
Would you like me to help you stage and commit these changes?

You: /quit
Goodbye!
```

## MCP (Model Context Protocol) Integration

The chat CLI now supports MCP servers, allowing you to extend functionality with external tools and services.

### Quick Start with MCP

1. **Install an MCP server** (example: Git server):
   ```bash
   pip install mcp-server-git
   ```

2. **Configure the server**:
   ```bash
   export MCP_SERVERS="git|stdio|uvx|mcp-server-git"
   ```

3. **Run the chat CLI**:
   ```bash
   cargo run
   ```

4. **Use MCP tools in conversation**:
   ```
   You: /tools
   Available tools (3):
     - shell_command: Execute a shell command
     - git:git_status: Check git repository status
     - git:git_diff: Show git differences
   
   You: What's the status of my repository?
   AI: I'll check that for you using the git status tool...
   ```

### Popular MCP Servers

- **Git operations**: `git|stdio|uvx|mcp-server-git`
- **File system**: `fs|stdio|npx|-y @modelcontextprotocol/server-filesystem|/path/to/dir`
- **Everything server**: `everything|stdio|npx|-y @modelcontextprotocol/server-everything`

### MCP Configuration Format

```bash
export MCP_SERVERS="name1|transport|endpoint|args,name2|transport|endpoint|args"
```

- **name**: Unique identifier for the server
- **transport**: `stdio` (child process) or `sse` (HTTP server-sent events)
- **endpoint**: Command/executable for stdio, URL for sse
- **args**: Additional arguments (optional, space-separated for stdio)

See `mcp_config_example.md` for detailed configuration examples.

## Architecture

### Core Components:

1. **GeminiClient** (`src/gemini.rs`):
   - Handles API communication with Google's Gemini service
   - Supports both streaming and non-streaming responses
   - Implements proper error handling and response parsing

2. **OpenAIClient** (`src/openai.rs`):
   - Handles API communication with OpenAI services
   - Compatible with OpenAI API and custom endpoints
   - Supports function calling and streaming

3. **MCP Integration** (`src/mcp_client.rs`):
   - Manages connections to MCP servers
   - Translates MCP tools to function calls
   - Supports both stdio and SSE transports

4. **Function Executor** (`src/function_calling.rs`):
   - Executes shell commands and MCP tools
   - Handles function call parsing and response formatting

5. **Main CLI** (`src/main.rs`):
   - Interactive command-line interface
   - User input handling and command processing
   - Response display and formatting

### Key Features:

- **Async/Await**: Built on Tokio for efficient async operations
- **Streaming Support**: Real-time response streaming using Server-Sent Events
- **Error Handling**: Comprehensive error handling with anyhow
- **Environment Config**: Secure API key management with dotenv
- **MCP Protocol**: Full Model Context Protocol support

## Configuration

The application uses environment variables for configuration:

```env
# Choose one API provider
GEMINI_API_KEY=your-gemini-api-key-here
OPENAI_API_KEY=your-openai-api-key-here

# Optional customization
GEMINI_MODEL=gemini-2.0-flash-exp
OPENAI_MODEL=gpt-4
OPENAI_BASE_URL=https://api.openai.com/v1

# Optional MCP server configuration
MCP_SERVERS=git|stdio|uvx|mcp-server-git
```

## Dependencies

- `tokio` - Async runtime
- `reqwest` - HTTP client for API calls
- `serde` - JSON serialization/deserialization
- `anyhow` - Error handling
- `futures` - Stream processing
- `dotenv` - Environment variable management
- `crossterm` - Terminal manipulation
- `rmcp` - Model Context Protocol implementation

## Development

### Build:
```bash
cargo build
```

### Run:
```bash
cargo run
```

### Run specific binaries:
```bash
# RAG demo
cargo run --bin rag

# SSE client test
cargo run --bin sse_client
```

## API Integration

The chat CLI integrates with multiple AI providers:

### Gemini API
- **Model**: `gemini-2.0-flash-exp` (configurable)
- **Endpoint**: `https://generativelanguage.googleapis.com/v1beta/models`
- **Features**: Streaming and non-streaming content generation

### OpenAI API
- **Model**: `gpt-4` (configurable)
- **Endpoint**: `https://api.openai.com/v1` (configurable)
- **Features**: Function calling, streaming, custom endpoints

### Request Configuration:
- Temperature: 0.7
- Top-P: 0.95
- Top-K: 40
- Max Output Tokens: 2048

## Troubleshooting

### Common Issues:

1. **API Key Error**:
   ```
   No API key found. Please set either OPENAI_API_KEY or GEMINI_API_KEY
   ```
   Solution: Copy `.env.example` to `.env` and add your API key.

2. **MCP Server Connection Failed**:
   ```
   [MCP] Failed to connect to MCP server 'git': ...
   ```
   Solution: Make sure the MCP server is installed and accessible.

3. **Build Errors**:
   Make sure you have Rust installed and run `cargo build`.

4. **Network Issues**:
   Check your internet connection and API key validity.

## License

This project is open source. Feel free to use and modify as needed.