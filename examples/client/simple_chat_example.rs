//! Simple chat example using the high-level client API

use cogni_client::Client;
use cogni_providers::openai::OpenAI;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let provider = OpenAI::with_api_key(api_key);

    // Create client with default model
    let client = Client::new(provider).with_model("gpt-4o-mini");

    // Simple one-line chat
    println!("=== Simple Chat ===");
    let response = client.chat("Hello! How are you today?").await?;
    println!("Response: {}", response);

    // Chat with conversation history
    println!("\n=== Conversation ===");
    let messages = vec![
        cogni_core::Message::system("You are a helpful assistant who speaks concisely"),
        cogni_core::Message::user("What is Rust?"),
        cogni_core::Message::assistant("Rust is a systems programming language focused on safety, speed, and concurrency."),
        cogni_core::Message::user("What makes it special?"),
    ];

    let response = client.chat(messages).await?;
    println!("Response: {}", response);

    Ok(())
}