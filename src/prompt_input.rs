use std::io::{self, Write};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
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

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt_text = prompt.to_string();
        self
    }

    /// Display the fancy input prompt and get user input
    pub fn get_input(&self) -> io::Result<String> {
        // Draw the input box once
        self.draw_input_box_static()?;
        
        // Use simple input reading
        let mut input = String::new();
        print!("│ > ");
        io::stdout().flush()?;
        
        io::stdin().read_line(&mut input)?;
        
        // Clean up the input box
        self.clear_input_box()?;
        
        Ok(input.trim().to_string())
    }

    /// Simple fallback input method
    pub fn get_input_simple(&self) -> io::Result<String> {
        let mut input = String::new();
        let mut cursor_pos = 0;

        // Enable raw mode for better input control
        terminal::enable_raw_mode()?;

        // Save current cursor position
        execute!(io::stdout(), cursor::SavePosition)?;
        
        // Draw initial input box
        self.draw_input_box(&input, cursor_pos)?;

        loop {
            // Handle input events
            if let Event::Key(key_event) = event::read()? {
                match key_event {
                    KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        terminal::disable_raw_mode()?;
                        // Move cursor below the input box
                        execute!(io::stdout(), cursor::MoveTo(0, 4))?;
                        return Ok(input);
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: KeyModifiers::SHIFT,
                        ..
                    } => {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                            input.remove(cursor_pos);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Delete,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        if cursor_pos < input.len() {
                            input.remove(cursor_pos);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        if cursor_pos < input.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Home,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        cursor_pos = 0;
                    }
                    KeyEvent {
                        code: KeyCode::End,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        cursor_pos = input.len();
                    }
                    KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        terminal::disable_raw_mode()?;
                        execute!(io::stdout(), cursor::MoveTo(0, 3))?;
                        return Ok(String::new());
                    }
                    _ => {}
                }
                
                // Redraw only the input box area, not the whole screen
                execute!(io::stdout(), cursor::RestorePosition)?;
                self.draw_input_box(&input, cursor_pos)?;
            }
        }
    }

    /// Draw a static input box that doesn't interfere with other output
    fn draw_input_box_static(&self) -> io::Result<()> {
        // Top border
        println!("╭{}╮", "─".repeat(self.width.saturating_sub(2)));
        
        // Input line placeholder
        print!("│ > ");
        
        Ok(())
    }

    /// Clear the input box area
    fn clear_input_box(&self) -> io::Result<()> {
        // Bottom border
        println!("╰{}╯", "─".repeat(self.width.saturating_sub(2)));
        Ok(())
    }

    /// Draw the fancy input box with current input and cursor
    fn draw_input_box(&self, input: &str, cursor_pos: usize) -> io::Result<()> {
        let content_width = self.width.saturating_sub(4); // Account for borders and padding
        
        // Top border
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            Print("╭"),
            Print("─".repeat(self.width.saturating_sub(2))),
            Print("╮")
        )?;

        // Input line with prompt and content
        let display_text = format!("{}{}", self.prompt_text, input);
        let padding = content_width.saturating_sub(display_text.len().min(content_width));
        
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 1),
            Print("│ "),
            SetForegroundColor(Color::Cyan),
            Print(&self.prompt_text),
            ResetColor,
            Print(&input),
            Print(" ".repeat(padding)),
            Print(" │")
        )?;

        // Bottom border
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 2),
            Print("╰"),
            Print("─".repeat(self.width.saturating_sub(2))),
            Print("╯")
        )?;

        // Position cursor
        let visual_cursor_pos = self.prompt_text.len() + cursor_pos;
        execute!(
            io::stdout(),
            cursor::MoveTo((2 + visual_cursor_pos) as u16, 1)
        )?;

        // Show help text
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 3),
            SetForegroundColor(Color::DarkGrey),
            Print("Type \"/\" for available commands."),
            Print(" ".repeat(50)),
            Print("Uses AI. Verify results."),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }
}

impl Default for PromptInput {
    fn default() -> Self {
        Self::new()
    }
}