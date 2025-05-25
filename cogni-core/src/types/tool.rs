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