//! Example demonstrating provider failover and switching

use cogni::providers::{Anthropic, Ollama, OpenAI};
use cogni::{Error, Message, Provider, Request};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Provider Failover Example\n");

    let request = Request::builder()
        .message(Message::user("What is 2 + 2?"))
        .max_tokens(50)
        .build();

    // Try Ollama first (local, free)
    println!("Attempting with Ollama (local)...");
    let ollama = Ollama::local()?;
    match ollama.request(request.clone()).await {
        Ok(response) => {
            println!("✓ Success with Ollama: {}\n", response.content);
            return Ok(());
        }
        Err(e) => {
            println!("✗ Failed: {} (Is Ollama running locally?)\n", e);
        }
    }

    // Try OpenAI as fallback
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("Attempting with OpenAI...");
        let openai = OpenAI::with_api_key(api_key)?;
        match openai.request(request.clone()).await {
            Ok(response) => {
                println!("✓ Success with OpenAI: {}\n", response.content);
                return Ok(());
            }
            Err(e) => {
                println!("✗ Failed: {}\n", e);
            }
        }
    } else {
        println!("Skipping OpenAI (no API key)\n");
    }

    // Try Anthropic as final fallback
    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
        println!("Attempting with Anthropic...");
        let anthropic = Anthropic::with_api_key(api_key)?;
        match anthropic.request(request.clone()).await {
            Ok(response) => {
                println!("✓ Success with Anthropic: {}\n", response.content);
                return Ok(());
            }
            Err(e) => {
                println!("✗ Failed: {}\n", e);
            }
        }
    } else {
        println!("Skipping Anthropic (no API key)\n");
    }

    println!("All providers failed!");

    // Demonstrate load balancing across providers
    println!("\n---\nLoad Balancing Example\n");

    let openai_key = env::var("OPENAI_API_KEY").ok();
    let anthropic_key = env::var("ANTHROPIC_API_KEY").ok();

    if openai_key.is_none() && anthropic_key.is_none() {
        println!("Skipping load balancing example (no API keys set)");
        return Ok(());
    }

    // Simple round-robin load balancing
    for i in 0..4 {
        let req = Request::builder()
            .message(Message::user(format!("Say 'Response {}'", i + 1)))
            .max_tokens(20)
            .build();

        // Alternate between providers
        if i % 2 == 0 {
            if let Some(ref key) = openai_key {
                println!("Request {} -> OpenAI", i + 1);
                let openai = OpenAI::with_api_key(key.clone())?;
                match openai.request(req).await {
                    Ok(response) => println!("Response: {}\n", response.content.trim()),
                    Err(e) => println!("Error: {}\n", e),
                }
            } else if let Some(ref key) = anthropic_key {
                println!("Request {} -> Anthropic", i + 1);
                let anthropic = Anthropic::with_api_key(key.clone())?;
                match anthropic.request(req).await {
                    Ok(response) => println!("Response: {}\n", response.content.trim()),
                    Err(e) => println!("Error: {}\n", e),
                }
            }
        } else if let Some(ref key) = anthropic_key {
            println!("Request {} -> Anthropic", i + 1);
            let anthropic = Anthropic::with_api_key(key.clone())?;
            match anthropic.request(req).await {
                Ok(response) => println!("Response: {}\n", response.content.trim()),
                Err(e) => println!("Error: {}\n", e),
            }
        } else if let Some(ref key) = openai_key {
            println!("Request {} -> OpenAI", i + 1);
            let openai = OpenAI::with_api_key(key.clone())?;
            match openai.request(req).await {
                Ok(response) => println!("Response: {}\n", response.content.trim()),
                Err(e) => println!("Error: {}\n", e),
            }
        }
    }

    Ok(())
}
