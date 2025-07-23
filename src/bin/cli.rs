use std::io::{self, Write};
use crossterm::{
    cursor, event,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use chat_cli::ui::input_box;


struct InputBox {
    lines: Vec<String>,
    current_line: usize,
    title: String,
}

impl InputBox {
    fn new(title: &str) -> Self {
        Self {
            lines: vec![String::new()],
            current_line: 0,
            title: title.to_string(),
        }
    }

    fn add_char(&mut self, c: char) {
        if self.current_line < self.lines.len() {
            self.lines[self.current_line].push(c);
        }
    }

    fn add_newline(&mut self) {
        self.current_line += 1;
        self.lines.push(String::new());
    }

    fn remove_char(&mut self) {
        if self.current_line < self.lines.len() && !self.lines[self.current_line].is_empty() {
            self.lines[self.current_line].pop();
        } else if self.current_line > 0 {
            // If current line is empty and we're not on the first line,
            // move to previous line and remove the empty line
            self.lines.remove(self.current_line);
            self.current_line -= 1;
        }
    }

    fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.current_line = 0;
    }

    fn draw(&self) -> io::Result<()> {
        let max_line_length = self.lines.iter().map(|line| line.len()).max().unwrap_or(0);
        let box_width = std::cmp::max(max_line_length + 4, self.title.len() + 8);
        let box_width = std::cmp::max(box_width, 45); // Minimum width
        
        let mut stdout = io::stdout();

        // Move cursor to column 0 (left side)
        queue!(stdout, cursor::MoveToColumn(0))?;

        // Top border with title
        queue!(stdout, SetForegroundColor(Color::Cyan))?;
        queue!(stdout, Print("╭─ "))?;
        queue!(stdout, Print(&self.title))?;
        queue!(stdout, Print(" "))?;
        for _ in 0..(box_width - self.title.len() - 4) {
            queue!(stdout, Print("─"))?;
        }
        queue!(stdout, Print("╮"))?;
        queue!(stdout, Print("\n"))?;

        // Content lines
        for (i, line) in self.lines.iter().enumerate() {
            queue!(stdout, cursor::MoveToColumn(0))?;
            queue!(stdout, Print("│ "))?;
            queue!(stdout, ResetColor)?;
            queue!(stdout, Print(line))?;
            
            // Add cursor indicator on current line
            if i == self.current_line {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
                queue!(stdout, cursor::MoveToColumn(box_width as u16))?;
                queue!(stdout, Print("│"))?; // Simple cursor indicator
            }
            
            queue!(stdout, SetForegroundColor(Color::Cyan))?;
            let padding = if i == self.current_line {
                box_width - line.len() - 3 // Account for cursor
            } else {
                box_width - line.len() - 2
            };
            
            for _ in 0..padding {
                queue!(stdout, Print(" "))?;
            }
            
            if i != self.current_line {
                queue!(stdout, Print("│"))?;
            }
            queue!(stdout, Print("\n"))?;
        }

        // Bottom border
        queue!(stdout, cursor::MoveToColumn(0))?;
        queue!(stdout, Print("╰"))?;
        for _ in 0..(box_width-1) {
            queue!(stdout, Print("─"))?;
        }
        queue!(stdout, Print("╯"))?;
        queue!(stdout, Print("\n"))?;
        queue!(stdout, ResetColor)?;

        stdout.flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut test_box = input_box::InputBox::new("Test Input");
    let res = test_box.get_input();
    match res {
        Ok(str) => println!("{str}"),
        Err(_) => panic!("Input Error"),
    }



    // Enable raw mode for character-by-character input
    // terminal::enable_raw_mode()?;
    
    // let mut stdout = io::stdout();
    // execute!(stdout, cursor::Hide)?;

    // println!("Rust CLI Key Input Demo");
    // println!("Type characters to see them in the box.");
    // println!("Press Alt+Enter to add a new line, Enter to create a new box, Esc to quit.\n");

    // let mut input_box = InputBox::new("Input");
    // let mut box_counter = 1;

    // // Initial draw
    // input_box.draw()?;

    // loop {
    //     // Read input events
    //     if event::poll(std::time::Duration::from_millis(100))? {
    //         if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
    //             match code {
    //                 KeyCode::Char(c) => {
    //                     // Calculate current box height for clearing
    //                     let box_height = input_box.lines.len() + 2; // +2 for top and bottom borders
                        
    //                     // Clear the current box display and move cursor to start
    //                     execute!(stdout, cursor::MoveUp(box_height as u16))?;
    //                     execute!(stdout, cursor::MoveToColumn(0))?;
    //                     execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                        
    //                     // Add character and redraw
    //                     input_box.add_char(c);
    //                     input_box.draw()?;
    //                 }
    //                 KeyCode::Backspace => {
    //                     // Calculate current box height for clearing
    //                     let box_height = input_box.lines.len() + 2; // +2 for top and bottom borders
                        
    //                     // Clear the current box display and move cursor to start
    //                     execute!(stdout, cursor::MoveUp(box_height as u16))?;
    //                     execute!(stdout, cursor::MoveToColumn(0))?;
    //                     execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                        
    //                     // Remove character and redraw
    //                     input_box.remove_char();
    //                     input_box.draw()?;
    //                 }
    //                 KeyCode::Enter => {
    //                     if !modifiers.contains(KeyModifiers::ALT) {
    //                         // Alt+Enter: Add new line within the box
    //                         let box_height = input_box.lines.len() + 2;
                            
    //                         execute!(stdout, cursor::MoveUp(box_height as u16))?;
    //                         execute!(stdout, cursor::MoveToColumn(0))?;
    //                         execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                            
    //                         input_box.add_newline();
    //                         input_box.draw()?;
    //                     } else {
    //                         // Regular Enter: Create new box
    //                         box_counter += 1;
    //                         input_box = InputBox::new(&format!("Input #{}", box_counter));
                            
    //                         println!(); // Add some spacing
    //                         input_box.draw()?;
    //                     }
    //                 }
    //                 KeyCode::Esc => {
    //                     break;
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    // }

    // // Cleanup
    // execute!(stdout, cursor::Show)?;
    // terminal::disable_raw_mode()?;
    // println!("\nGoodbye!");

    Ok(())
}

// Add this to your Cargo.toml:
/*
[package]
name = "key-input-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
crossterm = "0.27"
*/