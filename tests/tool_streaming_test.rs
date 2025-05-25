//! Integration tests for tool execution with streaming

use cogni::prelude::*;
use cogni::providers::{Anthropic, OpenAI};
use cogni::tools::executor::FunctionExecutorBuilder;
use cogni::tools::ToolRegistry;
use cogni::{Function, RequestBuilder, Response, ResponseMetadata, Tool};
use futures::StreamExt;
use serde_json::json;
use std::env;

/// Helper to add tools to a request builder
fn add_tools(mut builder: RequestBuilder, tools: &[Tool]) -> RequestBuilder {
    for tool in tools {
        builder = builder.tool(tool.clone());
    }
    builder
}

/// Create test tools for streaming
fn create_streaming_tools() -> Vec<Tool> {
    vec![Tool {
        name: "analyze_text".to_string(),
        description: "Analyze text and return statistics".to_string(),
        function: Function {
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text to analyze"
                    }
                },
                "required": ["text"]
            }),
            returns: Some(
                "Text statistics including word count, character count, etc.".to_string(),
            ),
        },
    }]
}

async fn create_streaming_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    let analyzer = FunctionExecutorBuilder::new("analyze_text")
        .description("Analyze text and return statistics")
        .parameters(json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to analyze"
                }
            },
            "required": ["text"]
        }))
        .build_sync(|args| {
            let text = args["text"].as_str().unwrap_or("");
            let words: Vec<&str> = text.split_whitespace().collect();
            let chars = text.len();
            let sentences = text.matches(|c| c == '.' || c == '!' || c == '?').count();

            Ok(json!({
                "word_count": words.len(),
                "character_count": chars,
                "sentence_count": sentences,
                "average_word_length": if words.is_empty() {
                    0.0
                } else {
                    words.iter().map(|w| w.len()).sum::<usize>() as f64 / words.len() as f64
                }
            }))
        });

    registry.register(analyzer).await.unwrap();
    registry
}

#[tokio::test]
async fn test_openai_streaming_with_tools() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI streaming tool test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let tools = create_streaming_tools();
    let registry = create_streaming_registry().await;

    // Create request
    let mut builder = Request::builder()
        .model("gpt-4")
        .message(Message::user(
            "Analyze this text: 'The quick brown fox jumps over the lazy dog. This is a test sentence!'"
        ))
        .max_tokens(200);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Stream response
    let mut stream = provider.stream(request).await.unwrap();
    let mut accumulator = StreamAccumulator::new();
    let mut tool_calls_started = false;

    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::Content(delta) => {
                print!("{}", delta.text);
            }
            StreamEvent::ToolCall(delta) => {
                tool_calls_started = true;
                accumulator
                    .process_event(StreamEvent::ToolCall(delta))
                    .unwrap();
            }
            StreamEvent::Done => {
                println!("\n[Stream done]");
                break;
            }
            _ => {}
        }
    }

    // Get accumulated response
    let response = Response {
        content: accumulator.content().to_string(),
        tool_calls: accumulator.tool_calls(),
        metadata: ResponseMetadata::default(),
    };

    // If tool calls were made, execute them
    if !response.tool_calls.is_empty() {
        println!("\nExecuting {} tool calls...", response.tool_calls.len());

        for tool_call in &response.tool_calls {
            let result = registry.execute(tool_call).await.unwrap();
            println!("Tool result: {}", result.content);
            println!("Tool result raw bytes: {:?}", result.content.as_bytes());

            // Verify the analysis
            match serde_json::from_str::<serde_json::Value>(&result.content) {
                Ok(analysis) => {
                    assert!(analysis["word_count"].as_u64().unwrap() > 0);
                    assert!(analysis["character_count"].as_u64().unwrap() > 0);
                }
                Err(e) => {
                    panic!("Failed to parse JSON: {}\nContent: {:?}", e, result.content);
                }
            }
        }
    } else if !tool_calls_started {
        println!("No tool calls were made during streaming");
    }
}

