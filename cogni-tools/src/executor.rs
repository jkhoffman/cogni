//! Tool executor trait and implementations

use crate::error::{Result, ToolError};
use cogni_core::{Tool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for async tool functions
pub type AsyncToolFunction = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync
>;

/// Type alias for sync tool functions
pub type SyncToolFunction = Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// Trait for executing tools
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool call
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult>;
    
    /// Get the tool definition
    fn tool(&self) -> &Tool;
    
    /// Validate arguments before execution (optional)
    async fn validate(&self, _args: &Value) -> Result<()> {
        Ok(())
    }
}

/// A function-based tool executor
pub struct FunctionExecutor {
    tool: Tool,
    pub(crate) func: AsyncToolFunction,
}

impl FunctionExecutor {
    /// Create a new function executor
    pub fn new(tool: Tool, func: AsyncToolFunction) -> Self {
        Self { tool, func }
    }
    
    /// Create from a synchronous function
    pub fn new_sync<F>(tool: Tool, func: F) -> Self
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        let func = Arc::new(func);
        let async_func: AsyncToolFunction = Arc::new(move |args| {
            let func = func.clone();
            Box::pin(async move { func(args) })
        });
        
        Self {
            tool,
            func: async_func,
        }
    }
}

#[async_trait]
impl ToolExecutor for FunctionExecutor {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        // Parse arguments
        let args: Value = serde_json::from_str(&call.arguments)
            .map_err(|e| ToolError::InvalidArguments {
                tool: call.name.clone(),
                message: format!("Failed to parse arguments: {}", e),
                source: Some(Box::new(e)),
            })?;
        
        // Validate arguments
        self.validate(&args).await?;
        
        // Execute the function
        match (self.func)(args).await {
            Ok(result) => {
                // Convert result to string - use to_string for compact JSON
                let content = result.to_string();
                
                Ok(ToolResult {
                    call_id: call.id.clone(),
                    content,
                    success: true,
                })
            }
            Err(e) => {
                // Return error as a tool result
                Ok(ToolResult {
                    call_id: call.id.clone(),
                    content: e.to_string(),
                    success: false,
                })
            }
        }
    }
    
    fn tool(&self) -> &Tool {
        &self.tool
    }
}

/// Builder for creating function executors
pub struct FunctionExecutorBuilder {
    name: String,
    description: String,
    parameters: Option<Value>,
    returns: Option<String>,
}

impl FunctionExecutorBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            parameters: None,
            returns: None,
        }
    }
    
    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
    
    /// Set the parameters schema
    pub fn parameters(mut self, params: Value) -> Self {
        self.parameters = Some(params);
        self
    }
    
    /// Set the return type description
    pub fn returns(mut self, returns: impl Into<String>) -> Self {
        self.returns = Some(returns.into());
        self
    }
    
    /// Build with an async function
    pub fn build_async<F, Fut>(self, func: F) -> FunctionExecutor
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value>> + Send + 'static,
    {
        let tool = Tool {
            name: self.name,
            description: self.description,
            function: cogni_core::Function {
                parameters: self.parameters.unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {}
                    })
                }),
                returns: self.returns,
            },
        };
        
        let async_func: AsyncToolFunction = Arc::new(move |args| {
            Box::pin(func(args))
        });
        
        FunctionExecutor::new(tool, async_func)
    }
    
    /// Build with a sync function
    pub fn build_sync<F>(self, func: F) -> FunctionExecutor
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        let tool = Tool {
            name: self.name,
            description: self.description,
            function: cogni_core::Function {
                parameters: self.parameters.unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {}
                    })
                }),
                returns: self.returns,
            },
        };
        
        FunctionExecutor::new_sync(tool, func)
    }
}