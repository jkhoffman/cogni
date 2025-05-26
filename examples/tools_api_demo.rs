//! Example demonstrating the improved tool API
//!
//! This example shows how to use the new convenience methods for working with tools.

use cogni::client::Client;
use cogni::providers::OpenAI;
use cogni::tools::{executor::FunctionExecutorBuilder, tools_vec, ToolExecutor, ToolRegistry};
use cogni::StructuredOutput;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct WeatherReport {
    location: String,
    temperature: f64,
    conditions: String,
    humidity: Option<u32>,
}

// Create a weather tool
fn create_weather_tool() -> impl ToolExecutor {
    FunctionExecutorBuilder::new("get_weather")
        .description("Get the current weather for a location")
        .parameters(json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                }
            },
            "required": ["location"]
        }))
        .build_sync(|args| {
            let location = args
                .get("location")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            // Simulate weather data
            let report = WeatherReport {
                location: location.to_string(),
                temperature: 72.5,
                conditions: "Partly cloudy".to_string(),
                humidity: Some(65),
            };

            serde_json::to_value(report).map_err(|e| e.into())
        })
}

// Create a calculation tool
fn create_calculator_tool() -> impl ToolExecutor {
    FunctionExecutorBuilder::new("calculate")
        .description("Perform basic mathematical calculations")
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
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            },
            "required": ["operation", "a", "b"]
        }))
        .build_sync(|args| {
            let operation = args.get("operation").and_then(|v| v.as_str()).unwrap();
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b != 0.0 {
                        a / b
                    } else {
                        return Ok(json!({ "error": "Division by zero" }));
                    }
                }
                _ => return Ok(json!({ "error": "Unknown operation" })),
            };

            Ok(json!({ "result": result }))
        })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("OPENAI_API_KEY")?;

    // Create provider
    let provider = OpenAI::with_api_key(api_key);

    // Method 1: Create registry using from_executors with the tools_vec! macro
    let registry =
        ToolRegistry::from_executors(tools_vec![create_weather_tool(), create_calculator_tool()])
            .await?;

    // Get tool definitions from registry
    let tool_defs = registry.list_tools().await;

    // Create client
    let client = Client::new(provider);

    // Method 2: Add tools directly to request using fluent API
    let response = client
        .request()
        .system("You are a helpful assistant with access to weather and calculation tools.")
        .user("What's the weather like in San Francisco? Also, what's 25 * 4?")
        .tools(tool_defs) // Add all tools from registry
        .send()
        .await?;

    println!("Assistant: {}", response.content);

    // Execute any tool calls
    if !response.tool_calls.is_empty() {
        println!("\nExecuting {} tool calls...", response.tool_calls.len());

        let results = registry.execute_many(&response.tool_calls).await;

        for (call, result) in response.tool_calls.iter().zip(results.iter()) {
            match result {
                Ok(tool_result) => {
                    println!("\nTool: {}", call.name);
                    println!("Arguments: {}", call.arguments);
                    println!("Result: {}", tool_result.content);
                }
                Err(e) => {
                    println!("\nError executing {}: {}", call.name, e);
                }
            }
        }
    }

    // Method 3: Using individual tool registration
    let registry2 = ToolRegistry::new();
    registry2.register(create_weather_tool()).await?;
    registry2.register(create_calculator_tool()).await?;

    println!(
        "\n\nAlternative registry has {} tools",
        registry2.len().await
    );

    // Method 4: Using the builder pattern (most ergonomic for inline creation)
    let registry3 = ToolRegistry::builder()
        .with_tools(tools_vec![create_weather_tool(), create_calculator_tool()])
        .build()
        .await?;

    println!("Builder registry has {} tools", registry3.len().await);

    Ok(())
}
