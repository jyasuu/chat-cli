╭─ Response ───────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Here are some comprehensive test prompts to help you verify all the implemented function tools:                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                 🔍 File System Tools                                                 │
│                                                                                                                      │
│                                                    list_directory                                                    │
│                                                                                                                      │
│                                                                                                                      │
│  "List all files in the current directory"                                                                           │
│                                                                                                                      │
│  "Show me all Rust files in the src directory, ignoring any test files"                                              │
│                                                                                                                      │
│  "List the contents of /workspace but ignore any .git, node_modules, and target directories"                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                      read_file                                                       │
│                                                                                                                      │
│                                                                                                                      │
│  "Read the contents of src/main.rs"                                                                                  │
│                                                                                                                      │
│  "Show me the first 20 lines of Cargo.toml"                                                                          │
│                                                                                                                      │
│  "Read lines 50-100 from src/tools/mod.rs"                                                                           │
│                                                                                                                      │
│  "What's in the README.md file?"                                                                                     │
│                                                                                                                      │
│                                                                                                                      │
│                                                      write_file                                                      │
│                                                                                                                      │
│                                                                                                                      │
│  "Create a new file called test_output.txt with the content 'Hello from chat CLI tools!'"                            │
│                                                                                                                      │
│  "Write a simple Python script to /tmp/hello.py that prints 'Hello World'"                                           │
│                                                                                                                      │
│  "Create a markdown file at ./test_results.md documenting our tool tests"                                            │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                             🔎 Search & Discovery Tools                                              │
│                                                                                                                      │
│                                                 search_file_content                                                  │
│                                                                                                                      │
│                                                                                                                      │
│  "Search for all occurrences of 'async fn' in the src directory"                                                     │
│                                                                                                                      │
│  "Find all TODO comments in Rust files"                                                                              │
│                                                                                                                      │
│  "Search for 'ToolDefinition' in the codebase"                                                                       │
│                                                                                                                      │
│  "Look for any error handling patterns using 'Result<' in TypeScript files"                                          │
│                                                                                                                      │
│                                                                                                                      │
│                                                         glob                                                         │
│                                                                                                                      │
│                                                                                                                      │
│  "Find all Rust files in the project"                                                                                │
│                                                                                                                      │
│  "Show me all markdown files, sorted by modification time"                                                           │
│                                                                                                                      │
│  "Find all configuration files (*.toml, *.json, *.yaml)"                                                             │
│                                                                                                                      │
│  "List all test files in the project"                                                                                │
│                                                                                                                      │
│                                                                                                                      │
│                                                   read_many_files                                                    │
│                                                                                                                      │
│                                                                                                                      │
│  "Read all Cargo.toml files in the project"                                                                          │
│                                                                                                                      │
│  "Show me the contents of all README files"                                                                          │
│                                                                                                                      │
│  "Read all the tool implementation files in src/tools/"                                                              │
│                                                                                                                      │
│  "Get the content of all TypeScript configuration files"                                                             │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                               🔧 Text Processing Tools                                               │
│                                                                                                                      │
│                                                       replace                                                        │
│                                                                                                                      │
│                                                                                                                      │
│  "In the file src/main.rs, replace the line containing 'println!("Hello")' with 'println!("Hello from updated CLI")  │
│                                                                                                                      │
│  "Update the version number in Cargo.toml from the current version to '0.2.0'"                                       │
│                                                                                                                      │
│  "Replace all occurrences of 'TODO' with 'DONE' in the file test_output.txt"                                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                               🌐 Web & External Tools                                                │
│                                                                                                                      │
│                                                      web_fetch                                                       │
│                                                                                                                      │
│                                                                                                                      │
│  "Fetch the content from https://httpbin.org/json and summarize what you find"                                       │
│                                                                                                                      │
│  "Get the HTML content from https://example.com and extract the main heading"                                        │
│                                                                                                                      │
│  "Fetch https://api.github.com/repos/microsoft/vscode/releases/latest and tell me about the latest VS Code release"  │
│                                                                                                                      │
│                                                                                                                      │
│                                                  run_shell_command                                                   │
│                                                                                                                      │
│                                                                                                                      │
│  "Run 'ls -la' to show detailed directory listing"                                                                   │
│                                                                                                                      │
│  "Execute 'git status' to check the repository status"                                                               │
│                                                                                                                      │
│  "Run 'cargo --version' to check the Rust toolchain version"                                                         │
│                                                                                                                      │
│  "Execute 'find . -name "*.rs" | wc -l' to count Rust files"                                                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                   💾 Memory Tools                                                    │
│                                                                                                                      │
│                                                     save_memory                                                      │
│                                                                                                                      │
│                                                                                                                      │
│  "Remember that I prefer using tabs instead of spaces for indentation"                                               │
│                                                                                                                      │
│  "Save to memory: The project uses async-trait for trait object compatibility"                                       │
│                                                                                                                      │
│  "Remember that this chat CLI supports both MCP tools and built-in function tools"                                   │
│                                                                                                                      │
│                                                                                                                      │
│                                           google_web_search (placeholder)                                            │
│                                                                                                                      │
│                                                                                                                      │
│  "Search the web for 'Rust async trait best practices'"                                                              │
│                                                                                                                      │
│  "Find information about 'MCP protocol implementation examples'"                                                     │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                           🧪 Complex Multi-Tool Workflows                                            │
│                                                                                                                      │
│                                                   Project Analysis                                                   │
│                                                                                                                      │
│                                                                                                                      │
│  "First list the src directory, then read the main.rs file, and search for any TODO comments in the entire codebase  │
│                                                                                                                      │
│  "Find all Rust files, then read the first 10 lines of each to understand the project structure"                     │
│                                                                                                                      │
│                                                                                                                      │
│                                                   Code Exploration                                                   │
│                                                                                                                      │
│                                                                                                                      │
│  "Search for 'Tool' in the codebase, then read the files that contain trait definitions"                             │
│                                                                                                                      │
│  "List all files in src/tools/, then read the mod.rs file to understand the tool registry"                           │
│                                                                                                                      │
│                                                                                                                      │
│                                               Documentation Generation                                               │
│                                                                                                                      │
│                                                                                                                      │
│  "Read all the tool files in src/tools/ and create a summary document of what each tool does"                        │
│                                                                                                                      │
│  "Search for all function signatures in the main.rs file and document them"                                          │
│                                                                                                                      │
│                                                                                                                      │
│                                                 Development Workflow                                                 │
│                                                                                                                      │
│                                                                                                                      │
│  "Check git status, list recent changes, and read any modified Rust files"                                           │
│                                                                                                                      │
│  "Find all test files, read one of them, and run the tests using shell command"                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                   🎯 Error Testing                                                   │
│                                                                                                                      │
│                                                  Invalid Parameters                                                  │
│                                                                                                                      │
│                                                                                                                      │
│  "Read a file that doesn't exist: /nonexistent/file.txt"                                                             │
│                                                                                                                      │
│  "List directory with invalid path: /invalid/path"                                                                   │
│                                                                                                                      │
│  "Search for content with empty pattern"                                                                             │
│                                                                                                                      │
│  "Write to a file in a directory that doesn't exist without parent creation"                                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                      Edge Cases                                                      │
│                                                                                                                      │
│                                                                                                                      │
│  "Read a very large file with pagination (if you have one)"                                                          │
│                                                                                                                      │
│  "Search for a pattern that matches thousands of results"                                                            │
│                                                                                                                      │
│  "Fetch a URL that returns a large response"                                                                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                 💡 Tips for Testing                                                  │
│                                                                                                                      │
│  1 Start Simple: Begin with basic file operations like listing and reading                                           │
│  2 Test Incrementally: Try one tool at a time before combining them                                                  │
│  3 Use Real Files: Test with actual files in your project for realistic scenarios                                    │
│  4 Check Error Handling: Intentionally provide invalid inputs to test error responses                                │
│  5 Combine Tools: Once individual tools work, try workflows that use multiple tools                                  │
│  6 Monitor Output: Pay attention to both the LLM content and user display formats                                    │
│                                                                                                                      │
│ These prompts will help you thoroughly test all the implemented function tools and ensure they're working correctly  │
│ in your chat CLI environment!                                                                                        │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