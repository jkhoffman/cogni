//! Example showing how to create custom tools

use async_trait::async_trait;
use cogni_tools::{
    validation::param_schema, FunctionExecutorBuilder, ToolCall, ToolExecutor, ToolRegistry,
};
use serde_json::{json, Value};

// Example 1: Simple synchronous tool
fn create_echo_tool() -> impl ToolExecutor {
    FunctionExecutorBuilder::new("echo")
        .description("Echo back the input message")
        .parameters(
            param_schema()
                .string_required("message", "The message to echo")
                .boolean("uppercase", "Whether to convert to uppercase")
                .build(),
        )
        .build_sync(|args| {
            let message = args["message"].as_str().unwrap_or("");
            let uppercase = args
                .get("uppercase")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let result = if uppercase {
                message.to_uppercase()
            } else {
                message.to_string()
            };

            Ok(json!({ "echoed": result }))
        })
}

// Example 2: Async tool with external API simulation
fn create_stock_price_tool() -> impl ToolExecutor {
    FunctionExecutorBuilder::new("get_stock_price")
        .description("Get the current stock price for a symbol")
        .parameters(json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "Stock symbol (e.g., AAPL, GOOGL)"
                },
                "exchange": {
                    "type": "string",
                    "enum": ["NYSE", "NASDAQ", "LSE"],
                    "description": "Stock exchange"
                }
            },
            "required": ["symbol"]
        }))
        .build_async(|args| async move {
            let symbol = args["symbol"].as_str().unwrap_or("UNKNOWN");

            // Simulate API call delay
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Return mock data
            let price = match symbol {
                "AAPL" => 175.43,
                "GOOGL" => 142.87,
                "MSFT" => 378.91,
                _ => 100.00,
            };

            Ok(json!({
                "symbol": symbol,
                "price": price,
                "currency": "USD",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        })
}

// Example 3: Custom executor with validation
struct FileOperationTool;

#[async_trait]
impl ToolExecutor for FileOperationTool {
    async fn execute(&self, call: &ToolCall) -> cogni_tools::error::Result<cogni_core::ToolResult> {
        let args: Value = serde_json::from_str(&call.arguments)?;

        // Validate arguments
        self.validate(&args).await?;

        let operation = args["operation"].as_str().unwrap();
        let path = args["path"].as_str().unwrap();

        // Simulate file operations (don't actually do them)
        let result = match operation {
            "exists" => json!({ "exists": path.contains(".") }),
            "size" => json!({ "size": path.len() * 100 }),
            "type" => json!({ "type": if path.ends_with('/') { "directory" } else { "file" } }),
            _ => json!({ "error": "Unknown operation" }),
        };

        Ok(cogni_core::ToolResult::success(
            &call.id,
            serde_json::to_string(&result)?,
        ))
    }

    fn tool(&self) -> &cogni_core::Tool {
        static TOOL: std::sync::OnceLock<cogni_core::Tool> = std::sync::OnceLock::new();
        TOOL.get_or_init(|| cogni_core::Tool {
            name: "file_operations".to_string(),
            description: "Perform file system operations".to_string(),
            function: cogni_core::Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["exists", "size", "type"],
                            "description": "The operation to perform"
                        },
                        "path": {
                            "type": "string",
                            "description": "File or directory path"
                        }
                    },
                    "required": ["operation", "path"]
                }),
                returns: Some("File operation result".to_string()),
            },
        })
    }

    async fn validate(&self, args: &Value) -> cogni_tools::error::Result<()> {
        // Custom validation
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if path.contains("..") {
                return Err(cogni_tools::error::ToolError::ValidationFailed {
                    tool: "file_operations".to_string(),
                    errors: vec!["Path cannot contain '..'".to_string()],
                });
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry
    let registry = ToolRegistry::new();

    // Register tools
    registry.register(create_echo_tool()).await?;
    registry.register(create_stock_price_tool()).await?;
    registry.register(FileOperationTool).await?;

    println!("=== Available Tools ===");
    for tool in registry.list_tools().await {
        println!("- {}: {}", tool.name, tool.description);
        println!(
            "  Parameters: {}",
            serde_json::to_string_pretty(&tool.function.parameters)?
        );
        println!();
    }

    // Test the tools
    println!("=== Testing Tools ===\n");

    // Test echo tool
    let echo_call = ToolCall {
        id: "echo-1".to_string(),
        name: "echo".to_string(),
        arguments: json!({
            "message": "Hello, Cogni!",
            "uppercase": true
        })
        .to_string(),
    };

    let result = registry.execute(&echo_call).await?;
    println!("Echo result: {}", result.content);

    // Test stock price tool
    let stock_call = ToolCall {
        id: "stock-1".to_string(),
        name: "get_stock_price".to_string(),
        arguments: json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ"
        })
        .to_string(),
    };

    let result = registry.execute(&stock_call).await?;
    println!("Stock price result: {}", result.content);

    // Test file operations tool with validation
    let file_call = ToolCall {
        id: "file-1".to_string(),
        name: "file_operations".to_string(),
        arguments: json!({
            "operation": "exists",
            "path": "/tmp/test.txt"
        })
        .to_string(),
    };

    let result = registry.execute(&file_call).await?;
    println!("File operation result: {}", result.content);

    // Test validation failure
    let invalid_call = ToolCall {
        id: "file-2".to_string(),
        name: "file_operations".to_string(),
        arguments: json!({
            "operation": "exists",
            "path": "../etc/passwd"  // Should fail validation
        })
        .to_string(),
    };

    match registry.execute(&invalid_call).await {
        Ok(result) => println!("Unexpected success: {}", result.content),
        Err(e) => println!("Validation correctly failed: {}", e),
    }

    Ok(())
}
