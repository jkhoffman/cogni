//! Example demonstrating use of multiple LLM providers

use cogni::{Provider, Request, Message, Error};
use cogni::providers::{OpenAI, Anthropic, Ollama};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize providers
    let openai = OpenAI::with_api_key(
        env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable not set")
    );
    
    let anthropic = Anthropic::with_api_key(
        env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable not set")
    );
    
    let ollama = Ollama::local(); // Assumes Ollama is running locally
    
    // Create a simple request
    let request = Request::builder()
        .message(Message::system(
            "You are a helpful assistant. Keep your responses brief."
        ))
        .message(Message::user(
            "What is the capital of France? Answer in one word."
        ))
        .max_tokens(100)
        .temperature(0.7)
        .build();
    
    println!("Testing multiple LLM providers with the same request:\n");
    
    // Test OpenAI
    println!("OpenAI Response:");
    match openai.request(request.clone()).await {
        Ok(response) => println!("{}\n", response.content),
        Err(e) => println!("Error: {}\n", e),
    }
    
    // Test Anthropic
    println!("Anthropic Response:");
    match anthropic.request(request.clone()).await {
        Ok(response) => println!("{}\n", response.content),
        Err(e) => println!("Error: {}\n", e),
    }
    
    // Test Ollama (if running)
    println!("Ollama Response:");
    match ollama.request(request.clone()).await {
        Ok(response) => println!("{}\n", response.content),
        Err(e) => println!("Error: {} (Is Ollama running locally?)\n", e),
    }
    
    // Demonstrate streaming with all providers
    println!("\nStreaming example with all providers:");
    
    let streaming_request = Request::builder()
        .message(Message::user(
            "Count from 1 to 5 slowly, one number per line."
        ))
        .max_tokens(50)
        .build();
    
    // Stream from OpenAI
    println!("\nOpenAI (streaming):");
    match stream_provider(&openai, streaming_request.clone()).await {
        Ok(_) => println!(),
        Err(e) => println!("Error: {}", e),
    }
    
    // Stream from Anthropic
    println!("\nAnthropic (streaming):");
    match stream_provider(&anthropic, streaming_request.clone()).await {
        Ok(_) => println!(),
        Err(e) => println!("Error: {}", e),
    }
    
    // Stream from Ollama
    println!("\nOllama (streaming):");
    match stream_provider(&ollama, streaming_request.clone()).await {
        Ok(_) => println!(),
        Err(e) => println!("Error: {} (Is Ollama running locally?)", e),
    }
    
    Ok(())
}

async fn stream_provider<P: Provider>(
    provider: &P,
    request: Request,
) -> Result<(), Error> {
    use futures::StreamExt;
    use cogni::StreamEvent;
    
    let mut stream = provider.stream(request).await?;
    
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Content(delta) => print!("{}", delta.text),
            StreamEvent::Done => break,
            _ => {} // Ignore other events for this example
        }
    }
    
    Ok(())
}