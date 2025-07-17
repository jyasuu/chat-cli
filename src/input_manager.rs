use std::collections::HashMap;
use std::io::{self, Write};
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use serde::{Deserialize, Serialize};
use rpassword::read_password;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInput {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub password: bool,
    #[serde(default)]
    pub required: bool,
}

/// Manages user inputs for MCP configuration
pub struct InputManager {
    inputs: HashMap<String, McpInput>,
    resolved_values: HashMap<String, String>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            resolved_values: HashMap::new(),
        }
    }

    /// Register input definitions from MCP config
    pub fn register_inputs(&mut self, inputs: Vec<McpInput>) {
        for input in inputs {
            self.inputs.insert(input.id.clone(), input);
        }
    }

    /// Get a resolved input value, prompting user if not available
    pub fn get_input_value(&mut self, input_id: &str) -> io::Result<Option<String>> {
        // Check if already resolved
        if let Some(value) = self.resolved_values.get(input_id) {
            return Ok(Some(value.clone()));
        }

        // Check environment variable first
        if let Ok(env_value) = std::env::var(&input_id.to_uppercase()) {
            self.resolved_values.insert(input_id.to_string(), env_value.clone());
            return Ok(Some(env_value));
        }

        // Get input definition
        let input_def = match self.inputs.get(input_id) {
            Some(def) => def,
            None => {
                println!("Warning: Unknown input variable '{}'", input_id);
                return Ok(None);
            }
        };

        // Prompt user for input
        let value = self.prompt_user_input(input_def)?;
        if let Some(ref val) = value {
            self.resolved_values.insert(input_id.to_string(), val.clone());
        }

        Ok(value)
    }

    /// Prompt user for a specific input
    fn prompt_user_input(&self, input_def: &McpInput) -> io::Result<Option<String>> {
        // Clear screen and show input prompt
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        
        // Display header
        execute!(io::stdout(), SetForegroundColor(Color::Cyan))?;
        println!("╭─────────────────────────────────────────────────────────────────────────────────────────────────╮");
        println!("│                                    MCP INPUT REQUIRED                                              │");
        println!("╰─────────────────────────────────────────────────────────────────────────────────────────────────╯");
        execute!(io::stdout(), ResetColor)?;
        
        println!();
        execute!(io::stdout(), SetForegroundColor(Color::Yellow))?;
        println!("Input ID: {}", input_def.id);
        execute!(io::stdout(), ResetColor)?;
        println!("Description: {}", input_def.description);
        println!("Type: {}", input_def.input_type);
        
        if input_def.required {
            execute!(io::stdout(), SetForegroundColor(Color::Red))?;
            println!("Required: Yes");
            execute!(io::stdout(), ResetColor)?;
        }
        
        println!();

        // Show input box
        let width = 100;
        println!("╭{}╮", "─".repeat(width - 2));
        
        if input_def.password {
            print!("│ Enter value (hidden): ");
            io::stdout().flush()?;
            
            // Position cursor and get password input
            let password = read_password()?;
            println!("{} │", " ".repeat(width - 25 - password.len().min(20)));
            
            if password.is_empty() && input_def.required {
                execute!(io::stdout(), SetForegroundColor(Color::Red))?;
                println!("│ Error: This input is required and cannot be empty{} │", 
                        " ".repeat(width - 50));
                execute!(io::stdout(), ResetColor)?;
                println!("╰{}╯", "─".repeat(width - 2));
                println!("\nPress Enter to try again...");
                let mut dummy = String::new();
                io::stdin().read_line(&mut dummy)?;
                return self.prompt_user_input(input_def);
            }
            
            println!("╰{}╯", "─".repeat(width - 2));
            Ok(if password.is_empty() { None } else { Some(password) })
        } else {
            print!("│ Enter value: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_string();
            
            println!("│{} │", " ".repeat(width - 4));
            println!("╰{}╯", "─".repeat(width - 2));
            
            if input.is_empty() && input_def.required {
                execute!(io::stdout(), SetForegroundColor(Color::Red))?;
                println!("\nError: This input is required and cannot be empty");
                execute!(io::stdout(), ResetColor)?;
                println!("Press Enter to try again...");
                let mut dummy = String::new();
                io::stdin().read_line(&mut dummy)?;
                return self.prompt_user_input(input_def);
            }
            
            Ok(if input.is_empty() { None } else { Some(input) })
        }
    }

    /// Resolve environment variable placeholders in a string
    pub fn resolve_env_vars(&mut self, value: &str) -> io::Result<String> {
        if value.starts_with("${input:") && value.ends_with("}") {
            let input_id = &value[8..value.len()-1];
            match self.get_input_value(input_id)? {
                Some(resolved) => Ok(resolved),
                None => {
                    if let Some(input_def) = self.inputs.get(input_id) {
                        if input_def.required {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!("Required input '{}' was not provided", input_id)
                            ));
                        }
                    }
                    Ok(String::new())
                }
            }
        } else {
            Ok(value.to_string())
        }
    }

    /// Check if any unresolved inputs are needed for the given environment variables
    pub fn check_dependencies(&self, env_vars: &HashMap<String, String>) -> Vec<String> {
        let mut missing_inputs = Vec::new();
        
        for value in env_vars.values() {
            if value.starts_with("${input:") && value.ends_with("}") {
                let input_id = &value[8..value.len()-1];
                if !self.resolved_values.contains_key(input_id) && 
                   std::env::var(&input_id.to_uppercase()).is_err() {
                    missing_inputs.push(input_id.to_string());
                }
            }
        }
        
        missing_inputs
    }

    /// Get all resolved values
    pub fn get_resolved_values(&self) -> &HashMap<String, String> {
        &self.resolved_values
    }

    /// Clear all resolved values (useful for re-prompting)
    pub fn clear_resolved_values(&mut self) {
        self.resolved_values.clear();
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}