#[tokio::test]
async fn test_anthropic_streaming_with_tools() {
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping Anthropic streaming tool test - ANTHROPIC_API_KEY not set");
            return;
        }
    };

    let provider = Anthropic::with_api_key(api_key);
    let tools = create_streaming_tools();
    let registry = create_streaming_registry().await;

    // Create request
    let mut builder = Request::builder()
        .model("claude-3-5-sonnet-20241022")
        .message(Message::user(
            "Please analyze this text using the analyze_text tool: 'Hello world. How are you today?'"
        ))
        .max_tokens(200);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Stream response
    let mut stream = provider.stream(request).await.unwrap();
    let mut accumulator = StreamAccumulator::new();
    let mut content_chunks = Vec::new();

    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::Content(delta) => {
                content_chunks.push(delta.text.clone());
                print!("{}", delta.text);
            }
            StreamEvent::ToolCall(delta) => {
                accumulator
                    .process_event(StreamEvent::ToolCall(delta))
                    .unwrap();
            }
            StreamEvent::Done => {
                println!("\n[Stream done]");
                break;
            }
            _ => {}
        }
    }

    // Get accumulated response
    let response = Response {
        content: accumulator.content().to_string(),
        tool_calls: accumulator.tool_calls(),
        metadata: ResponseMetadata::default(),
    };

    // Execute tool calls if any
    if !response.tool_calls.is_empty() {
        println!("\nExecuting tool calls...");

        for tool_call in &response.tool_calls {
            let result = registry.execute(tool_call).await.unwrap();
            println!("Tool result: {}", result.content);
        }
    }

    // Verify we got some content
    assert!(
        !content_chunks.is_empty() || !response.tool_calls.is_empty(),
        "Expected either content or tool calls"
    );
}

#[tokio::test]
async fn test_streaming_accumulation_with_tools() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping streaming accumulation test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);

    // Create tools
    let tools = vec![Tool {
        name: "get_random_number".to_string(),
        description: "Generate a random number".to_string(),
        function: Function {
            parameters: json!({
                "type": "object",
                "properties": {
                    "min": { "type": "number", "description": "Minimum value" },
                    "max": { "type": "number", "description": "Maximum value" }
                },
                "required": ["min", "max"]
            }),
            returns: Some("A random number between min and max".to_string()),
        },
    }];

    // Create request that might trigger multiple tool calls
    let mut builder = Request::builder()
        .model("gpt-4")
        .message(Message::user(
            "Generate three random numbers: one between 1-10, one between 50-100, and one between 1000-2000."
        ))
        .max_tokens(300);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Stream and accumulate
    let mut stream = provider.stream(request).await.unwrap();
    let mut accumulator = StreamAccumulator::new();
    let mut event_count = 0;

    while let Some(event) = stream.next().await {
        event_count += 1;
        match event.unwrap() {
            StreamEvent::Content(delta) => {
                accumulator
                    .process_event(StreamEvent::Content(delta))
                    .unwrap();
            }
            StreamEvent::ToolCall(delta) => {
                println!(
                    "Tool call delta: index={}, id={:?}, name={:?}",
                    delta.index, delta.id, delta.name
                );
                accumulator
                    .process_event(StreamEvent::ToolCall(delta))
                    .unwrap();
            }
            StreamEvent::Metadata(delta) => {
                accumulator
                    .process_event(StreamEvent::Metadata(delta))
                    .unwrap();
            }
            StreamEvent::Done => break,
        }
    }

    // Build final response
    let response = Response {
        content: accumulator.content().to_string(),
        tool_calls: accumulator.tool_calls(),
        metadata: ResponseMetadata::default(),
    };

    println!("\n--- Accumulated Response ---");
    println!("Content length: {}", response.content.len());
    println!("Tool calls: {}", response.tool_calls.len());
    println!("Total events: {}", event_count);

    // Verify accumulation worked
    if !response.tool_calls.is_empty() {
        for (i, call) in response.tool_calls.iter().enumerate() {
            println!(
                "Tool call {}: {} with args: {}",
                i, call.name, call.arguments
            );

            // Verify the arguments are valid JSON
            let args: serde_json::Value = serde_json::from_str(&call.arguments).unwrap();
            assert!(args.is_object());
            assert!(args.get("min").is_some());
            assert!(args.get("max").is_some());
        }
    }
}

#[tokio::test]
async fn test_interleaved_content_and_tools() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping interleaved test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let tools = create_streaming_tools();

    // Request that should produce both content and tool calls
    let mut builder = Request::builder()
        .model("gpt-4")
        .message(Message::user(
            "First, tell me about text analysis, then analyze this text: 'Testing 123.'",
        ))
        .max_tokens(300);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    let mut stream = provider.stream(request).await.unwrap();
    let mut content_events = 0;
    let mut tool_events = 0;

    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::Content(_) => content_events += 1,
            StreamEvent::ToolCall(_) => tool_events += 1,
            StreamEvent::Done => break,
            _ => {}
        }
    }

    println!(
        "Content events: {}, Tool events: {}",
        content_events, tool_events
    );

    // We expect to see both types of events (or at least one type)
    assert!(
        content_events > 0 || tool_events > 0,
        "Expected either content or tool events"
    );
}
