//! Streaming example using the high-level client API

use cogni_client::Client;
use cogni_providers::anthropic::Anthropic;
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let provider = Anthropic::with_api_key(api_key);

    // Create client
    let client = Client::new(provider).with_model("claude-3-haiku-20240307");

    // Stream a response
    println!("=== Streaming Response ===");
    println!("Generating a story...\n");

    let mut stream = client
        .stream_chat("Tell me a very short story about a robot learning to paint")
        .await?;

    // Process the stream chunk by chunk
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => print!("{}", text),
            Err(e) => eprintln!("\nError: {}", e),
        }
    }
    println!("\n\nDone!");

    Ok(())
}