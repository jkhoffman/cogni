//! Built-in tools for common operations

use crate::executor::{FunctionExecutor, FunctionExecutorBuilder};
use crate::error::Result;
use crate::registry::RegistryBuilder;
use crate::validation::param_schema;
use serde_json::{json, Value};

/// Create a calculator tool
pub fn calculator() -> FunctionExecutor {
    FunctionExecutorBuilder::new("calculator")
        .description("Perform basic arithmetic operations")
        .parameters(param_schema()
            .string_required("operation", "The operation to perform: add, subtract, multiply, divide")
            .number("a", "First operand")
            .number("b", "Second operand")
            .build())
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
        .parameters(param_schema()
            .string_required("operation", "The operation: uppercase, lowercase, reverse, length, contains, replace")
            .string_required("text", "The text to operate on")
            .string("search", "Search string (for contains and replace operations)")
            .string("replacement", "Replacement string (for replace operation)")
            .build())
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
        .parameters(param_schema()
            .string_required("operation", "The operation: parse, stringify, get_field, set_field")
            .string("json_string", "JSON string to parse")
            .additional_properties(true) // Allow additional fields for flexible operations
            .build())
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
                                    return Ok(json!({ "error": format!("Field '{}' not found", part) }));
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
            .parameters(param_schema()
                .string_required("operation", "Operation: sqrt, pow, log, sin, cos, tan, abs, round, ceil, floor")
                .number("value", "The value to operate on")
                .number("n", "Second parameter (for pow: exponent, for round: decimal places)")
                .build())
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
    
    #[test]
    fn test_calculator() {
        let calc = calculator();
        
        // Test addition
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "add",
                "a": 5,
                "b": 3
            });
            (calc.func)(args).await
        }).unwrap();
        
        assert_eq!(result["result"], 8.0);
    }
    
    #[test]
    fn test_string_tools() {
        let string_tool = string_tools();
        
        // Test uppercase
        let result = tokio_test::block_on(async {
            let args = json!({
                "operation": "uppercase",
                "text": "hello world"
            });
            (string_tool.func)(args).await
        }).unwrap();
        
        assert_eq!(result["result"], "HELLO WORLD");
    }
}