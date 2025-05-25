//! Integration tests for the high-level client API

use cogni_client::{Client, RequestBuilder};
use cogni_core::{Message, Role, Tool};
use cogni_providers::{anthropic::Anthropic, openai::OpenAI};
use futures::StreamExt;
use std::env;

/// Helper to check if we have API keys for testing
fn has_api_keys() -> bool {
    env::var("OPENAI_API_KEY").is_ok() || env::var("ANTHROPIC_API_KEY").is_ok()
}

#[tokio::test]
async fn test_client_simple_chat_openai() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI client test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider).with_model("gpt-4o-mini");

    let response = client.chat("Say 'Hello from Cogni client!'").await.unwrap();
    assert!(response.to_lowercase().contains("hello") || response.to_lowercase().contains("cogni"));
}

#[tokio::test]
async fn test_client_simple_chat_anthropic() {
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping Anthropic client test - ANTHROPIC_API_KEY not set");
            return;
        }
    };

    let provider = Anthropic::with_api_key(api_key);
    let client = Client::new(provider).with_model("claude-3-haiku-20240307");

    let response = client.chat("Say 'Hello from Cogni client!'").await.unwrap();
    assert!(response.to_lowercase().contains("hello") || response.to_lowercase().contains("cogni"));
}

#[tokio::test]
async fn test_client_streaming_openai() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI streaming test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider).with_model("gpt-4o-mini");

    let mut stream = client
        .stream_chat("Count from 1 to 3")
        .await
        .unwrap();

    let mut full_response = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => full_response.push_str(&text),
            Err(e) => panic!("Stream error: {}", e),
        }
    }

    assert!(!full_response.is_empty());
    // Check that we got some numbers
    assert!(full_response.contains('1') || full_response.contains("one"));
}

#[tokio::test]
async fn test_client_request_builder() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping request builder test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    let response = client
        .request()
        .model("gpt-4o-mini")
        .system("You are a concise assistant. Respond in one word only.")
        .user("What is 2+2?")
        .temperature(0.0)
        .max_tokens(10)
        .send()
        .await
        .unwrap();

    // Should get a very short response
    assert!(response.content.len() < 50);
    assert!(response.content.contains('4') || response.content.to_lowercase().contains("four"));
}

#[tokio::test]
async fn test_client_with_multiple_messages() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping multiple messages test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider).with_model("gpt-4o-mini");

    let messages = vec![
        Message::system("You are a helpful assistant"),
        Message::user("My name is Alice"),
        Message::assistant("Nice to meet you, Alice!"),
        Message::user("What is my name?"),
    ];

    let response = client.chat(messages).await.unwrap();
    assert!(response.to_lowercase().contains("alice"));
}

#[tokio::test]
async fn test_client_with_tools() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping tools test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    // Create a simple calculator tool
    let calculator_tool = Tool {
        name: "calculator".to_string(),
        description: "A simple calculator that can add two numbers".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }),
    };

    let response = client
        .request()
        .model("gpt-4o-mini")
        .user("What is 25 + 17? Use the calculator tool.")
        .tool(calculator_tool)
        .send()
        .await
        .unwrap();

    // Should have tool calls
    assert!(!response.tool_calls.is_empty());
    assert_eq!(response.tool_calls[0].name, "calculator");
}

#[tokio::test]
async fn test_standalone_request_builder() {
    let request = RequestBuilder::new()
        .system("You are a helpful assistant")
        .user("Hello")
        .assistant("Hi there! How can I help you?")
        .user("What's the weather?")
        .model("gpt-4")
        .temperature(0.7)
        .max_tokens(150)
        .build();

    assert_eq!(request.messages.len(), 4);
    assert_eq!(request.model.to_string(), "gpt-4");
    assert_eq!(request.parameters.temperature, Some(0.7));
    assert_eq!(request.parameters.max_tokens, Some(150));
}

#[tokio::test]
async fn test_client_error_handling() {
    // Test with invalid API key
    let provider = OpenAI::with_api_key("invalid-key");
    let client = Client::new(provider);

    let result = client.chat("Hello").await;
    assert!(result.is_err());
}

/// Example showing all client features
#[tokio::test]
#[ignore] // This is more of a demo than a test
async fn demo_all_client_features() {
    if !has_api_keys() {
        return;
    }

    let api_key = env::var("OPENAI_API_KEY").unwrap();
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider).with_model("gpt-4o-mini");

    // Simple chat
    println!("=== Simple Chat ===");
    let response = client.chat("Hello!").await.unwrap();
    println!("Response: {}", response);

    // Streaming
    println!("\n=== Streaming ===");
    let mut stream = client.stream_chat("Count to 5").await.unwrap();
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk.unwrap());
    }
    println!();

    // Request builder
    println!("\n=== Request Builder ===");
    let response = client
        .request()
        .system("You are a pirate")
        .user("Hello")
        .temperature(1.0)
        .send()
        .await
        .unwrap();
    println!("Pirate says: {}", response.content);

    // With tools
    println!("\n=== With Tools ===");
    let weather_tool = Tool {
        name: "get_weather".to_string(),
        description: "Get weather for a location".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    };

    let response = client
        .request()
        .user("What's the weather in Paris?")
        .tool(weather_tool)
        .send()
        .await
        .unwrap();

    if !response.tool_calls.is_empty() {
        println!("Tool called: {:?}", response.tool_calls[0]);
    }
}