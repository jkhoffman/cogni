//! Built-in tools for common operations

use crate::error::Result;
use crate::executor::{FunctionExecutor, FunctionExecutorBuilder};
use crate::registry::RegistryBuilder;
use crate::validation::param_schema;
use serde_json::{json, Value};

/// Create a calculator tool
pub fn calculator() -> FunctionExecutor {
    FunctionExecutorBuilder::new("calculator")
        .description("Perform basic arithmetic operations")
        .parameters(
            param_schema()
                .string_required(
                    "operation",
                    "The operation to perform: add, subtract, multiply, divide",
                )
                .number("a", "First operand")
                .number("b", "Second operand")
                .build(),
        )
        .returns("The result of the arithmetic operation")
        .build_sync(|args| {
            let operation = args["operation"].as_str().unwrap_or("");
            let a = args["a"].as_f64().unwrap_or(0.0);
            let b = args["b"].as_f64().unwrap_or(0.0);

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

/// Create a string manipulation tool
pub fn string_tools() -> FunctionExecutor {
    FunctionExecutorBuilder::new("string_tools")
        .description("Perform string manipulation operations")
        .parameters(
            param_schema()
                .string_required(
                    "operation",
                    "The operation: uppercase, lowercase, reverse, length, contains, replace",
                )
                .string_required("text", "The text to operate on")
                .string(
                    "search",
                    "Search string (for contains and replace operations)",
                )
                .string("replacement", "Replacement string (for replace operation)")
                .build(),
        )
        .returns("The result of the string operation")
        .build_sync(|args| {
            let operation = args["operation"].as_str().unwrap_or("");
            let text = args["text"].as_str().unwrap_or("");

            let result = match operation {
                "uppercase" => json!({ "result": text.to_uppercase() }),
                "lowercase" => json!({ "result": text.to_lowercase() }),
                "reverse" => json!({ "result": text.chars().rev().collect::<String>() }),
                "length" => json!({ "result": text.len() }),
                "contains" => {
                    let search = args["search"].as_str().unwrap_or("");
                    json!({ "result": text.contains(search) })
                }
                "replace" => {
                    let search = args["search"].as_str().unwrap_or("");
                    let replacement = args["replacement"].as_str().unwrap_or("");
                    json!({ "result": text.replace(search, replacement) })
                }
                _ => json!({ "error": "Unknown operation" }),
            };

            Ok(result)
        })
}

/// Create a JSON manipulation tool
pub fn json_tools() -> FunctionExecutor {
    FunctionExecutorBuilder::new("json_tools")
        .description("Parse, query, and manipulate JSON data")
        .parameters(
            param_schema()
                .string_required(
                    "operation",
                    "The operation: parse, stringify, get_field, set_field",
                )
                .string("json_string", "JSON string to parse")
                .additional_properties(true) // Allow additional fields for flexible operations
                .build(),
        )
        .returns("The result of the JSON operation")
        .build_sync(|args| {
            let operation = args["operation"].as_str().unwrap_or("");

            match operation {
                "parse" => {
                    let json_string = args["json_string"].as_str().unwrap_or("{}");
                    match serde_json::from_str::<Value>(json_string) {
                        Ok(parsed) => Ok(json!({ "result": parsed })),
                        Err(e) => Ok(json!({ "error": format!("Parse error: {}", e) })),
                    }
                }
                "stringify" => {
                    if let Some(data) = args.get("data") {
                        match serde_json::to_string_pretty(data) {
                            Ok(json_string) => Ok(json!({ "result": json_string })),
                            Err(e) => Ok(json!({ "error": format!("Stringify error: {}", e) })),
                        }
                    } else {
                        Ok(json!({ "error": "No data field provided" }))
                    }
                }
                "get_field" => {
                    if let (Some(data), Some(path)) = (args.get("data"), args["path"].as_str()) {
                        let parts: Vec<&str> = path.split('.').collect();
                        let mut current = data;

                        for part in parts {
                            if let Some(obj) = current.as_object() {
                                if let Some(value) = obj.get(part) {
                                    current = value;
                                } else {
                                    return Ok(
                                        json!({ "error": format!("Field '{}' not found", part) }),
                                    );
                                }
                            } else {
                                return Ok(json!({ "error": "Not an object" }));
                            }
                        }

                        Ok(json!({ "result": current }))
                    } else {
                        Ok(json!({ "error": "Missing data or path" }))
                    }
                }
                _ => Ok(json!({ "error": "Unknown operation" })),
            }
        })
}

/// Create a collection of math tools
pub fn math_tools() -> Vec<FunctionExecutor> {
    vec![
        // Basic calculator
        calculator(),
        // Advanced math functions
        FunctionExecutorBuilder::new("math_advanced")
            .description("Advanced mathematical operations")
            .parameters(
                param_schema()
                    .string_required(
                        "operation",
                        "Operation: sqrt, pow, log, sin, cos, tan, abs, round, ceil, floor",
                    )
                    .number("value", "The value to operate on")
                    .number(
                        "n",
                        "Second parameter (for pow: exponent, for round: decimal places)",
                    )
                    .build(),
            )
            .returns("The result of the mathematical operation")
            .build_sync(|args| {
                let operation = args["operation"].as_str().unwrap_or("");
                let value = args["value"].as_f64().unwrap_or(0.0);

                let result = match operation {
                    "sqrt" => value.sqrt(),
                    "pow" => {
                        let n = args["n"].as_f64().unwrap_or(2.0);
                        value.powf(n)
                    }
                    "log" => value.ln(),
                    "sin" => value.sin(),
                    "cos" => value.cos(),
                    "tan" => value.tan(),
                    "abs" => value.abs(),
                    "round" => {
                        let decimals = args["n"].as_u64().unwrap_or(0) as i32;
                        let multiplier = 10f64.powi(decimals);
                        (value * multiplier).round() / multiplier
                    }
                    "ceil" => value.ceil(),
                    "floor" => value.floor(),
                    _ => return Ok(json!({ "error": "Unknown operation" })),
                };

                Ok(json!({ "result": result }))
            }),
    ]
}

/// Create a registry with all built-in tools
pub async fn create_builtin_registry() -> Result<crate::registry::ToolRegistry> {
    let mut builder = RegistryBuilder::new()
        .with_tool(calculator())
        .with_tool(string_tools())
        .with_tool(json_tools());

    for tool in math_tools() {
        builder = builder.with_tool(tool);
    }

    builder.build().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ToolExecutor;
    use cogni_core::ToolCall;

    #[test]
    fn test_calculator_operations() {
        let calc = calculator();

        // Test addition
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "add",
                "a": 5,
                "b": 3
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 8.0);

        // Test subtraction
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "subtract",
                "a": 10,
                "b": 4
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 6.0);

        // Test multiplication
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "multiply",
                "a": 7,
                "b": 6
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 42.0);

        // Test division
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "divide",
                "a": 15,
                "b": 3
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 5.0);

        // Test division by zero
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "divide",
                "a": 10,
                "b": 0
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Division by zero");

        // Test unknown operation
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "modulo",
                "a": 10,
                "b": 3
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Unknown operation");

        // Test missing parameters (defaults to 0)
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "add"
            });
            (calc.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 0.0);
    }

    #[test]
    fn test_calculator_metadata() {
        let calc = calculator();
        let tool = calc.tool();
        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, "Perform basic arithmetic operations");
        assert_eq!(tool.function.returns.as_ref().unwrap(), "The result of the arithmetic operation");
    }

    #[test]
    fn test_string_tools_operations() {
        let string_tool = string_tools();

        // Test uppercase
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "uppercase",
                "text": "hello world"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], "HELLO WORLD");

        // Test lowercase
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "lowercase",
                "text": "HELLO WORLD"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], "hello world");

        // Test reverse
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "reverse",
                "text": "hello"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], "olleh");

        // Test length
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "length",
                "text": "hello world"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 11);

        // Test contains
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "contains",
                "text": "hello world",
                "search": "world"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], true);

        // Test contains - not found
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "contains",
                "text": "hello world",
                "search": "foo"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], false);

        // Test replace
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "replace",
                "text": "hello world",
                "search": "world",
                "replacement": "rust"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], "hello rust");

        // Test unknown operation
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "split",
                "text": "hello world"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Unknown operation");

        // Test empty text
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "uppercase"
            });
            (string_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], "");
    }

    #[test]
    fn test_string_tools_metadata() {
        let string_tool = string_tools();
        let tool = string_tool.tool();
        assert_eq!(tool.name, "string_tools");
        assert_eq!(tool.description, "Perform string manipulation operations");
        assert_eq!(tool.function.returns.as_ref().unwrap(), "The result of the string operation");
    }

    #[test]
    fn test_json_tools_operations() {
        let json_tool = json_tools();

        // Test parse
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "parse",
                "json_string": r#"{"name": "John", "age": 30}"#
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"]["name"], "John");
        assert_eq!(result["result"]["age"], 30);

        // Test parse error
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "parse",
                "json_string": "invalid json"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert!(result["error"].as_str().unwrap().contains("Parse error"));

        // Test parse with empty string (defaults to {})
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "parse"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], json!({}));

        // Test stringify
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "stringify",
                "data": {"name": "John", "age": 30}
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        let stringified = result["result"].as_str().unwrap();
        assert!(stringified.contains("\"name\": \"John\""));
        assert!(stringified.contains("\"age\": 30"));

        // Test stringify - no data
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "stringify"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "No data field provided");

        // Test get_field
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "get_field",
                "data": {"user": {"name": "John", "details": {"age": 30}}},
                "path": "user.details.age"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 30);

        // Test get_field - field not found
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "get_field",
                "data": {"user": {"name": "John"}},
                "path": "user.age"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Field 'age' not found");

        // Test get_field - not an object
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "get_field",
                "data": {"values": [1, 2, 3]},
                "path": "values.first"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Not an object");

        // Test get_field - missing data or path
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "get_field",
                "path": "user.name"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Missing data or path");

        // Test unknown operation
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "merge"
            });
            (json_tool.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Unknown operation");
    }

    #[test]
    fn test_json_tools_metadata() {
        let json_tool = json_tools();
        let tool = json_tool.tool();
        assert_eq!(tool.name, "json_tools");
        assert_eq!(tool.description, "Parse, query, and manipulate JSON data");
        assert_eq!(tool.function.returns.as_ref().unwrap(), "The result of the JSON operation");
        
        // Test that additional_properties is allowed
        let params = &tool.function.parameters;
        assert!(params["additionalProperties"].as_bool().unwrap_or(false));
    }

    #[test]
    fn test_math_advanced_operations() {
        let tools = math_tools();
        assert!(tools.len() >= 2); // At least calculator and math_advanced
        
        let math_advanced = tools.iter().find(|t| t.tool().name == "math_advanced").unwrap();

        // Test sqrt
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "sqrt",
                "value": 16
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 4.0);

        // Test pow
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "pow",
                "value": 2,
                "n": 3
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 8.0);

        // Test pow with default n=2
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "pow",
                "value": 5
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 25.0);

        // Test log (natural logarithm)
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "log",
                "value": std::f64::consts::E
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert!((result["result"].as_f64().unwrap() - 1.0).abs() < 0.0001);

        // Test sin
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "sin",
                "value": std::f64::consts::PI / 2.0
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert!((result["result"].as_f64().unwrap() - 1.0).abs() < 0.0001);

        // Test cos
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "cos",
                "value": 0.0
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 1.0);

        // Test tan
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "tan",
                "value": 0.0
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 0.0);

        // Test abs
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "abs",
                "value": -42.5
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 42.5);

        // Test round with decimals
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "round",
                "value": 3.14159,
                "n": 2
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 3.14);

        // Test round without decimals
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "round",
                "value": 3.7
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 4.0);

        // Test ceil
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "ceil",
                "value": 3.1
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 4.0);

        // Test floor
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "floor",
                "value": 3.9
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 3.0);

        // Test unknown operation
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "factorial",
                "value": 5
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["error"], "Unknown operation");

        // Test with missing value (defaults to 0)
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "abs"
            });
            (math_advanced.func)(args).await
        })
        .unwrap();
        assert_eq!(result["result"], 0.0);
    }

    #[test]
    fn test_math_advanced_metadata() {
        let tools = math_tools();
        let math_advanced = tools.iter().find(|t| t.tool().name == "math_advanced").unwrap();
        let tool = math_advanced.tool();
        
        assert_eq!(tool.name, "math_advanced");
        assert_eq!(tool.description, "Advanced mathematical operations");
        assert_eq!(tool.function.returns.as_ref().unwrap(), "The result of the mathematical operation");
    }

    #[tokio::test]
    async fn test_create_builtin_registry() {
        let registry = create_builtin_registry().await.unwrap();
        
        // Check that all expected tools are registered
        assert!(registry.get("calculator").await.is_some());
        assert!(registry.get("string_tools").await.is_some());
        assert!(registry.get("json_tools").await.is_some());
        assert!(registry.get("math_advanced").await.is_some());
        
        // Verify tool count
        let tools = registry.list_tools().await;
        assert!(tools.len() >= 4); // At least the 4 main tools
        
        // Test that we can execute a tool from the registry
        let calc = registry.get("calculator").await.unwrap();
        let call = ToolCall {
            id: "test".to_string(),
            name: "calculator".to_string(),
            arguments: json!({
                "operation": "multiply",
                "a": 6,
                "b": 7
            }).to_string(),
        };
        let result = calc.execute(&call).await.unwrap();
        let result_json: Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(result_json["result"], 42.0);
    }

    #[test]
    fn test_math_tools_returns_multiple_tools() {
        let tools = math_tools();
        
        // Should have at least calculator and math_advanced
        assert!(tools.len() >= 2);
        
        // Check calculator is included
        assert!(tools.iter().any(|t| t.tool().name == "calculator"));
        
        // Check math_advanced is included
        assert!(tools.iter().any(|t| t.tool().name == "math_advanced"));
        
        // All tools should be properly configured
        for executor in &tools {
            let tool = executor.tool();
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.function.returns.is_some());
        }
    }
}
