//! Example demonstrating tool calling with multiple providers

use cogni::providers::{Anthropic, Ollama, OpenAI};
use cogni::{Error, Function, Message, Provider, Request, Tool};
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize providers that support tool calling
    let openai = OpenAI::with_api_key(
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set"),
    )?;

    let anthropic = Anthropic::with_api_key(
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set"),
    )?;

    // Define a simple weather tool
    let weather_tool = Tool {
        name: "get_weather".to_string(),
        description: "Get the current weather in a given location".to_string(),
        function: Function {
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "The unit of temperature"
                    }
                },
                "required": ["location"]
            }),
            returns: Some("Weather information".to_string()),
        },
    };

    // Create a request that might trigger tool use
    let request = Request::builder()
        .message(Message::user(
            "What's the weather like in Tokyo and New York?",
        ))
        .tools([weather_tool.clone()])
        .max_tokens(500)
        .build();

    println!("Testing tool calling with multiple providers:\n");

    // Test OpenAI
    println!("OpenAI with tools:");
    match openai.request(request.clone()).await {
        Ok(response) => {
            if response.has_tool_calls() {
                println!("Tool calls requested:");
                for call in &response.tool_calls {
                    println!("  - {} with args: {}", call.name, call.arguments);
                }
            } else {
                println!("Response: {}", response.content);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    println!();

    // Test Anthropic
    println!("Anthropic with tools:");
    let anthropic_request = Request::builder()
        .message(Message::user(
            "What's the weather like in Tokyo and New York?",
        ))
        .model("claude-3-haiku-20240307")
        .tools([weather_tool.clone()])
        .max_tokens(500)
        .build();
    match anthropic.request(anthropic_request).await {
        Ok(response) => {
            if response.has_tool_calls() {
                println!("Tool calls requested:");
                for call in &response.tool_calls {
                    println!("  - {} with args: {}", call.name, call.arguments);
                }
            } else {
                println!("Response: {}", response.content);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Demonstrate streaming with tool calls
    println!("\n\nStreaming with tool calls:");

    let streaming_request = Request::builder()
        .message(Message::user(
            "Check the weather in Paris, France using the weather tool.",
        ))
        .tools([Tool {
            name: "get_weather".to_string(),
            description: "Get the current weather in a given location".to_string(),
            function: Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state/country"
                        }
                    },
                    "required": ["location"]
                }),
                returns: Some("Weather data".to_string()),
            },
        }])
        .build();

    // Stream from OpenAI
    println!("\nOpenAI (streaming with tools):");
    match stream_with_tools(&openai, streaming_request.clone()).await {
        Ok(_) => println!(),
        Err(e) => println!("Error: {}", e),
    }

    // Stream from Anthropic
    println!("\nAnthropic (streaming with tools):");
    let anthropic_streaming_request = Request::builder()
        .message(Message::user(
            "Check the weather in Paris, France using the weather tool.",
        ))
        .model("claude-3-haiku-20240307")
        .tools([Tool {
            name: "get_weather".to_string(),
            description: "Get the current weather in a given location".to_string(),
            function: Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state, e.g. San Francisco, CA"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "description": "The unit of temperature"
                        }
                    },
                    "required": ["location"]
                }),
                returns: Some("Weather information".to_string()),
            },
        }])
        .max_tokens(500)
        .build();
    match stream_with_tools(&anthropic, anthropic_streaming_request).await {
        Ok(_) => println!(),
        Err(e) => println!("Error: {}", e),
    }

    // Test with Ollama
    let ollama = Ollama::local()?;

    println!("\n\nOllama with tools:");
    let ollama_request = Request::builder()
        .message(Message::user(
            "What is the weather like in Berlin? Use the weather tool to check.",
        ))
        .model("llama3.2")
        .tools([weather_tool.clone()])
        .build();

    match ollama.request(ollama_request).await {
        Ok(response) => {
            if response.has_tool_calls() {
                println!("Tool calls requested:");
                for call in &response.tool_calls {
                    println!("  - {} with args: {}", call.name, call.arguments);
                }
            } else {
                println!("Response: {}", response.content);
                if response.tool_calls.is_empty() {
                    println!("(Note: Ollama model may not support tool calling)");
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}

async fn stream_with_tools<P: Provider>(provider: &P, request: Request) -> Result<(), Error> {
    use cogni::{StreamAccumulator, StreamEvent};
    use futures::StreamExt;

    let mut stream = provider.stream(request).await?;
    let mut accumulator = StreamAccumulator::new();

    while let Some(event) = stream.next().await {
        let event = event?;
        accumulator.process_event(event.clone())?;

        match event {
            StreamEvent::Content(delta) => print!("{}", delta.text),
            StreamEvent::ToolCall(delta) => {
                println!("\n[Tool Call Delta] index: {}", delta.index);
                if let Some(id) = delta.id {
                    println!("  ID: {}", id);
                }
                if let Some(name) = delta.name {
                    println!("  Name: {}", name);
                }
                if let Some(args) = delta.arguments {
                    println!("  Arguments chunk: {}", args);
                }
            }
            StreamEvent::Metadata(delta) => {
                if let Some(model) = delta.model {
                    println!("[Using model: {}]", model);
                }
            }
            StreamEvent::Done => {
                println!("\n[Stream complete]");

                // Show accumulated tool calls
                let tool_calls = accumulator.tool_calls();
                if !tool_calls.is_empty() {
                    println!("\nAccumulated tool calls:");
                    for call in tool_calls {
                        println!("  - {} ({}): {}", call.name, call.id, call.arguments);
                    }
                }
                break;
            }
        }
    }

    Ok(())
}
