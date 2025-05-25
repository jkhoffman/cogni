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
pub use executor::{AsyncToolFunction, SyncToolFunction, ToolExecutor};
pub use registry::ToolRegistry;
pub use validation::ToolValidator;
