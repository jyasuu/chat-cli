# Gemini Chat CLI

A command-line interface for chatting with Google's Gemini AI model, built in Rust.

## Features

- **Interactive Chat**: Real-time conversation with Gemini AI
- **Streaming Responses**: See responses as they're generated (default mode)
- **Non-streaming Mode**: Get complete responses at once
- **Built-in Commands**: Help, clear screen, quit, toggle streaming
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
   # Edit .env and add your Gemini API key
   ```

3. **Get a Gemini API key:**
   - Go to [Google AI Studio](https://aistudio.google.com/app/apikey)
   - Create a new API key
   - Copy it to your `.env` file

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

You: Hello! Can you explain what Rust is?
Gemini: Rust is a systems programming language that focuses on safety, speed, and concurrency...

You: /stream
Streaming mode: OFF

You: What are the main benefits of Rust?
Gemini: The main benefits of Rust include memory safety, performance, and fearless concurrency...

You: /quit
Goodbye!
```

## Architecture

### Core Components:

1. **GeminiClient** (`src/gemini.rs`):
   - Handles API communication with Google's Gemini service
   - Supports both streaming and non-streaming responses
   - Implements proper error handling and response parsing

2. **Main CLI** (`src/main.rs`):
   - Interactive command-line interface
   - User input handling and command processing
   - Response display and formatting

3. **Additional Tools**:
   - `src/bin/rag.rs` - RAG (Retrieval-Augmented Generation) implementation
   - `src/bin/sse_client.rs` - Server-Sent Events client for testing

### Key Features:

- **Async/Await**: Built on Tokio for efficient async operations
- **Streaming Support**: Real-time response streaming using Server-Sent Events
- **Error Handling**: Comprehensive error handling with anyhow
- **Environment Config**: Secure API key management with dotenv

## Dependencies

- `tokio` - Async runtime
- `reqwest` - HTTP client for API calls
- `serde` - JSON serialization/deserialization
- `anyhow` - Error handling
- `futures` - Stream processing
- `dotenv` - Environment variable management
- `crossterm` - Terminal manipulation

## Configuration

The application uses environment variables for configuration:

```env
GEMINI_API_KEY=your-actual-api-key-here
```

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

The chat CLI integrates with Google's Gemini API using:
- **Model**: `gemini-2.0-flash-exp` (configurable)
- **Endpoint**: `https://generativelanguage.googleapis.com/v1beta/models`
- **Features**: Streaming and non-streaming content generation

### Request Configuration:
- Temperature: 0.7
- Top-P: 0.95
- Top-K: 40
- Max Output Tokens: 2048

## Troubleshooting

### Common Issues:

1. **API Key Error**:
   ```
   GEMINI_API_KEY environment variable not set
   ```
   Solution: Copy `.env.example` to `.env` and add your API key.

2. **Build Errors**:
   Make sure you have Rust installed and run `cargo build`.

3. **Network Issues**:
   Check your internet connection and API key validity.

## License

This project is open source. Feel free to use and modify as needed.