//! Integration tests for tool execution with providers

use cogni::prelude::*;
use cogni::providers::{Anthropic, Ollama, OpenAI};
use cogni::tools::executor::FunctionExecutorBuilder;
use cogni::tools::{builtin, ToolRegistry};
use cogni::{Function, RequestBuilder, Tool, ToolCall};
use serde_json::json;
use std::env;

/// Helper to add tools to a request builder
fn add_tools(mut builder: RequestBuilder, tools: &[Tool]) -> RequestBuilder {
    for tool in tools {
        builder = builder.tool(tool.clone());
    }
    builder
}

/// Helper to create a calculator tool
fn create_test_tools() -> Vec<Tool> {
    vec![
        // Simple calculator
        Tool {
            name: "calculator".to_string(),
            description: "Perform basic arithmetic operations".to_string(),
            function: Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"],
                            "description": "The operation to perform"
                        },
                        "a": {
                            "type": "number",
                            "description": "First number"
                        },
                        "b": {
                            "type": "number",
                            "description": "Second number"
                        }
                    },
                    "required": ["operation", "a", "b"]
                }),
                returns: Some("The result of the calculation".to_string()),
            },
        },
        // Get current time
        Tool {
            name: "get_current_time".to_string(),
            description: "Get the current date and time".to_string(),
            function: Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "description": "Timezone (e.g., 'UTC', 'America/New_York')"
                        }
                    }
                }),
                returns: Some("Current date and time".to_string()),
            },
        },
    ]
}

/// Create a registry with test tools
async fn create_test_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Register calculator
    let calc = FunctionExecutorBuilder::new("calculator")
        .description("Perform basic arithmetic operations")
        .parameters(json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["operation", "a", "b"]
        }))
        .build_sync(|args| {
            let op = args["operation"].as_str().unwrap_or("");
            let a = args["a"].as_f64().unwrap_or(0.0);
            let b = args["b"].as_f64().unwrap_or(0.0);

            let result = match op {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b != 0.0 {
                        a / b
                    } else {
                        f64::NAN
                    }
                }
                _ => return Ok(json!({ "error": "Unknown operation" })),
            };

            Ok(json!({ "result": result }))
        });

    registry.register([calc]).await.unwrap();

    // Register time tool
    let time_tool = FunctionExecutorBuilder::new("get_current_time")
        .description("Get the current date and time")
        .parameters(json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "Timezone (e.g., 'UTC', 'America/New_York')"
                }
            }
        }))
        .build_sync(|args| {
            let timezone = args
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("UTC");

            Ok(json!({
                "time": chrono::Utc::now().to_rfc3339(),
                "timezone": timezone
            }))
        });

    registry.register([time_tool]).await.unwrap();

    registry
}

#[tokio::test]
async fn test_openai_tool_execution() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI tool test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let tools = create_test_tools();
    let registry = create_test_registry().await;

    // Create a request that should trigger tool use
    let mut builder = Request::builder()
        .model("gpt-4")
        .message(Message::user(
            "What is 25 * 4? Please use the calculator tool.",
        ))
        .max_tokens(150);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Get response
    let response = provider.request(request).await.unwrap();

    // Check if tools were called
    assert!(
        !response.tool_calls.is_empty(),
        "Expected tool calls but got none"
    );

    // Execute the tool calls
    for tool_call in &response.tool_calls {
        println!(
            "Tool call: {} with args: {}",
            tool_call.name, tool_call.arguments
        );

        let result = registry.execute(tool_call).await.unwrap();
        println!("Tool result: {}", result.content);

        // Verify it's a calculator call
        assert_eq!(tool_call.name, "calculator");

        // Parse and verify the result
        let result_json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(result_json["result"], 100.0);
    }
}

#[tokio::test]
async fn test_anthropic_tool_execution() {
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping Anthropic tool test - ANTHROPIC_API_KEY not set");
            return;
        }
    };

    let provider = Anthropic::with_api_key(api_key);
    let tools = create_test_tools();
    let registry = create_test_registry().await;

    // Create a request that should trigger tool use
    let mut builder = Request::builder()
        .model("claude-3-5-sonnet-20241022")
        .message(Message::user(
            "What is 15 + 27? Use the calculator tool to compute this.",
        ))
        .max_tokens(150);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Get response
    let response = provider.request(request).await.unwrap();

    // Check if tools were called
    assert!(
        !response.tool_calls.is_empty(),
        "Expected tool calls but got none"
    );

    // Execute the tool calls
    for tool_call in &response.tool_calls {
        println!(
            "Tool call: {} with args: {}",
            tool_call.name, tool_call.arguments
        );

        let result = registry.execute(tool_call).await.unwrap();
        println!("Tool result: {}", result.content);

        // Verify it's a calculator call
        assert_eq!(tool_call.name, "calculator");

        // Parse and verify the result
        let result_json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(result_json["result"], 42.0);
    }
}

