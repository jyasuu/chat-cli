# Gemini Chat CLI

A terminal-based chat interface for Google's Gemini AI using Rust and Ratatui.

## Features

- Interactive terminal UI with Ratatui
- Real-time chat with Gemini AI
- Message history with timestamps
- Scrollable chat view
- Text wrapping for long messages
- Error handling and display
- Loading indicators
- Keyboard shortcuts

## Prerequisites

- Rust 1.70 or later
- Gemini API key from Google AI Studio

## Installation

1. Clone the repository or create the project files
2. Run `cargo build --release`

## Usage

### Set up API Key

Get your Gemini API key from [Google AI Studio](https://makersuite.google.com/app/apikey).

### Run the CLI

```bash
# Using environment variable
export GEMINI_API_KEY=your_api_key_here
cargo run

# Or pass directly as argument
cargo run -- --api-key your_api_key_here

# Use a different model
cargo run -- --api-key your_api_key_here --model gemini-1.5-pro
```

### Keyboard Controls

- **Normal Mode:**
  - `i` - Switch to editing mode
  - `q` - Quit the application
  - `c` - Clear chat history
  - `↑/↓` - Scroll through messages

- **Editing Mode:**
  - `Enter` - Send message
  - `Esc` - Return to normal mode
  - `Backspace` - Delete character
  - Type normally to compose message

## Configuration

You can configure the following options:

- `--api-key, -a` - Your Gemini API key (can also use `GEMINI_API_KEY` env var)
- `--model, -m` - Model to use (default: `gemini-1.5-flash`)

Available models:
- `gemini-1.5-flash` (default, fast and efficient)
- `gemini-1.5-pro` (more capable, slower)
- `gemini-1.0-pro` (older model)

## Project Structure

- `main.rs` - Main application loop and terminal setup
- `gemini.rs` - Gemini API client implementation
- `ui.rs` - UI state management and data structures
- `Cargo.toml` - Dependencies and project configuration

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Async runtime
- `reqwest` - HTTP client for API calls
- `serde` - Serialization/deserialization
- `anyhow` - Error handling
- `clap` - Command line argument parsing
- `chrono` - Date/time handling
- `textwrap` - Text wrapping utilities

## Error Handling

The application handles various error scenarios:

- Network connectivity issues
- Invalid API responses
- API rate limits
- Authentication errors
- Terminal display errors

Errors are displayed in the chat with a red color and "Error" prefix.

## Contributing

Feel free to submit issues and enhancement requests!

## License

This project is open source and available under the MIT License.