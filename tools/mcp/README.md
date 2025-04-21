# Cogni MCP Tool

A Rust implementation of the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) for the Cogni framework. This tool enables interaction with MCP-compatible servers and exposes Cogni tools via the MCP protocol.

## Features

- Full implementation of the MCP JSON-RPC protocol
- Bidirectional communication with MCP servers over stdio
- Concurrency control with configurable limits
- Rate limiting for tool calls
- Retry mechanisms for transient failures
- Error handling and mapping to Cogni errors

## Usage

### Basic Client Usage

```rust
use cogni_tool_mcp::{MCPClient, MCPClientConfig};
use cogni_tools_common::RateLimiterConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client
    let config = MCPClientConfig {
        server_path: "/path/to/mcp/server.py", // or .js for Node.js servers
        env: Some(vec![("PYTHONUNBUFFERED".to_string(), "1".to_string())]),
        startup_timeout_secs: 5,
        max_concurrent_requests: 5,
        max_retries: 3,
        rate_limiter_config: RateLimiterConfig::default(),
    };

    // Connect to the server
    let mut client = MCPClient::connect(config).await?;

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:?}", tools);

    // Call a tool
    let input = serde_json::json!({
        "query": "What's the weather in London?"
    });
    let result = client.call_tool("weather", input, None).await?;
    println!("Tool result: {:?}", result);

    Ok(())
}
```

### Using the Router to Expose Cogni Tools

```rust
use cogni_tool_mcp::routing::MCPRouter;
use cogni_tools_registry::ToolRegistry;
use std::sync::Arc;

async fn expose_tools() -> Result<(), Box<dyn std::error::Error>> {
    // Create a tool registry
    let mut registry = ToolRegistry::new();
    
    // Register your tools
    // registry.register("math", Arc::new(MathTool::new(config)?))?;
    // registry.register("search", Arc::new(SearchTool::new(config)?))?;
    
    // Create an MCP router with the registry
    let router = MCPRouter::new(registry);
    
    // Process a tool call
    let call_json = r#"{"jsonrpc": "2.0", "id": 1, "method": "callTool", "params": {"toolName": "math", "input": {"expression": "2+2"}}}"#;
    let result = router.handle_call(call_json).await?;
    
    Ok(())
}
```

## API Reference

### `MCPClientConfig`

Configuration for the MCP client:

```rust
pub struct MCPClientConfig {
    pub server_path: String,                       // Path to the MCP server executable
    pub env: Option<Vec<(String, String)>>,        // Optional environment variables
    pub startup_timeout_secs: u64,                 // Timeout for server startup
    pub max_concurrent_requests: usize,            // Maximum concurrent requests
    pub max_retries: u32,                          // Maximum number of retries
    pub rate_limiter_config: RateLimiterConfig,    // Rate limiting configuration
}
```

### `MCPClient`

Main client for interacting with MCP servers:

- `connect(config: MCPClientConfig) -> Result<Self, McpError>` - Connect to an MCP server
- `list_tools() -> Result<Vec<ToolSpec>, McpError>` - List available tools
- `call_tool(tool_name: &str, input: serde_json::Value, request_id: Option<String>) -> Result<ToolResult, McpError>` - Call a tool

### `MCPRouter`

Routes MCP calls to Cogni tools:

- `new(registry: ToolRegistry) -> Self` - Create a new router
- `handle_call(call_json: &str) -> Result<String, McpError>` - Handle a JSON-RPC call
- `list_tools() -> Result<Vec<ToolSpec>, McpError>` - List available tools

### Error Handling

The MCP tool defines its own error type `McpError` that maps to various failure modes:

```rust
pub enum McpError {
    Transport(String),     // Communication errors
    Protocol(String),      // MCP protocol errors
    Serialization(String), // JSON serialization errors
    Timeout(String),       // Request timeouts
    Tool(String),          // Tool execution errors
}
```

## Examples

See the `examples` directory for complete working examples:

- `simple_client.rs` - Basic MCP client usage
- `tool_server.rs` - Exposing Cogni tools via MCP
- `chat_router.rs` - Integration with LLM chat interface

## License

This project is licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option. 