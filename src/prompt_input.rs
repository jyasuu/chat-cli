use std::io::{self, Write};
use crossterm::{
    cursor,
    execute,
};

/// A fancy prompt input interface with bordered input box
pub struct PromptInput {
    width: usize,
    prompt_text: String,
}

impl PromptInput {
    pub fn new() -> Self {
        Self {
            width: 120,
            prompt_text: "> ".to_string(),
        }
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Display the fancy input prompt and get user input
    pub fn get_input(&self) -> io::Result<String> {
        // Draw the complete input box with bottom border and help text BEFORE input
        self.draw_complete_input_box_with_help()?;
        
        // Move cursor back up to the input line and position after "│ > "
        execute!(io::stdout(), cursor::MoveUp(3), cursor::MoveToColumn(5))?;
        
        // Get input using standard input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        // Clear the help text line after input submission
        execute!(io::stdout(), cursor::MoveDown(1), cursor::MoveToColumn(1))?;
        println!("{}", " ".repeat(120)); // Clear the help text line
        execute!(io::stdout(), cursor::MoveUp(1))?; // Move back up
        
        Ok(input.trim().to_string())
    }

    /// Draw a complete input box with help text that matches the target design
    fn draw_complete_input_box_with_help(&self) -> io::Result<()> {
        let content_width = self.width.saturating_sub(4); // Account for "│ " and " │"
        
        // Top border
        println!("╭{}╮", "─".repeat(self.width.saturating_sub(2)));
        
        // Input line with prompt and padding to complete the box
        let remaining_space = content_width.saturating_sub(2); // Account for "> "
        println!("│ > {}{} │", " ".repeat(remaining_space), "");
        
        // Bottom border
        println!("╰{}╯", "─".repeat(self.width.saturating_sub(2)));
        
        // Help text
        println!("Type \"/\" for available commands.                                                Uses AI. Verify results.");
        
        Ok(())
    }



}

impl Default for PromptInput {
    fn default() -> Self {
        Self::new()
    }
}