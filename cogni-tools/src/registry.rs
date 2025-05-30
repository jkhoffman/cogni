//! Tool registry for managing available tools

use crate::error::{Result, ToolError};
use crate::executor::ToolExecutor;
use cogni_core::{Tool, ToolCall, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a registry from a collection of executors
    pub async fn from_executors(
        executors: impl IntoIterator<Item = impl ToolExecutor + 'static>,
    ) -> Result<Self> {
        let registry = Self::new();
        registry.register(executors).await?;
        Ok(registry)
    }

    /// Register one or more tool executors
    pub async fn register(
        &self,
        executors: impl IntoIterator<Item = impl ToolExecutor + 'static>,
    ) -> Result<()> {
        let mut tools = self.tools.write().await;
        for executor in executors {
            let tool = executor.tool();
            let name = tool.name.clone();
            tools.insert(name, Arc::new(executor));
        }
        Ok(())
    }

    /// Get a tool by name (primarily for testing)
    #[doc(hidden)]
    pub async fn get(&self, name: &str) -> Option<Arc<dyn ToolExecutor>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Get all registered tools
    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        // Pre-allocate with known capacity
        let mut result = Vec::with_capacity(tools.len());
        for executor in tools.values() {
            result.push(executor.tool().clone());
        }
        result
    }

    /// Execute a tool call
    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        let tools = self.tools.read().await;

        let executor = tools.get(&call.name).ok_or_else(|| ToolError::NotFound {
            name: call.name.clone(),
        })?;

        // Clone the Arc to avoid holding the lock
        let executor = executor.clone();

        // Drop the read lock before executing
        drop(tools);

        executor.execute(call).await
    }

    /// Execute multiple tool calls in parallel
    pub async fn execute_many(&self, calls: &[ToolCall]) -> Vec<Result<ToolResult>> {
        use futures::future;

        let futures = calls.iter().map(|call| self.execute(call));
        future::join_all(futures).await
    }

    /// Remove a tool from the registry
    #[doc(hidden)]
    pub async fn remove(&self, name: &str) -> Option<Arc<dyn ToolExecutor>> {
        let mut tools = self.tools.write().await;
        tools.remove(name)
    }

    /// Check if a tool exists
    pub async fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// Get the number of registered tools
    pub async fn len(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Check if the registry is empty
    pub async fn is_empty(&self) -> bool {
        let tools = self.tools.read().await;
        tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::FunctionExecutorBuilder;
    use serde_json::json;

    #[tokio::test]
    async fn test_registry_basic_operations() {
        let registry = ToolRegistry::new();

        // Create a simple tool
        let tool = FunctionExecutorBuilder::new("test_tool")
            .description("A test tool")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }))
            .build_sync(|args| {
                let input = args
                    .get("input")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                Ok(json!({ "output": format!("Processed: {}", input) }))
            });

        // Register the tool
        registry.register([tool]).await.unwrap();

        // Check it exists
        assert!(registry.contains("test_tool").await);
        assert_eq!(registry.len().await, 1);

        // Execute it
        let call = ToolCall {
            id: "test-1".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"input": "hello"}"#.to_string(),
        };

        let result = registry.execute(&call).await.unwrap();
        assert_eq!(result.call_id, "test-1");
        assert!(result.success);

        // Remove it
        registry.remove("test_tool").await;
        assert!(!registry.contains("test_tool").await);
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_from_executors() {
        // Create multiple tools
        let tool1 = FunctionExecutorBuilder::new("tool1")
            .description("First tool")
            .build_sync(|_| Ok(json!({ "result": "tool1" })));

        let tool2 = FunctionExecutorBuilder::new("tool2")
            .description("Second tool")
            .build_sync(|_| Ok(json!({ "result": "tool2" })));

        let tool3 = FunctionExecutorBuilder::new("tool3")
            .description("Third tool")
            .build_sync(|_| Ok(json!({ "result": "tool3" })));

        // Create registry from executors using the macro
        let registry = ToolRegistry::from_executors(crate::tools_vec![tool1, tool2, tool3])
            .await
            .unwrap();

        // Verify all tools are registered
        assert_eq!(registry.len().await, 3);
        assert!(registry.contains("tool1").await);
        assert!(registry.contains("tool2").await);
        assert!(registry.contains("tool3").await);

        // Verify they work
        let call = ToolCall {
            id: "test-1".to_string(),
            name: "tool2".to_string(),
            arguments: "{}".to_string(),
        };

        let result = registry.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("tool2"));
    }
}
