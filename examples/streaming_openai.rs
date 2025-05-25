//! Example of streaming responses from OpenAI

use cogni::prelude::*;
use cogni::providers::OpenAI;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), cogni::Error> {
    // Get API key from environment
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("Please set OPENAI_API_KEY environment variable");
    
    // Create provider
    let provider = OpenAI::with_api_key(api_key);
    
    // Create a request
    let request = Request::builder()
        .message(Message::user("Write a haiku about Rust programming"))
        .model("gpt-3.5-turbo")
        .temperature(0.9)
        .build();
    
    println!("Streaming response from OpenAI...\n");
    
    // Get streaming response
    let mut stream = provider.stream(request).await?;
    
    // Process stream events
    let mut accumulator = StreamAccumulator::new();
    
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Content(delta) => {
                print!("{}", delta.text);
                accumulator.process_event(StreamEvent::Content(delta))?;
            }
            StreamEvent::Done => {
                println!("\n\nStream completed!");
                break;
            }
            _ => {}
        }
    }
    
    println!("\nFull response: {}", accumulator.content());
    
    Ok(())
}