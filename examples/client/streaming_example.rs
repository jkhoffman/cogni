//! Streaming example using the high-level client API
//!
//! This example demonstrates the simple streaming API that automatically
//! handles text content and properly terminates when the stream is complete.

use cogni_client::Client;
use cogni_providers::anthropic::Anthropic;
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let provider = Anthropic::with_api_key(api_key)?;

    // Create client with a specific model
    let client = Client::new(provider).with_model("claude-3-haiku-20240307");

    // Stream a response
    println!("=== Streaming Response ===");
    println!("Generating a story...\n");

    // Use the simple stream_chat API which returns a stream of text chunks
    let mut stream = client
        .stream_chat("Tell me a very short story about a robot learning to paint")
        .await?;

    // Process the stream chunk by chunk
    // The stream will automatically complete when Done event is received
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => print!("{}", text),
            Err(e) => {
                eprintln!("\nError: {}", e);
                break; // Exit on error
            }
        }
    }

    println!("\n\nDone!");

    Ok(())
}
