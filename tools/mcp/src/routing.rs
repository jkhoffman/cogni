//! MCP tool routing implementation for Cogni.
//!
//! See TDD.md and https://modelcontextprotocol.io/docs/concepts/tools

use crate::error::McpError;
use crate::protocol::{ContentBlock, ToolCall, ToolResult};
use async_trait::async_trait;
use cogni_core::traits::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MCPToolRouter {
    tools: HashMap<
        String,
        Arc<
            dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>
                + Send
                + Sync,
        >,
    >,
}

impl MCPToolRouter {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool<T>(&mut self, name: &str, tool: T)
    where
        T: Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>
            + Send
            + Sync
            + 'static,
    {
        self.tools.insert(name.to_string(), Arc::new(tool));
    }
}

impl MCPToolRouter {
    pub async fn handle_call(&self, call: ToolCall) -> Result<ToolResult, McpError> {
        let tool = self
            .tools
            .get(&call.tool_name)
            .ok_or_else(|| McpError::ToolNotFound(call.tool_name.clone()))?;
        let req_id = call.request_id.clone();
        match tool.invoke(call.input).await {
            Ok(output) => Ok(ToolResult {
                tool_name: call.tool_name,
                is_error: Some(false),
                content: Some(vec![ContentBlock::Text {
                    text: serde_json::to_string(&output)
                        .unwrap_or_else(|_| "<serialization error>".to_string()),
                }]),
                output: Some(output),
                request_id: req_id,
            }),
            Err(e) => Ok(ToolResult {
                tool_name: call.tool_name,
                is_error: Some(true),
                content: Some(vec![ContentBlock::Text {
                    text: format!("Error: {e}"),
                }]),
                output: None,
                request_id: req_id,
            }),
        }
    }
}
