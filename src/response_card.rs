use std::io::{self, Write};

/// Formats and displays responses in a bordered card format
pub struct ResponseCard {
    width: usize,
    title: String,
}

impl ResponseCard {
    /// Create a new response card with default settings
    pub fn new() -> Self {
        Self {
            width: 120, // Default width that fits most terminals
            title: "Response".to_string(),
        }
    }

    /// Create a new response card with custom title
    pub fn with_title(title: &str) -> Self {
        Self {
            width: 120,
            title: title.to_string(),
        }
    }

    /// Set the width of the response card
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Display a complete response in a bordered card
    pub fn display_complete(&self, content: &str) -> io::Result<()> {
        self.print_header()?;
        self.print_content(content)?;
        self.print_footer()?;
        println!(); // Extra line for spacing
        Ok(())
    }

    /// Start a streaming response card (prints header only)
    pub fn start_streaming(&self) -> io::Result<()> {
        self.print_header()?;
        print!("│ ");
        io::stdout().flush()?;
        Ok(())
    }

    /// Add content to a streaming response (no borders, just content)
    pub fn stream_content(&self, chunk: &str) -> io::Result<()> {
        // Handle line breaks in streaming content
        let lines: Vec<&str> = chunk.split('\n').collect();
        
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                // New line - start with border
                println!();
                print!("│ ");
            }
            print!("{}", line);
            io::stdout().flush()?;
        }
        Ok(())
    }

    /// End a streaming response card (prints footer)
    pub fn end_streaming(&self) -> io::Result<()> {
        // Ensure we're on a new line and add padding to reach the border
        println!();
        self.print_footer()?;
        println!(); // Extra line for spacing
        Ok(())
    }

    /// Print the top border with title
    fn print_header(&self) -> io::Result<()> {
        let title_with_spaces = format!(" {} ", self.title);
        let title_len = title_with_spaces.len();
        
        // Calculate padding for centering the title
        let remaining_width = self.width.saturating_sub(2); // Account for corner characters
        let left_padding = (remaining_width.saturating_sub(title_len)) / 2;
        let right_padding = remaining_width.saturating_sub(title_len).saturating_sub(left_padding);
        
        print!("╭");
        print!("{}", "─".repeat(left_padding));
        print!("{}", title_with_spaces);
        print!("{}", "─".repeat(right_padding));
        println!("╮");
        
        Ok(())
    }

    /// Print the bottom border
    fn print_footer(&self) -> io::Result<()> {
        print!("╰");
        print!("{}", "─".repeat(self.width.saturating_sub(2)));
        println!("╯");
        Ok(())
    }

    /// Print content with proper word wrapping and borders
    fn print_content(&self, content: &str) -> io::Result<()> {
        let content_width = self.width.saturating_sub(4); // Account for "│ " on both sides
        
        for line in content.lines() {
            if line.is_empty() {
                println!("│{}", " ".repeat(self.width.saturating_sub(2)));
                continue;
            }
            
            // Word wrap long lines
            let wrapped_lines = self.wrap_text(line, content_width);
            for wrapped_line in wrapped_lines {
                let padding = content_width.saturating_sub(wrapped_line.len());
                println!("│ {}{} │", wrapped_line, " ".repeat(padding));
            }
        }
        Ok(())
    }

    /// Wrap text to fit within the specified width
    fn wrap_text(&self, text: &str, width: usize) -> Vec<String> {
        if text.len() <= width {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
}

impl Default for ResponseCard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text() {
        let card = ResponseCard::new();
        let text = "This is a very long line that should be wrapped properly";
        let wrapped = card.wrap_text(text, 20);
        
        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(line.len() <= 20);
        }
    }

    #[test]
    fn test_card_creation() {
        let card = ResponseCard::new();
        assert_eq!(card.title, "Response");
        assert_eq!(card.width, 120);

        let custom_card = ResponseCard::with_title("Custom").with_width(80);
        assert_eq!(custom_card.title, "Custom");
        assert_eq!(custom_card.width, 80);
    }
}