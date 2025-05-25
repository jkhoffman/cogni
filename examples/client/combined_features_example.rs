//! Example combining multiple advanced features

use cogni_client::{Client, RequestBuilder, parallel_requests};
use cogni_providers::{openai::OpenAI, anthropic::Anthropic};
use cogni_core::{Message, Tool};
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Multi-provider comparison
    println!("=== Multi-Provider Comparison ===");
    if let (Ok(openai_key), Ok(anthropic_key)) = (
        env::var("OPENAI_API_KEY"),
        env::var("ANTHROPIC_API_KEY"),
    ) {
        compare_providers(&openai_key, &anthropic_key).await?;
    } else {
        println!("Skipping multi-provider example - both API keys required");
    }

    // Example 2: Complex request building
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("\n=== Complex Request Building ===");
        complex_request_example(&api_key).await?;
    }

    // Example 3: Streaming with state management
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("\n=== Streaming with State ===");
        streaming_with_state(&api_key).await?;
    }

    Ok(())
}

/// Compare responses from different providers
async fn compare_providers(
    openai_key: &str,
    anthropic_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = "Explain quantum entanglement in one sentence.";
    
    // Create providers
    let openai = OpenAI::with_api_key(openai_key.to_string());
    let anthropic = Anthropic::with_api_key(anthropic_key.to_string());
    
    // Build the same request for both
    let request = RequestBuilder::new()
        .system("You are a physics teacher. Be concise.")
        .user(query)
        .temperature(0.7)
        .max_tokens(100)
        .build();
    
    println!("Query: {}", query);
    println!("\nResponses:");
    
    // Execute requests in parallel using tokio::join!
    let (openai_result, anthropic_result) = tokio::join!(
        openai.request(request.clone()),
        anthropic.request(request)
    );
    
    let openai_response = openai_result?;
    let anthropic_response = anthropic_result?;
    
    println!("OpenAI: {}", openai_response.content.trim());
    println!("Anthropic: {}", anthropic_response.content.trim());
    
    Ok(())
}

/// Demonstrate complex request building with tools and parameters
async fn complex_request_example(api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let provider = OpenAI::with_api_key(api_key.to_string());
    let client = Client::new(provider);
    
    // Define a weather tool
    let weather_tool = Tool {
        name: "get_weather".to_string(),
        description: "Get the current weather for a location".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit"
                }
            },
            "required": ["location"]
        }),
    };
    
    // Build a complex request
    let response = client
        .request()
        .model("gpt-4o-mini")
        .system("You are a helpful weather assistant.")
        .user("What's the weather like in Paris, France?")
        .tool(weather_tool)
        .temperature(0.3)
        .max_tokens(200)
        .send()
        .await?;
    
    if !response.tool_calls.is_empty() {
        println!("Tool calls made:");
        for call in &response.tool_calls {
            println!("  - {} with args: {}", call.name, call.arguments);
        }
    } else {
        println!("Response: {}", response.content);
    }
    
    Ok(())
}

/// Demonstrate streaming with state management
async fn streaming_with_state(api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let provider = OpenAI::with_api_key(api_key.to_string());
    let client = Client::new(provider).with_model("gpt-4o-mini");
    
    // State to track streaming
    #[derive(Default)]
    struct StreamState {
        tokens: usize,
        words: usize,
        start_time: Option<std::time::Instant>,
    }
    
    let mut state = StreamState::default();
    
    println!("Streaming response with state tracking:");
    
    let mut stream = client
        .stream_chat("Tell me a short story about a robot (2-3 sentences)")
        .await?;
    
    state.start_time = Some(std::time::Instant::now());
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => {
                print!("{}", text);
                state.tokens += 1;
                state.words += text.split_whitespace().count();
            }
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }
    
    let elapsed = state.start_time.unwrap().elapsed();
    println!("\n\nStreaming stats:");
    println!("  Tokens received: {}", state.tokens);
    println!("  Words: {}", state.words);
    println!("  Time: {:?}", elapsed);
    println!("  Tokens/sec: {:.2}", state.tokens as f64 / elapsed.as_secs_f64());
    
    Ok(())
}