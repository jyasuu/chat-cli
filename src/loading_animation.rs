use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};

/// Docker-style loading animation with progress indicators
pub struct LoadingAnimation {
    message: String,
    is_running: Arc<AtomicBool>,
    style: AnimationStyle,
}

#[derive(Clone)]
pub enum AnimationStyle {
    Spinner,
    Dots,
    Progress,
    Docker,
}

impl LoadingAnimation {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            is_running: Arc::new(AtomicBool::new(false)),
            style: AnimationStyle::Docker,
        }
    }

    pub fn with_style(mut self, style: AnimationStyle) -> Self {
        self.style = style;
        self
    }

    /// Start the loading animation
    pub fn start(&self) -> LoadingHandle {
        self.is_running.store(true, Ordering::Relaxed);
        let is_running = Arc::clone(&self.is_running);
        let message = self.message.clone();
        let style = self.style.clone();

        let handle = tokio::spawn(async move {
            let mut frame = 0;
            
            while is_running.load(Ordering::Relaxed) {
                // Save cursor position and hide cursor
                execute!(io::stdout(), cursor::SavePosition, cursor::Hide).ok();
                
                match style {
                    AnimationStyle::Spinner => {
                        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                        let spinner_char = spinner_chars[frame % spinner_chars.len()];
                        print!("\r{} {} {}", 
                            SetForegroundColor(Color::Cyan),
                            spinner_char,
                            message
                        );
                    }
                    AnimationStyle::Dots => {
                        let dots = ".".repeat((frame % 4) + 1);
                        print!("\r{} {}{}{}", 
                            SetForegroundColor(Color::Yellow),
                            message,
                            dots,
                            " ".repeat(4 - dots.len())
                        );
                    }
                    AnimationStyle::Progress => {
                        let progress = frame % 20;
                        let filled = "█".repeat(progress);
                        let empty = "░".repeat(20 - progress);
                        print!("\r{} {} [{}{}]", 
                            SetForegroundColor(Color::Green),
                            message,
                            filled,
                            empty
                        );
                    }
                    AnimationStyle::Docker => {
                        let docker_chars = ['◐', '◓', '◑', '◒'];
                        let docker_char = docker_chars[frame % docker_chars.len()];
                        let elapsed = frame / 4; // Rough seconds
                        print!("\r{} {} {} {}s", 
                            SetForegroundColor(Color::Blue),
                            docker_char,
                            message,
                            elapsed
                        );
                    }
                }
                
                print!("{}", ResetColor);
                io::stdout().flush().ok();
                
                // Restore cursor position and show cursor
                execute!(io::stdout(), cursor::RestorePosition, cursor::Show).ok();
                
                tokio::time::sleep(Duration::from_millis(100)).await;
                frame += 1;
            }
            
            // Clear the loading line
            print!("\r{}", " ".repeat(80));
            print!("\r");
            io::stdout().flush().ok();
        });

        LoadingHandle {
            handle,
            is_running: Arc::clone(&self.is_running),
        }
    }
}

/// Handle for controlling the loading animation
pub struct LoadingHandle {
    handle: tokio::task::JoinHandle<()>,
    is_running: Arc<AtomicBool>,
}

impl LoadingHandle {
    /// Stop the loading animation
    pub async fn stop(self) {
        self.is_running.store(false, Ordering::Relaxed);
        let _ = self.handle.await;
    }

    /// Update the loading message (for future enhancement)
    pub fn update_message(&self, _message: &str) {
        // Could be implemented to update the message dynamically
    }
}

/// Convenience function for showing a simple loading spinner
pub async fn show_loading<F, T>(message: &str, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let loading = LoadingAnimation::new(message);
    let handle = loading.start();
    
    let result = future.await;
    
    handle.stop().await;
    result
}

/// Docker-style progress steps
pub struct ProgressSteps {
    steps: Vec<String>,
    current_step: usize,
}

impl ProgressSteps {
    pub fn new(steps: Vec<String>) -> Self {
        Self {
            steps,
            current_step: 0,
        }
    }

    pub fn start_step(&mut self, step_index: usize) -> io::Result<()> {
        if step_index < self.steps.len() {
            self.current_step = step_index;
            execute!(
                io::stdout(),
                Print(format!("{} Step {}/{}: {}\n", 
                    SetForegroundColor(Color::Blue),
                    step_index + 1,
                    self.steps.len(),
                    self.steps[step_index]
                )),
                ResetColor
            )?;
        }
        Ok(())
    }

    pub fn complete_step(&self, step_index: usize) -> io::Result<()> {
        if step_index < self.steps.len() {
            execute!(
                io::stdout(),
                Print(format!("{} ✓ Step {}/{}: {} - DONE\n", 
                    SetForegroundColor(Color::Green),
                    step_index + 1,
                    self.steps.len(),
                    self.steps[step_index]
                )),
                ResetColor
            )?;
        }
        Ok(())
    }

    pub fn fail_step(&self, step_index: usize, error: &str) -> io::Result<()> {
        if step_index < self.steps.len() {
            execute!(
                io::stdout(),
                Print(format!("{} ✗ Step {}/{}: {} - FAILED: {}\n", 
                    SetForegroundColor(Color::Red),
                    step_index + 1,
                    self.steps.len(),
                    self.steps[step_index],
                    error
                )),
                ResetColor
            )?;
        }
        Ok(())
    }
}