//! Example to test tool execution locally without API keys

use cogni_tools::{FunctionExecutorBuilder, ToolCall, ToolRegistry};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Local Tool Execution Test ===\n");

    // Create a registry
    let registry = ToolRegistry::new();

    // Register some tools
    let calc = FunctionExecutorBuilder::new("calculator")
        .description("Perform calculations")
        .parameters(json!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" }
            },
            "required": ["expression"]
        }))
        .build_sync(|args| {
            let expr = args["expression"].as_str().unwrap_or("");

            // Simple expression parser (just for demo)
            let result = match expr {
                "2 + 2" => 4.0,
                "10 * 5" => 50.0,
                "100 / 4" => 25.0,
                _ => {
                    return Ok(json!({ "error": "Cannot parse expression" }));
                }
            };

            Ok(json!({ "result": result }))
        });

    registry.register(calc).await?;

    // Test tool execution
    let test_calls = vec![
        ToolCall {
            id: "test-1".to_string(),
            name: "calculator".to_string(),
            arguments: json!({ "expression": "2 + 2" }).to_string(),
        },
        ToolCall {
            id: "test-2".to_string(),
            name: "calculator".to_string(),
            arguments: json!({ "expression": "10 * 5" }).to_string(),
        },
        ToolCall {
            id: "test-3".to_string(),
            name: "calculator".to_string(),
            arguments: json!({ "expression": "100 / 4" }).to_string(),
        },
    ];

    // Execute tools
    println!("Executing tool calls...");
    for call in &test_calls {
        let result = registry.execute(call).await?;
        println!("Call {}: {} -> {}", call.id, call.arguments, result.content);
        assert!(result.success);
    }

    // Test parallel execution
    println!("\nTesting parallel execution...");
    let start = std::time::Instant::now();
    let results = registry.execute_many(&test_calls).await;
    let duration = start.elapsed();

    println!("Executed {} calls in {:?}", results.len(), duration);
    for (call, result) in test_calls.iter().zip(results.iter()) {
        match result {
            Ok(r) => println!("  {} -> {}", call.id, r.content),
            Err(e) => println!("  {} -> Error: {}", call.id, e),
        }
    }

    println!("\n✅ All tests passed!");

    // Instructions for running integration tests
    println!("\n=== To run integration tests ===");
    println!("1. Set environment variables:");
    println!("   export OPENAI_API_KEY=your-key");
    println!("   export ANTHROPIC_API_KEY=your-key");
    println!("\n2. Run specific tests:");
    println!("   cargo test test_openai_tool_execution -- --nocapture");
    println!("   cargo test test_anthropic_tool_execution -- --nocapture");
    println!("   cargo test test_tool_error_handling -- --nocapture");
    println!("\n3. Run all tool tests:");
    println!("   cargo test tool_ -- --nocapture");

    Ok(())
}
