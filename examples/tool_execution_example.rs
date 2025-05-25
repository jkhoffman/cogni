//! Example demonstrating tool execution with the Cogni framework

use cogni_core::{Provider, Request, Message, Error, ToolCall};
use cogni_providers::OpenAI;
use cogni_tools::{ToolRegistry, FunctionExecutorBuilder, builtin};
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Create a tool registry
    let registry = ToolRegistry::new();
    
    // Register built-in tools
    registry.register(builtin::calculator()).await?;
    registry.register(builtin::string_tools()).await?;
    
    // Create a custom weather tool
    let weather_tool = FunctionExecutorBuilder::new("get_weather")
        .description("Get the current weather for a location")
        .parameters(json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit"
                }
            },
            "required": ["location"]
        }))
        .returns("Weather information including temperature and conditions")
        .build_async(|args| async move {
            let location = args["location"].as_str().unwrap_or("Unknown");
            let unit = args.get("unit")
                .and_then(|u| u.as_str())
                .unwrap_or("celsius");
            
            // Simulate weather API call
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            // Return mock weather data
            Ok(json!({
                "location": location,
                "temperature": if unit == "celsius" { 22 } else { 72 },
                "unit": unit,
                "conditions": "Partly cloudy",
                "humidity": "65%",
                "wind": "10 km/h"
            }))
        });
    
    registry.register(weather_tool).await?;
    
    println!("=== Registered Tools ===");
    for tool in registry.list_tools().await {
        println!("- {}: {}", tool.name, tool.description);
    }
    println!();
    
    // Example 1: Direct tool execution
    println!("=== Direct Tool Execution ===");
    
    let calc_call = ToolCall {
        id: "calc-1".to_string(),
        name: "calculator".to_string(),
        arguments: json!({
            "operation": "multiply",
            "a": 7,
            "b": 8
        }).to_string(),
    };
    
    let result = registry.execute(&calc_call).await?;
    println!("Calculator result: {}", result.content);
    
    // Example 2: Parallel tool execution
    println!("\n=== Parallel Tool Execution ===");
    
    let tool_calls = vec![
        ToolCall {
            id: "weather-1".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({
                "location": "New York, NY",
                "unit": "fahrenheit"
            }).to_string(),
        },
        ToolCall {
            id: "string-1".to_string(),
            name: "string_tools".to_string(),
            arguments: json!({
                "operation": "uppercase",
                "text": "hello cogni"
            }).to_string(),
        },
        ToolCall {
            id: "calc-2".to_string(),
            name: "calculator".to_string(),
            arguments: json!({
                "operation": "add",
                "a": 100,
                "b": 23
            }).to_string(),
        },
    ];
    
    let results = registry.execute_many(&tool_calls).await;
    
    for (call, result) in tool_calls.iter().zip(results.iter()) {
        match result {
            Ok(r) => println!("{}: {}", call.name, r.content),
            Err(e) => println!("{}: Error - {}", call.name, e),
        }
    }
    
    // Example 3: LLM integration (if API key is available)
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("\n=== LLM Tool Usage ===");
        
        let openai = OpenAI::new(&api_key);
        
        // Get available tools for the LLM
        let tools = registry.list_tools().await;
        
        // Create a request with tools
        let request = Request::builder()
            .model("gpt-4")
            .message(Message::user("What's the weather in Paris, France? Also calculate 42 * 17."))
            .tools(tools)
            .build();
        
        // Get response from LLM
        let response = openai.request(request).await?;
        
        // Check if the LLM wants to call tools
        if !response.tool_calls.is_empty() {
            println!("LLM requested {} tool calls:", response.tool_calls.len());
            
            // Execute the requested tools
            let tool_results = registry.execute_many(&response.tool_calls).await;
            
            for (call, result) in response.tool_calls.iter().zip(tool_results.iter()) {
                match result {
                    Ok(r) => println!("- {}: {}", call.name, if r.success { &r.content } else { "Failed" }),
                    Err(e) => println!("- {}: Error - {}", call.name, e),
                }
            }
        } else {
            println!("LLM response: {}", response.content);
        }
    } else {
        println!("\n(Skipping LLM example - OPENAI_API_KEY not set)");
    }
    
    Ok(())
}