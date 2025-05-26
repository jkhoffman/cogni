//! Request builder example showing advanced client usage

use cogni_client::Client;
use cogni_core::{Function, Parameters, Tool};
use cogni_providers::openai::OpenAI;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    // Example 1: Basic request builder
    println!("=== Basic Request Builder ===");
    let response = client
        .request()
        .model("gpt-4o-mini")
        .system("You are a helpful coding assistant")
        .user("What is a closure in Rust?")
        .temperature(0.7)
        .max_tokens(200)
        .send()
        .await?;

    println!("Response: {}\n", response.content);

    // Example 2: Multi-turn conversation
    println!("=== Multi-turn Conversation ===");
    let response = client
        .request()
        .model("gpt-4o-mini")
        .system("You are a math tutor")
        .user("What is the pythagorean theorem?")
        .assistant("The Pythagorean theorem states that in a right triangle, the square of the hypotenuse (c) equals the sum of squares of the other two sides (a and b): a² + b² = c²")
        .user("Can you give me an example?")
        .temperature(0.5)
        .send()
        .await?;

    println!("Response: {}\n", response.content);

    // Example 3: With custom parameters
    println!("=== Custom Parameters ===");
    let params = Parameters {
        temperature: Some(0.9),
        max_tokens: Some(150),
        top_p: Some(0.95),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.5),
        ..Default::default()
    };

    let response = client
        .request()
        .model("gpt-4o-mini")
        .user("Write a creative haiku about programming")
        .parameters(params)
        .send()
        .await?;

    println!("Creative haiku:\n{}\n", response.content);

    // Example 4: With tools
    println!("=== With Tools ===");
    let word_count_tool = Tool {
        name: "count_words".to_string(),
        description: "Count the number of words in a text".to_string(),
        function: Function {
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text to count words in"
                    }
                },
                "required": ["text"]
            }),
            returns: Some("integer".to_string()),
        },
    };

    let response = client
        .request()
        .model("gpt-4o-mini")
        .user("How many words are in 'The quick brown fox jumps over the lazy dog'? Use the word count tool.")
        .tool(word_count_tool)
        .send()
        .await?;

    if !response.tool_calls.is_empty() {
        println!("Tool calls made:");
        for call in &response.tool_calls {
            println!("  - {} with args: {}", call.name, call.arguments);
        }
    }

    if !response.content.is_empty() {
        println!("Response: {}", response.content);
    }

    // Example 5: Building and reusing requests
    println!("\n=== Reusable Requests ===");
    let base_request = client
        .request()
        .model("gpt-4o-mini")
        .system("You are a helpful assistant. Always be concise.")
        .temperature(0.7)
        .build();

    // Use the base request with different user messages
    let questions = vec!["What is Rust?", "What is Python?", "What is JavaScript?"];

    for question in questions {
        let mut request = base_request.clone();
        request.messages.push(cogni_core::Message::user(question));

        let response = client.execute(request).await?;
        println!("Q: {}", question);
        println!("A: {}\n", response.content);
    }

    Ok(())
}
