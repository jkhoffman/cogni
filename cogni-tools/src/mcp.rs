//! Model Context Protocol (MCP) support
//!
//! This module will provide integration with MCP servers
//! for tool execution. Currently a placeholder.

use crate::error::Result;

/// MCP transport trait (placeholder)
#[allow(async_fn_in_trait)]
pub trait McpTransport: Send + Sync {
    /// Send a request to the MCP server
    async fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

// TODO: Implement HTTP and stdio transports
// TODO: Implement MCP tool discovery and execution