#[tokio::test]
async fn test_ollama_tool_execution() {
    // Check if Ollama is running
    let client = reqwest::Client::new();
    if client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .is_err()
    {
        eprintln!("Skipping Ollama tool test - Ollama not running");
        return;
    }

    let provider = Ollama::local();
    let tools = create_test_tools();
    let registry = create_test_registry().await;

    // Create a request that should trigger tool use
    let mut builder = Request::builder()
        .model("llama3.2") // or another model that supports tools
        .message(Message::user(
            "Calculate 12 divided by 3 using the calculator tool.",
        ));

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Get response
    match provider.request(request).await {
        Ok(response) => {
            if !response.tool_calls.is_empty() {
                // Execute the tool calls
                for tool_call in &response.tool_calls {
                    println!(
                        "Tool call: {} with args: {}",
                        tool_call.name, tool_call.arguments
                    );

                    let result = registry.execute(tool_call).await.unwrap();
                    println!("Tool result: {}", result.content);

                    // Verify it's a calculator call
                    assert_eq!(tool_call.name, "calculator");
                }
            } else {
                println!("Ollama did not use tools - model may not support tool calling");
            }
        }
        Err(e) => {
            eprintln!("Ollama request failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_multiple_tool_calls() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping multiple tool test - OPENAI_API_KEY not set");
            return;
        }
    };

    let provider = OpenAI::with_api_key(api_key);
    let tools = create_test_tools();
    let registry = create_test_registry().await;

    // Create a request that should trigger multiple tool calls
    let mut builder = Request::builder()
        .model("gpt-4")
        .message(Message::user(
            "Please do these calculations separately: First, calculate 10 + 5. Second, calculate 20 * 3. Third, tell me the current time. Use a separate tool call for each."
        ))
        .max_tokens(300);

    builder = add_tools(builder, &tools);
    let request = builder.build();

    // Get response
    let response = provider.request(request).await.unwrap();

    println!("Got {} tool calls", response.tool_calls.len());

    // We expect at least 1 tool call, ideally 3
    // Note: LLMs may combine operations, so we're flexible
    if response.tool_calls.is_empty() {
        panic!("Expected at least one tool call");
    }

    if response.tool_calls.len() < 2 {
        eprintln!("Warning: Only got {} tool call(s), expected multiple. LLM may have combined operations.", response.tool_calls.len());
    }

    // Execute all tool calls
    let results = registry.execute_many(&response.tool_calls).await;

    for (call, result) in response.tool_calls.iter().zip(results.iter()) {
        match result {
            Ok(r) => {
                println!("Tool {} succeeded: {}", call.name, r.content);
                assert!(r.success);
            }
            Err(e) => panic!("Tool execution failed: {}", e),
        }
    }
}

#[tokio::test]
async fn test_tool_error_handling() {
    let registry = ToolRegistry::new();

    // Register a tool that always fails
    let failing_tool = FunctionExecutorBuilder::new("failing_tool")
        .description("A tool that always fails")
        .parameters(json!({
            "type": "object",
            "properties": {}
        }))
        .build_sync(|_args| {
            Err(cogni::tools::error::ToolError::ExecutionFailed {
                tool: "failing_tool".to_string(),
                message: "This tool always fails".to_string(),
                source: None,
            })
        });

    registry.register([failing_tool]).await.unwrap();

    // Try to execute the failing tool
    let call = ToolCall {
        id: "test-1".to_string(),
        name: "failing_tool".to_string(),
        arguments: "{}".to_string(),
    };

    let result = registry.execute(&call).await.unwrap();

    // The result should indicate failure
    assert!(!result.success);
    assert!(result.content.contains("This tool always fails"));
}

#[tokio::test]
async fn test_tool_validation() {
    let registry = ToolRegistry::new();

    // Register calculator with strict validation
    let calc = builtin::calculator();
    registry.register([calc]).await.unwrap();

    // Try with invalid arguments
    let call = ToolCall {
        id: "test-1".to_string(),
        name: "calculator".to_string(),
        arguments: json!({
            "operation": "invalid_op",
            "a": "not_a_number",
            "b": true
        })
        .to_string(),
    };

    let result = registry.execute(&call).await.unwrap();

    // Should handle invalid operation gracefully
    let result_json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(result_json.get("error").is_some() || result_json.get("result").is_some());
}
