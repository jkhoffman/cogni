//! Basic example of using the OpenAI provider

use cogni::prelude::*;
use cogni::providers::OpenAI;

#[tokio::main]
async fn main() -> Result<(), cogni::Error> {
    // Get API key from environment
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("Please set OPENAI_API_KEY environment variable");

    // Create provider
    let provider = OpenAI::with_api_key(api_key);

    // Create a simple request
    let request = Request::builder()
        .message(Message::system("You are a helpful assistant."))
        .message(Message::user("What is the capital of France?"))
        .model("gpt-3.5-turbo")
        .temperature(0.7)
        .build();

    println!("Sending request to OpenAI...");

    // Get response
    let response = provider.request(request).await?;

    println!("Response: {}", response.content);

    if let Some(usage) = response.metadata.usage {
        println!(
            "Tokens used: {} (prompt: {}, completion: {})",
            usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
        );
    }

    Ok(())
}
