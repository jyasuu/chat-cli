use std::env;
use std::fs;
use reqwest::Client;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable not set");

    // Read JSON payload from file
    let json_data = fs::read_to_string("case1.json")
        .expect("Failed to read case1.json");

    // Construct API URL
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite-preview-06-17:streamGenerateContent?alt=sse&key={}",
        api_key
    );

    // Create HTTP client
    let client = Client::new();
    
    // Send POST request with streaming response
    let mut response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(json_data)
        .send()
        .await?;

    // Stream response chunks to stdout
    let mut stdout = tokio::io::stdout();
    while let Some(chunk) = response.chunk().await? {
        println!("{}",String::from_utf8(chunk.to_vec()).unwrap());
        stdout.write_all(&chunk).await?;
    }

    Ok(())
}

// data: {"candidates": [{"content": {"parts": [{"text": "AI learns"}],"role": "model"},"index": 0}],"usageMetadata": {"promptTokenCount": 8,"candidatesTokenCount": 2,"totalTokenCount": 10,"promptTokensDetails": [{"modality": "TEXT","tokenCount": 8}]},"modelVersion": "gemini-2.5-flash-lite-preview-06-17","responseId": "znFzaPOHN6aTjMcP5u2r4AI"}


// data: {"candidates": [{"content": {"parts": [{"text": "AI learns"}],"role": "model"},"index": 0}],"usageMetadata": {"promptTokenCount": 8,"candidatesTokenCount": 2,"totalTokenCount": 10,"promptTokensDetails": [{"modality": "TEXT","tokenCount": 8}]},"modelVersion": "gemini-2.5-flash-lite-preview-06-17","responseId": "znFzaPOHN6aTjMcP5u2r4AI"}

// data: {"candidates": [{"content": {"parts": [{"text": " from data to make decisions or predictions."}],"role": "model"},"finishReason": "STOP","index": 0}],"usageMetadata": {"promptTokenCount": 8,"candidatesTokenCount": 10,"totalTokenCount": 18,"promptTokensDetails": [{"modality": "TEXT","tokenCount": 8}]},"modelVersion": "gemini-2.5-flash-lite-preview-06-17","responseId": "znFzaPOHN6aTjMcP5u2r4AI"}


// data: {"candidates": [{"content": {"parts": [{"text": " from data to make decisions or predictions."}],"role": "model"},"finishReason": "STOP","index": 0}],"usageMetadata": {"promptTokenCount": 8,"candidatesTokenCount": 10,"totalTokenCount": 18,"promptTokensDetails": [{"modality": "TEXT","tokenCount": 8}]},"modelVersion": "gemini-2.5-flash-lite-preview-06-17","responseId": "znFzaPOHN6aTjMcP5u2r4AI"}