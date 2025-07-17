use std::env;
use std::fs;
use std::path::Path;
use std::process;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, CONTENT_LENGTH};
use serde_json::{json, Value};
use tokio;

#[derive(Debug)]
struct AudioFile {
    path: String,
    mime_type: String,
    num_bytes: u64,
    display_name: String,
}

impl AudioFile {
    fn new(path: &str, display_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let metadata = fs::metadata(path)?;
        let num_bytes = metadata.len();
        
        // Get MIME type using file extension or default to audio/mpeg
        let mime_type = match Path::new(path).extension().and_then(|s| s.to_str()) {
            Some("mp3") => "audio/mpeg",
            Some("wav") => "audio/wav",
            Some("m4a") => "audio/mp4",
            Some("ogg") => "audio/ogg",
            Some("flac") => "audio/flac",
            _ => "audio/mpeg", // default
        }.to_string();
        
        Ok(AudioFile {
            path: path.to_string(),
            mime_type,
            num_bytes,
            display_name: display_name.to_string(),
        })
    }
}

async fn start_resumable_upload(
    client: &reqwest::Client,
    api_key: &str,
    audio_file: &AudioFile,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://generativelanguage.googleapis.com/upload/v1beta/files";
    
    let mut headers = HeaderMap::new();
    headers.insert("x-goog-api-key", HeaderValue::from_str(api_key)?);
    headers.insert("X-Goog-Upload-Protocol", HeaderValue::from_static("resumable"));
    headers.insert("X-Goog-Upload-Command", HeaderValue::from_static("start"));
    headers.insert("X-Goog-Upload-Header-Content-Length", HeaderValue::from_str(&audio_file.num_bytes.to_string())?);
    headers.insert("X-Goog-Upload-Header-Content-Type", HeaderValue::from_str(&audio_file.mime_type)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    
    let body = json!({
        "file": {
            "display_name": audio_file.display_name
        }
    });
    
    let response = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to start upload: {}", response.status()).into());
    }
    
    let upload_url = response
        .headers()
        .get("x-goog-upload-url")
        .ok_or("Upload URL not found in response headers")?
        .to_str()?
        .to_string();
    
    Ok(upload_url)
}

async fn upload_file_data(
    client: &reqwest::Client,
    upload_url: &str,
    audio_file: &AudioFile,
) -> Result<Value, Box<dyn std::error::Error>> {
    let file_data = fs::read(&audio_file.path)?;
    
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_str(&audio_file.num_bytes.to_string())?);
    headers.insert("X-Goog-Upload-Offset", HeaderValue::from_static("0"));
    headers.insert("X-Goog-Upload-Command", HeaderValue::from_static("upload, finalize"));
    
    let response = client
        .post(upload_url)
        .headers(headers)
        .body(file_data)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to upload file: {}", response.status()).into());
    }
    
    let file_info: Value = response.json().await?;
    Ok(file_info)
}

async fn generate_content(
    client: &reqwest::Client,
    api_key: &str,
    file_uri: &str,
    mime_type: &str,
    prompt: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";
    
    let mut headers = HeaderMap::new();
    headers.insert("x-goog-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    
    let body = json!({
        "contents": [{
            "parts": [
                {"text": prompt},
                {"file_data": {"mime_type": mime_type, "file_uri": file_uri}}
            ]
        }]
    });
    
    let response = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to generate content: {}", response.status()).into());
    }
    
    let content: Value = response.json().await?;
    Ok(content)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY environment variable not set")?;
    
    // Get audio file path from command line args or use default
    let args: Vec<String> = env::args().collect();
    let audio_path = if args.len() > 1 {
        &args[1]
    } else {
        "path/to/sample.mp3"
    };
    
    // Check if file exists
    if !Path::new(audio_path).exists() {
        eprintln!("Error: File '{}' not found", audio_path);
        process::exit(1);
    }
    
    let audio_file = AudioFile::new(audio_path, "AUDIO")?;
    let client = reqwest::Client::new();
    
    println!("Starting resumable upload for: {}", audio_file.path);
    println!("File size: {} bytes", audio_file.num_bytes);
    println!("MIME type: {}", audio_file.mime_type);
    
    // Step 1: Start resumable upload
    let upload_url = start_resumable_upload(&client, &api_key, &audio_file).await?;
    println!("Upload URL obtained");
    
    // Step 2: Upload the file data
    let file_info = upload_file_data(&client, &upload_url, &audio_file).await?;
    println!("File uploaded successfully");
    
    // Extract file URI
    let file_uri = file_info["file"]["uri"]
        .as_str()
        .ok_or("File URI not found in response")?;
    
    println!("file_uri={}", file_uri);
    
    // Step 3: Generate content using the uploaded file
    let prompt = "Describe this audio clip";
    let response = generate_content(&client, &api_key, file_uri, &audio_file.mime_type, prompt).await?;
    
    // Print full response
    println!("\nFull response:");
    println!("{}", serde_json::to_string_pretty(&response)?);
    
    // Extract and print just the text content
    if let Some(candidates) = response["candidates"].as_array() {
        for candidate in candidates {
            if let Some(parts) = candidate["content"]["parts"].as_array() {
                for part in parts {
                    if let Some(text) = part["text"].as_str() {
                        println!("\nGenerated text:");
                        println!("{}", text);
                    }
                }
            }
        }
    }
    
    Ok(())
}



// https://www.kaggle.com/datasets/pavanelisetty/sample-audio-files-for-speech-recognition?resource=download
// Starting resumable upload for: harvard.wav
// File size: 3249924 bytes
// MIME type: audio/wav
// Upload URL obtained
// File uploaded successfully
// file_uri=https://generativelanguage.googleapis.com/v1beta/files/bpfqazeqscck

// Full response:
// {
//   "candidates": [
//     {
//       "content": {
//         "parts": [
//           {
//             "text": "The audio clip features a male voice speaking a series of short, unrelated phrases, such as \"The stale smell of old beer lingers,\" \"A cold dip restores health and zest,\" and \"Tacos al pastor are my favorite.\" The speech is clear and articulate. At the very end of the clip, there is a distinct, sustained high-pitched tone."
//           }
//         ],
//         "role": "model"
//       },
//       "finishReason": "STOP",
//       "index": 0
//     }
//   ],
//   "modelVersion": "gemini-2.5-flash",
//   "responseId": "8A55aNHxENHVjrEP3OvduQo",
//   "usageMetadata": {
//     "candidatesTokenCount": 72,
//     "promptTokenCount": 592,
//     "promptTokensDetails": [
//       {
//         "modality": "TEXT",
//         "tokenCount": 4
//       },
//       {
//         "modality": "AUDIO",
//         "tokenCount": 588
//       }
//     ],
//     "thoughtsTokenCount": 288,
//     "totalTokenCount": 952
//   }
// }

// Generated text:
// The audio clip features a male voice speaking a series of short, unrelated phrases, such as "The stale smell of old beer lingers," "A cold dip restores health and zest," and "Tacos al pastor are my favorite." The speech is clear and articulate. At the very end of the clip, there is a distinct, sustained high-pitched tone.