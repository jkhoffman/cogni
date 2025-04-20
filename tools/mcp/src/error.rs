//! MCP error types for Cogni.
//!
//! See TDD.md and https://modelcontextprotocol.io/docs/concepts/tools

use cogni_core::error::ToolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Tool invocation failed: {0}")]
    InvocationFailed(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<McpError> for ToolError {
    fn from(e: McpError) -> Self {
        match e {
            McpError::Transport(msg)
            | McpError::Protocol(msg)
            | McpError::ToolNotFound(msg)
            | McpError::InvocationFailed(msg)
            | McpError::Serialization(msg) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("MCP", "mcp_error"),
                message: msg,
                retryable: false,
            },
        }
    }
}
