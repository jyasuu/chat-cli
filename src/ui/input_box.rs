use std::io::{self, Write};
use crossterm::{
    cursor, event,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};



pub struct InputBox {
    lines: Vec<String>,
    current_line: usize,
    title: String,
}

impl InputBox {
    pub fn new(title: &str) -> Self {
        Self {
            lines: vec![String::new()],
            current_line: 0,
            title: title.to_string(),
        }
    }

    pub fn add_char(&mut self, c: char) {
        if self.current_line < self.lines.len() {
            self.lines[self.current_line].push(c);
            if self.lines[self.current_line].len() == 120
            {
                self.add_newline();
            }

        }
    }

    pub fn add_newline(&mut self) {
        self.current_line += 1;
        self.lines.push(String::new());
    }

    pub fn remove_char(&mut self) {
        if self.current_line < self.lines.len() && !self.lines[self.current_line].is_empty() {
            self.lines[self.current_line].pop();
        } else if self.current_line > 0 {
            // If current line is empty and we're not on the first line,
            // move to previous line and remove the empty line
            self.lines.remove(self.current_line);
            self.current_line -= 1;
        }
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.current_line = 0;
    }

    pub fn draw(&self) -> io::Result<()> {
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
            
            queue!(stdout, SetForegroundColor(Color::Cyan))?;
            queue!(stdout, cursor::MoveToColumn(box_width as u16))?;
            queue!(stdout, Print("│"))?; // Simple cursor indicator
            queue!(stdout, Print("\n"))?;
            queue!(stdout, cursor::MoveToColumn(0))?;
            
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
        queue!(stdout, cursor::MoveToColumn(0))?;

        stdout.flush()?;
        Ok(())
    }

    pub fn get_input(&mut self) -> io::Result<String>{
        terminal::enable_raw_mode()?;
        
        let mut stdout = io::stdout();
        execute!(stdout, cursor::Hide)?;


        // Initial draw
        self.draw()?;

        loop {
            // Read input events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                    match code {
                        KeyCode::Char(c) => {
                            // Calculate current box height for clearing
                            let box_height = self.lines.len() + 2; // +2 for top and bottom borders
                            
                            // Clear the current box display and move cursor to start
                            execute!(stdout, cursor::MoveUp(box_height as u16))?;
                            execute!(stdout, cursor::MoveToColumn(0))?;
                            execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                            
                            // Add character and redraw
                            self.add_char(c);
                            self.draw()?;
                        }
                        KeyCode::Backspace => {
                            // Calculate current box height for clearing
                            let box_height = self.lines.len() + 2; // +2 for top and bottom borders
                            
                            // Clear the current box display and move cursor to start
                            execute!(stdout, cursor::MoveUp(box_height as u16))?;
                            execute!(stdout, cursor::MoveToColumn(0))?;
                            execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                            
                            // Remove character and redraw
                            self.remove_char();
                            self.draw()?;
                        }
                        KeyCode::Enter => {
                            if !modifiers.contains(KeyModifiers::ALT) {
                                // Alt+Enter: Add new line within the box
                                let box_height = self.lines.len() + 2;
                                
                                execute!(stdout, cursor::MoveUp(box_height as u16))?;
                                execute!(stdout, cursor::MoveToColumn(0))?;
                                execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
                                
                                self.add_newline();
                                self.draw()?;
                            } else {
                                // Regular Enter: Create new box
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
    }

    // Cleanup
    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;

    let mut res = String::new();
    for line in self.lines.clone()
    {
        res.push_str(&line);
        res.push('\n');

    }
    
    Ok(res)


    }
}
