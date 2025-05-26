//! Tool/function calling types

/// A tool that can be called by the model
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// The function definition
    pub function: Function,
}

/// Function definition for a tool
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// JSON Schema for the parameters
    pub parameters: serde_json::Value,
    /// Whether this function returns a value
    pub returns: Option<String>,
}

/// How the model should use tools
#[derive(Debug, Clone, PartialEq)]
pub enum ToolChoice {
    /// Let the model decide
    Auto,
    /// Never call tools
    None,
    /// Must call a tool
    Required,
    /// Call a specific tool
    Specific(String),
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}

/// A tool call requested by the model
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    /// Unique ID for this call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// JSON-encoded arguments
    pub arguments: String,
}

/// Result from executing a tool
#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
    /// ID of the tool call this is responding to
    pub call_id: String,
    /// The result content
    pub content: String,
    /// Whether the tool execution succeeded
    pub success: bool,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            content: content.into(),
            success: true,
        }
    }

    /// Create a failed tool result
    pub fn error(call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            content: error.into(),
            success: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_creation() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: "Performs basic math operations".to_string(),
            function: Function {
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "a": { "type": "number" },
                        "b": { "type": "number" },
                        "operation": { "type": "string", "enum": ["add", "subtract", "multiply", "divide"] }
                    },
                    "required": ["a", "b", "operation"]
                }),
                returns: Some("number".to_string()),
            },
        };

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, "Performs basic math operations");
        assert!(tool.function.parameters.is_object());
        assert_eq!(tool.function.returns, Some("number".to_string()));
    }

    #[test]
    fn test_function_without_returns() {
        let function = Function {
            parameters: json!({"type": "object"}),
            returns: None,
        };

        assert!(function.returns.is_none());
    }

    #[test]
    fn test_tool_choice_variants() {
        let auto = ToolChoice::Auto;
        assert!(matches!(auto, ToolChoice::Auto));

        let none = ToolChoice::None;
        assert!(matches!(none, ToolChoice::None));

        let required = ToolChoice::Required;
        assert!(matches!(required, ToolChoice::Required));

        let specific = ToolChoice::Specific("my_tool".to_string());
        match specific {
            ToolChoice::Specific(name) => assert_eq!(name, "my_tool"),
            _ => panic!("Expected Specific variant"),
        }
    }

    #[test]
    fn test_tool_choice_default() {
        let choice = ToolChoice::default();
        assert!(matches!(choice, ToolChoice::Auto));
    }

    #[test]
    fn test_tool_call_creation() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "weather".to_string(),
            arguments: r#"{"location": "San Francisco", "unit": "celsius"}"#.to_string(),
        };

        assert_eq!(call.id, "call_123");
        assert_eq!(call.name, "weather");
        assert!(call.arguments.contains("San Francisco"));
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("call_123", "The weather is sunny");

        assert_eq!(result.call_id, "call_123");
        assert_eq!(result.content, "The weather is sunny");
        assert!(result.success);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("call_456", "Location not found");

        assert_eq!(result.call_id, "call_456");
        assert_eq!(result.content, "Location not found");
        assert!(!result.success);
    }

    #[test]
    fn test_tool_clone() {
        let tool = Tool {
            name: "test".to_string(),
            description: "Test tool".to_string(),
            function: Function {
                parameters: json!({}),
                returns: None,
            },
        };

        let cloned = tool.clone();
        assert_eq!(tool, cloned);
    }

    #[test]
    fn test_tool_choice_clone() {
        let choices = vec![
            ToolChoice::Auto,
            ToolChoice::None,
            ToolChoice::Required,
            ToolChoice::Specific("tool".to_string()),
        ];

        for choice in choices {
            let cloned = choice.clone();
            assert_eq!(choice, cloned);
        }
    }

    #[test]
    fn test_tool_call_clone() {
        let call = ToolCall {
            id: "id".to_string(),
            name: "name".to_string(),
            arguments: "args".to_string(),
        };

        let cloned = call.clone();
        assert_eq!(call, cloned);
    }

    #[test]
    fn test_tool_result_clone() {
        let result = ToolResult::success("id", "content");
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_debug_implementations() {
        let tool = Tool {
            name: "debug_test".to_string(),
            description: "Test".to_string(),
            function: Function {
                parameters: json!({}),
                returns: None,
            },
        };
        let debug_str = format!("{:?}", tool);
        assert!(debug_str.contains("debug_test"));

        let choice = ToolChoice::Required;
        let debug_str = format!("{:?}", choice);
        assert!(debug_str.contains("Required"));

        let call = ToolCall {
            id: "123".to_string(),
            name: "test".to_string(),
            arguments: "{}".to_string(),
        };
        let debug_str = format!("{:?}", call);
        assert!(debug_str.contains("123"));

        let result = ToolResult::success("123", "ok");
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("success: true"));
    }
}
