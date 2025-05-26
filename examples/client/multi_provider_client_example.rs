//! Example showing how to use the client with different providers

use cogni_client::Client;
use cogni_providers::{anthropic::Anthropic, openai::OpenAI};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Using with OpenAI
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("=== OpenAI Client ===");
        let provider = OpenAI::with_api_key(api_key);
        let client = Client::new(provider).with_model("gpt-4o-mini");

        let response = client
            .chat("What are the main features of Rust in one sentence?")
            .await?;

        println!("OpenAI says: {}\n", response);
    }

    // Example 2: Using with Anthropic
    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
        println!("=== Anthropic Client ===");
        let provider = Anthropic::with_api_key(api_key);
        let client = Client::new(provider).with_model("claude-3-haiku-20240307");

        let response = client
            .chat("What are the main features of Rust in one sentence?")
            .await?;

        println!("Claude says: {}\n", response);
    }

    // Example 3: Using with custom default parameters
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("=== Client with Custom Defaults ===");
        let provider = OpenAI::with_api_key(api_key);

        let mut default_params = cogni_core::Parameters::default();
        default_params.temperature = Some(0.3);
        default_params.max_tokens = Some(100);

        let client = Client::new(provider)
            .with_model("gpt-4o-mini")
            .with_parameters(default_params);

        // All requests will use these defaults
        let response1 = client.chat("Define 'algorithm' in one sentence.").await?;
        let response2 = client
            .chat("Define 'data structure' in one sentence.")
            .await?;

        println!("Algorithm: {}", response1);
        println!("Data structure: {}", response2);
    }

    // Example 4: Provider-agnostic function
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("\n=== Provider-Agnostic Function ===");
        let provider = OpenAI::with_api_key(api_key);
        let result = analyze_sentiment(provider, "I love programming in Rust!").await?;
        println!("Sentiment: {}", result);
    }

    Ok(())
}

/// Example of a provider-agnostic function
async fn analyze_sentiment<P: cogni_core::Provider>(
    provider: P,
    text: &str,
) -> Result<String, cogni_core::Error> {
    let client = Client::new(provider);

    client
        .request()
        .model("gpt-4o-mini") // You might want to make this configurable
        .system("You are a sentiment analyzer. Respond with only: POSITIVE, NEGATIVE, or NEUTRAL")
        .user(format!("Analyze the sentiment of: {}", text))
        .temperature(0.0)
        .max_tokens(10)
        .send()
        .await
        .map(|response| response.content.trim().to_string())
}
