use anyhow::Result;
use cogni_tool_mcp::{MCPClient, MCPClientConfig};
use cogni_tools_common::RateLimiterConfig;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure the client
    let config = MCPClientConfig {
        server_path: "./examples/mock_server.py".to_string(),
        env: Some(vec![("PYTHONUNBUFFERED".to_string(), "1".to_string())]),
        startup_timeout_secs: 5,
        max_concurrent_requests: 3,
        max_retries: 2,
        rate_limiter_config: RateLimiterConfig {
            max_requests_per_minute: 10,
            ..Default::default()
        },
    };

    println!("Connecting to MCP server...");
    let mut client = MCPClient::connect(config).await?;
    println!("Connected successfully!");

    // List available tools
    println!("Listing available tools...");
    let tools = client.list_tools().await?;
    println!("Available tools:");
    for tool in tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

    // Call a tool
    println!("\nCalling 'example' tool...");
    let input = json!({
        "message": "Hello from Cogni!"
    });

    let result = client.call_tool("example", input, None).await?;
    println!("Tool result: {}", result.output);

    // Demonstrate concurrency
    println!("\nDemonstrating concurrent calls...");
    let mut handles = Vec::new();

    for i in 1..=5 {
        let mut client_clone = client.clone();
        let input = json!({ "message": format!("Concurrent request {}", i) });

        let handle = tokio::spawn(async move {
            println!("Starting request {}...", i);
            let start = std::time::Instant::now();
            let result = client_clone.call_tool("example", input, None).await;
            let elapsed = start.elapsed();
            (i, result, elapsed)
        });

        handles.push(handle);

        // Small delay between spawning tasks
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    for handle in handles {
        let (i, result, elapsed) = handle.await?;
        match result {
            Ok(output) => println!(
                "Request {} completed in {:?}: {}",
                i, elapsed, output.output
            ),
            Err(e) => println!("Request {} failed: {:?}", i, e),
        }
    }

    // Clean shutdown
    println!("\nShutting down client...");
    client.shutdown().await?;
    println!("Client shutdown complete");

    Ok(())
}
