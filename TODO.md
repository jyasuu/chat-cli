# help me implement function tools into chat cli

## reference code implement in gemini-cli/packages/core/src/tools/*

## include below tools
```json

{
    "name": "list_directory",
    "description": "Lists the names of files and subdirectories directly within a specified directory path. Can optionally ignore entries matching provided glob patterns.",
    "parameters": {
        "properties": {
            "path": {
                "description": "The absolute path to the directory to list (must be absolute, not relative)",
                "type": "STRING"
            },
            "ignore": {
                "description": "List of glob patterns to ignore",
                "items": {
                    "type": "STRING"
                },
                "type": "ARRAY"
            },
            "respect_git_ignore": {
                "description": "Optional: Whether to respect .gitignore patterns when listing files. Only available in git repositories. Defaults to true.",
                "type": "BOOLEAN"
            }
        },
        "required": [
            "path"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "read_file",
    "description": "Reads and returns the content of a specified file from the local filesystem. Handles text, images (PNG, JPG, GIF, WEBP, SVG, BMP), and PDF files. For text files, it can read specific line ranges.",
    "parameters": {
        "properties": {
            "absolute_path": {
                "description": "The absolute path to the file to read (e.g., '/home/user/project/file.txt'). Relative paths are not supported. You must provide an absolute path.",
                "type": "STRING"
            },
            "offset": {
                "description": "Optional: For text files, the 0-based line number to start reading from. Requires 'limit' to be set. Use for paginating through large files.",
                "type": "NUMBER"
            },
            "limit": {
                "description": "Optional: For text files, maximum number of lines to read. Use with 'offset' to paginate through large files. If omitted, reads the entire file (if feasible, up to a default limit).",
                "type": "NUMBER"
            }
        },
        "required": [
            "absolute_path"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "search_file_content",
    "description": "Searches for a regular expression pattern within the content of files in a specified directory (or current working directory). Can filter files by a glob pattern. Returns the lines containing matches, along with their file paths and line numbers.",
    "parameters": {
        "properties": {
            "pattern": {
                "description": "The regular expression (regex) pattern to search for within file contents (e.g., 'function\\s+myFunction', 'import\\s+\\{.*\\}\\s+from\\s+.*').",
                "type": "STRING"
            },
            "path": {
                "description": "Optional: The absolute path to the directory to search within. If omitted, searches the current working directory.",
                "type": "STRING"
            },
            "include": {
                "description": "Optional: A glob pattern to filter which files are searched (e.g., '*.js', '*.{ts,tsx}', 'src/**'). If omitted, searches all files (respecting potential global ignores).",
                "type": "STRING"
            }
        },
        "required": [
            "pattern"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "glob",
    "description": "Efficiently finds files matching specific glob patterns (e.g., `src/**/*.ts`, `**/*.md`), returning absolute paths sorted by modification time (newest first). Ideal for quickly locating files based on their name or path structure, especially in large codebases.",
    "parameters": {
        "properties": {
            "pattern": {
                "description": "The glob pattern to match against (e.g., '**/*.py', 'docs/*.md').",
                "type": "STRING"
            },
            "path": {
                "description": "Optional: The absolute path to the directory to search within. If omitted, searches the root directory.",
                "type": "STRING"
            },
            "case_sensitive": {
                "description": "Optional: Whether the search should be case-sensitive. Defaults to false.",
                "type": "BOOLEAN"
            },
            "respect_git_ignore": {
                "description": "Optional: Whether to respect .gitignore patterns when finding files. Only available in git repositories. Defaults to true.",
                "type": "BOOLEAN"
            }
        },
        "required": [
            "pattern"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "replace",
    "description": "Replaces text within a file. By default, replaces a single occurrence, but can replace multiple occurrences when `expected_replacements` is specified. This tool requires providing significant context around the change to ensure precise targeting. Always use the read_file tool to examine the file's current content before attempting a text replacement.\n\n      The user has the ability to modify the `new_string` content. If modified, this will be stated in the response.\n\nExpectation for required parameters:\n1. `file_path` MUST be an absolute path; otherwise an error will be thrown.\n2. `old_string` MUST be the exact literal text to replace (including all whitespace, indentation, newlines, and surrounding code etc.).\n3. `new_string` MUST be the exact literal text to replace `old_string` with (also including all whitespace, indentation, newlines, and surrounding code etc.). Ensure the resulting code is correct and idiomatic.\n4. NEVER escape `old_string` or `new_string`, that would break the exact literal text requirement.\n**Important:** If ANY of the above are not satisfied, the tool will fail. CRITICAL for `old_string`: Must uniquely identify the single instance to change. Include at least 3 lines of context BEFORE and AFTER the target text, matching whitespace and indentation precisely. If this string matches multiple locations, or does not match exactly, the tool will fail.\n**Multiple replacements:** Set `expected_replacements` to the number of occurrences you want to replace. The tool will replace ALL occurrences that match `old_string` exactly. Ensure the number of replacements matches your expectation.",
    "parameters": {
        "properties": {
            "file_path": {
                "description": "The absolute path to the file to modify. Must start with '/'.",
                "type": "STRING"
            },
            "old_string": {
                "description": "The exact literal text to replace, preferably unescaped. For single replacements (default), include at least 3 lines of context BEFORE and AFTER the target text, matching whitespace and indentation precisely. For multiple replacements, specify expected_replacements parameter. If this string is not the exact literal text (i.e. you escaped it) or does not match exactly, the tool will fail.",
                "type": "STRING"
            },
            "new_string": {
                "description": "The exact literal text to replace `old_string` with, preferably unescaped. Provide the EXACT text. Ensure the resulting code is correct and idiomatic.",
                "type": "STRING"
            },
            "expected_replacements": {
                "type": "NUMBER",
                "description": "Number of replacements expected. Defaults to 1 if not specified. Use when you want to replace multiple occurrences.",
                "minimum": 1
            }
        },
        "required": [
            "file_path",
            "old_string",
            "new_string"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "write_file",
    "description": "Writes content to a specified file in the local filesystem. \n      \n      The user has the ability to modify `content`. If modified, this will be stated in the response.",
    "parameters": {
        "properties": {
            "file_path": {
                "description": "The absolute path to the file to write to (e.g., '/home/user/project/file.txt'). Relative paths are not supported.",
                "type": "STRING"
            },
            "content": {
                "description": "The content to write to the file.",
                "type": "STRING"
            }
        },
        "required": [
            "file_path",
            "content"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "web_fetch",
    "description": "Processes content from URL(s), including local and private network addresses (e.g., localhost), embedded in a prompt. Include up to 20 URLs and instructions (e.g., summarize, extract specific data) directly in the 'prompt' parameter.",
    "parameters": {
        "properties": {
            "prompt": {
                "description": "A comprehensive prompt that includes the URL(s) (up to 20) to fetch and specific instructions on how to process their content (e.g., \"Summarize https://example.com/article and extract key points from https://another.com/data\"). Must contain as least one URL starting with http:// or https://.",
                "type": "STRING"
            }
        },
        "required": [
            "prompt"
        ],
        "type": "OBJECT"
    }
},
{
    "name": "read_many_files",
    "description": "Reads content from multiple files specified by paths or glob patterns within a configured target directory. For text files, it concatenates their content into a single string. It is primarily designed for text-based files. However, it can also process image (e.g., .png, .jpg) and PDF (.pdf) files if their file names or extensions are explicitly included in the 'paths' argument. For these explicitly requested non-text files, their data is read and included in a format suitable for model consumption (e.g., base64 encoded).\n\nThis tool is useful when you need to understand or analyze a collection of files, such as:\n- Getting an overview of a codebase or parts of it (e.g., all TypeScript files in the 'src' directory).\n- Finding where specific functionality is implemented if the user asks broad questions about code.\n- Reviewing documentation files (e.g., all Markdown files in the 'docs' directory).\n- Gathering context from multiple configuration files.\n- When the user asks to \"read all files in X directory\" or \"show me the content of all Y files\".\n\nUse this tool when the user's query implies needing the content of several files simultaneously for context, analysis, or summarization. For text files, it uses default UTF-8 encoding and a '--- {filePath} ---' separator between file contents. Ensure paths are relative to the target directory. Glob patterns like 'src/**/*.js' are supported. Avoid using for single files if a more specific single-file reading tool is available, unless the user specifically requests to process a list containing just one file via this tool. Other binary files (not explicitly requested as image/PDF) are generally skipped. Default excludes apply to common non-text files (except for explicitly requested images/PDFs) and large dependency directories unless 'useDefaultExcludes' is false.",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "paths": {
                "type": "ARRAY",
                "items": {
                    "type": "STRING",
                    "minLength": "1"
                },
                "minItems": "1",
                "description": "Required. An array of glob patterns or paths relative to the tool's target directory. Examples: ['src/**/*.ts'], ['README.md', 'docs/']"
            },
            "include": {
                "type": "ARRAY",
                "items": {
                    "type": "STRING",
                    "minLength": "1"
                },
                "description": "Optional. Additional glob patterns to include. These are merged with `paths`. Example: [\"*.test.ts\"] to specifically add test files if they were broadly excluded.",
                "default": []
            },
            "exclude": {
                "type": "ARRAY",
                "items": {
                    "type": "STRING",
                    "minLength": "1"
                },
                "description": "Optional. Glob patterns for files/directories to exclude. Added to default excludes if useDefaultExcludes is true. Example: [\"**/*.log\", \"temp/\"]",
                "default": []
            },
            "recursive": {
                "type": "BOOLEAN",
                "description": "Optional. Whether to search recursively (primarily controlled by `**` in glob patterns). Defaults to true.",
                "default": true
            },
            "useDefaultExcludes": {
                "type": "BOOLEAN",
                "description": "Optional. Whether to apply a list of default exclusion patterns (e.g., node_modules, .git, binary files). Defaults to true.",
                "default": true
            },
            "respect_git_ignore": {
                "type": "BOOLEAN",
                "description": "Optional. Whether to respect .gitignore patterns when discovering files. Only available in git repositories. Defaults to true.",
                "default": true
            }
        },
        "required": [
            "paths"
        ]
    }
},
{
    "name": "run_shell_command",
    "description": "This tool executes a given shell command as `bash -c <command>`. Command can start background processes using `&`. Command is executed as a subprocess that leads its own process group. Command process group can be terminated as `kill -- -PGID` or signaled as `kill -s SIGNAL -- -PGID`.\n\nThe following information is returned:\n\nCommand: Executed command.\nDirectory: Directory (relative to project root) where command was executed, or `(root)`.\nStdout: Output on stdout stream. Can be `(empty)` or partial on error and for any unwaited background processes.\nStderr: Output on stderr stream. Can be `(empty)` or partial on error and for any unwaited background processes.\nError: Error or `(none)` if no error was reported for the subprocess.\nExit Code: Exit code or `(none)` if terminated by signal.\nSignal: Signal number or `(none)` if no signal was received.\nBackground PIDs: List of background processes started or `(none)`.\nProcess Group PGID: Process group started or `(none)`",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "command": {
                "type": "STRING",
                "description": "Exact bash command to execute as `bash -c <command>`"
            },
            "description": {
                "type": "STRING",
                "description": "Brief description of the command for the user. Be specific and concise. Ideally a single sentence. Can be up to 3 sentences for clarity. No line breaks."
            },
            "directory": {
                "type": "STRING",
                "description": "(OPTIONAL) Directory to run the command in, if not the project root directory. Must be relative to the project root directory and must already exist."
            }
        },
        "required": [
            "command"
        ]
    }
},
{
    "name": "save_memory",
    "description": "\nSaves a specific piece of information or fact to your long-term memory.\n\nUse this tool:\n\n- When the user explicitly asks you to remember something (e.g., \"Remember that I like pineapple on pizza\", \"Please save this: my cat's name is Whiskers\").\n- When the user states a clear, concise fact about themselves, their preferences, or their environment that seems important for you to retain for future interactions to provide a more personalized and effective assistance.\n\nDo NOT use this tool:\n\n- To remember conversational context that is only relevant for the current session.\n- To save long, complex, or rambling pieces of text. The fact should be relatively short and to the point.\n- If you are unsure whether the information is a fact worth remembering long-term. If in doubt, you can ask the user, \"Should I remember that for you?\"\n\n## Parameters\n\n- `fact` (string, required): The specific fact or piece of information to remember. This should be a clear, self-contained statement. For example, if the user says \"My favorite color is blue\", the fact would be \"My favorite color is blue\".\n",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "fact": {
                "type": "STRING",
                "description": "The specific fact or piece of information to remember. Should be a clear, self-contained statement."
            }
        },
        "required": [
            "fact"
        ]
    }
},
{
    "name": "google_web_search",
    "description": "Performs a web search using Google Search (via the Gemini API) and returns the results. This tool is useful for finding information on the internet based on a query.",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "query": {
                "type": "STRING",
                "description": "The search query to find information on the web."
            }
        },
        "required": [
            "query"
        ]
    }
}
```                