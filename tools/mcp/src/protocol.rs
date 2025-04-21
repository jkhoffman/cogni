//! MCP protocol handler implementation for Cogni.
//!
//! See TDD.md and https://modelcontextprotocol.io/docs/concepts/tools

use serde::{Deserialize, Serialize};

/// Specification for a tool, as described by the MCP protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub examples: Vec<serde_json::Value>,
}

/// A request to invoke a tool via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_name: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Content block for tool result, as per MCP convention.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    // Extend with more types as MCP evolves
}

/// The result of a tool invocation via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub tool_name: String,
    #[serde(default)]
    pub is_error: Option<bool>,
    #[serde(default)]
    pub content: Option<Vec<ContentBlock>>,
    /// Deprecated: use content/is_error instead
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Error envelope for MCP protocol errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub request_id: Option<String>,
}

pub struct MCPProtocolHandler;

impl Default for MCPProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPProtocolHandler {
    pub fn new() -> Self {
        MCPProtocolHandler
    }
}
