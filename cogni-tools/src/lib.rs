//! Tool execution framework for the Cogni LLM library
//!
//! This crate provides a flexible system for defining and executing tools
//! that can be called by language models. It supports both synchronous and
//! asynchronous tools, automatic JSON schema generation, and integration
//! with various tool protocols including MCP (Model Context Protocol).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod builtin;
pub mod error;
pub mod executor;
pub mod registry;
pub mod validation;

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-export core types from cogni-core
pub use cogni_core::{Function, Tool, ToolCall, ToolChoice, ToolResult};

// Re-export main types
pub use error::{ToolError, ToolErrorKind};
pub use executor::{
    AsyncToolFunction, FunctionExecutor, FunctionExecutorBuilder, SyncToolFunction, ToolExecutor,
};
pub use registry::ToolRegistry;
pub use validation::ToolValidator;

/// Macro for creating a vector of boxed tool executors
///
/// This macro simplifies the creation of a vector of `Box<dyn ToolExecutor>`
/// from a list of tool executors, avoiding the need for explicit boxing and type annotations.
///
/// # Examples
///
/// ```
/// # use cogni_tools::{tools_vec, FunctionExecutorBuilder, ToolRegistry};
/// # use serde_json::json;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let tool1 = FunctionExecutorBuilder::new("tool1")
///     .description("First tool")
///     .build_sync(|_| Ok(json!({ "result": "tool1" })));
///
/// let tool2 = FunctionExecutorBuilder::new("tool2")
///     .description("Second tool")
///     .build_sync(|_| Ok(json!({ "result": "tool2" })));
///
/// // Much cleaner than vec![Box::new(tool1) as Box<dyn ToolExecutor>, ...]
/// let tools = tools_vec![tool1, tool2];
/// let registry = ToolRegistry::from_executors(tools).await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! tools_vec {
    ($($tool:expr),* $(,)?) => {
        vec![
            $(Box::new($tool) as Box<dyn $crate::ToolExecutor>),*
        ]
    };
}
