use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::Duration,
    fs::OpenOptions,
    io::Write as IoWrite,
};
use tokio::sync::Mutex;

mod gemini;
mod ui;

use gemini::GeminiClient;
use ui::{App, InputMode, Message, MessageType};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Gemini API key
    #[arg(short, long, env = "GEMINI_API_KEY")]
    api_key: String,
    
    /// Model to use (default: gemini-2.5-flash-lite-preview-06-17)
    #[arg(short, long, default_value = "gemini-2.5-flash-lite-preview-06-17")]
    model: String,
}

type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let gemini_client = GeminiClient::new(args.api_key, args.model);
    let app = Arc::new(Mutex::new(App::new(gemini_client)));

    // Run the app
    let result = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

async fn run_app(terminal: &mut AppTerminal, app: Arc<Mutex<App>>) -> Result<()> {
    loop {
        // Draw UI
        {
            let app_guard = app.lock().await;
            terminal.draw(|f| ui(&mut *f, &app_guard))?;
        }

        // Handle events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut app_guard = app.lock().await;
                match app_guard.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('i') => {
                            app_guard.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('c') => {
                            app_guard.messages.clear();
                        }
                        KeyCode::Up => {
                            if app_guard.scroll_offset > 0 {
                                app_guard.scroll_offset -= 1;
                            }
                        }
                        KeyCode::Down => {
                            app_guard.scroll_offset += 1;
                        }
                        _ => {}
                    }
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let input = app_guard.input.clone();
                            if !input.trim().is_empty() {
                                app_guard.messages.push(Message {
                                    content: input.clone(),
                                    message_type: MessageType::User,
                                    timestamp: chrono::Utc::now(),
                                });
                                app_guard.input.clear();
                                app_guard.input_mode = InputMode::Normal;
                                app_guard.is_loading = true;
                                
                                // Add logging function
                                let log_debug = |msg: &str| {
                                    if let Ok(mut file) = OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open("tmp_rovodev_streaming_debug.log") 
                                    {
                                        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                                        let _ = writeln!(file, "[UI] [{}] {}", timestamp, msg);
                                    }
                                };
                                
                                log_debug(&format!("Starting to send message: {:?}", input));
                                
                                // Send message to Gemini with streaming
                                let client = app_guard.gemini_client.clone();
                                let app_clone = Arc::clone(&app);
                                tokio::spawn(async move {
                                    log_debug("Calling send_message_stream");
                                    match client.send_message_stream(&input).await {
                                        Ok(mut rx) => {
                                            log_debug("Successfully got receiver from send_message_stream");
                                            log_debug("Receiver channel capacity and status check...");
                                            let mut accumulated_response = String::new();
                                            let mut received_any_data = false;
                                            let mut chunk_count = 0;
                                            
                                            // Remove the delay that causes race condition
                                            log_debug("Starting to receive immediately...");
                                            
                                            log_debug("Starting to receive chunks from channel...");
                                            log_debug(&format!("Channel receiver created, starting loop..."));
                                            
                                            // Use a while let loop instead of manual loop with timeout
                                            while let Some(chunk) = rx.recv().await {
                                                chunk_count += 1;
                                                log_debug(&format!("MAIN: Received chunk #{}: {:?} (length: {})", chunk_count, chunk, chunk.len()));
                                                
                                                // Check if this is an error message
                                                if chunk.starts_with("JSON parse error:") || chunk.starts_with("Stream error:") {
                                                    log_debug(&format!("Received error chunk: {:?}", chunk));
                                                    let mut app_guard = app_clone.lock().await;
                                                    app_guard.streaming_message = None;
                                                    app_guard.messages.push(Message {
                                                        content: format!("Streaming error: {}. Falling back to regular API.", chunk),
                                                        message_type: MessageType::Error,
                                                        timestamp: chrono::Utc::now(),
                                                    });
                                                    app_guard.is_loading = false;
                                                    break;
                                                }
                                                
                                                // Accumulate all non-error chunks
                                                received_any_data = true;
                                                accumulated_response.push_str(&chunk);
                                                log_debug(&format!("Accumulated response now: {:?}", accumulated_response));
                                                
                                                // Update streaming message in real-time
                                                {
                                                    let mut app_guard = app_clone.lock().await;
                                                    app_guard.streaming_message = Some(accumulated_response.clone());
                                                    log_debug("Updated streaming_message in app state");
                                                }
                                            }
                                            
                                            log_debug("Channel closed - no more chunks to receive");
                                            log_debug(&format!("Finished receiving chunks. received_any_data: {}, accumulated_response: {:?}", received_any_data, accumulated_response));
                                            
                                            // Finalize the message if we received data
                                            if received_any_data && !accumulated_response.trim().is_empty() {
                                                log_debug("Finalizing message with accumulated response");
                                                let mut app_guard = app_clone.lock().await;
                                                app_guard.streaming_message = None;
                                                app_guard.messages.push(Message {
                                                    content: accumulated_response,
                                                    message_type: MessageType::Assistant,
                                                    timestamp: chrono::Utc::now(),
                                                });
                                                app_guard.is_loading = false;
                                            } else {
                                                log_debug("No data received, falling back to regular API");
                                                // Fallback to regular API if streaming failed
                                                match client.send_message(&input).await {
                                                    Ok(response) => {
                                                        let mut app_guard = app_clone.lock().await;
                                                        app_guard.streaming_message = None;
                                                        app_guard.messages.push(Message {
                                                            content: response,
                                                            message_type: MessageType::Assistant,
                                                            timestamp: chrono::Utc::now(),
                                                        });
                                                        app_guard.is_loading = false;
                                                    }
                                                    Err(fallback_e) => {
                                                        let mut app_guard = app_clone.lock().await;
                                                        app_guard.streaming_message = None;
                                                        app_guard.messages.push(Message {
                                                            content: format!("Fallback API error: {}", fallback_e),
                                                            message_type: MessageType::Error,
                                                            timestamp: chrono::Utc::now(),
                                                        });
                                                        app_guard.is_loading = false;
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log_debug(&format!("send_message_stream failed with error: {}", e));
                                            // Try fallback to regular API
                                            match client.send_message(&input).await {
                                                Ok(response) => {
                                                    let mut app_guard = app_clone.lock().await;
                                                    app_guard.streaming_message = None;
                                                    app_guard.messages.push(Message {
                                                        content: response,
                                                        message_type: MessageType::Assistant,
                                                        timestamp: chrono::Utc::now(),
                                                    });
                                                    app_guard.is_loading = false;
                                                }
                                                Err(fallback_e) => {
                                                    let mut app_guard = app_clone.lock().await;
                                                    app_guard.streaming_message = None;
                                                    app_guard.messages.push(Message {
                                                        content: format!("Streaming failed: {}. Fallback failed: {}", e, fallback_e),
                                                        message_type: MessageType::Error,
                                                        timestamp: chrono::Utc::now(),
                                                    });
                                                    app_guard.is_loading = false;
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        KeyCode::Char(c) => {
                            app_guard.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app_guard.input.pop();
                        }
                        KeyCode::Esc => {
                            app_guard.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title bar
    let title = Paragraph::new("Gemini Chat CLI")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Chat messages
    let mut all_messages = app.messages.clone();
    
    // Add streaming message if present
    if let Some(ref streaming_content) = app.streaming_message {
        all_messages.push(Message {
            content: format!("{} |", streaming_content), // Add blinking cursor
            message_type: MessageType::Assistant,
            timestamp: chrono::Utc::now(),
        });
    }
    
    let messages: Vec<ListItem> = all_messages
        .iter()
        .skip(app.scroll_offset)
        .map(|m| {
            let timestamp = m.timestamp.format("%H:%M:%S").to_string();
            let (prefix, style) = match m.message_type {
                MessageType::User => ("You", Style::default().fg(Color::Green)),
                MessageType::Assistant => ("Gemini", Style::default().fg(Color::Blue)),
                MessageType::Error => ("Error", Style::default().fg(Color::Red)),
            };
            
            let wrapped_text = textwrap::fill(&m.content, chunks[1].width.saturating_sub(4) as usize);
            let lines: Vec<Line> = wrapped_text
                .split('\n')
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(format!("[{}] {}: ", timestamp, prefix), style.add_modifier(Modifier::BOLD)),
                            Span::raw(format!("{line}")),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw(" ".repeat(timestamp.len() + prefix.len() + 4)),
                            Span::raw(format!("{line}")),
                        ])
                    }
                })
                .collect();
            
            ListItem::new(lines)
        })
        .collect();

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    f.render_widget(messages_list, chunks[1]);

    // Input box
    let input_style = match app.input_mode {
        InputMode::Normal => Style::default(),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };
    
    let input_text = if app.is_loading {
        if app.streaming_message.is_some() {
            "Streaming response in real-time...".to_string()
        } else {
            "Waiting for response...".to_string()
        }
    } else {
        app.input.clone()
    };
    
    let input = Paragraph::new(input_text)
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title(match app.input_mode {
            InputMode::Normal => "Input (press 'i' to edit, 'q' to quit, 'c' to clear)",
            InputMode::Editing => "Input (press Esc to stop editing, Enter to send)",
        }))
        .wrap(Wrap { trim: true });
    f.render_widget(input, chunks[2]);

    // Set cursor position
    if app.input_mode == InputMode::Editing && !app.is_loading {
        f.set_cursor(
            chunks[2].x + app.input.len() as u16 + 1,
            chunks[2].y + 1,
        );
    }
}