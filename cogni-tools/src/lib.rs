//! Tool execution framework for the Cogni LLM library
//!
//! This crate provides a flexible system for defining and executing tools
//! that can be called by language models. It supports both synchronous and
//! asynchronous tools, automatic JSON schema generation, and integration
//! with various tool protocols including MCP (Model Context Protocol).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod executor;
pub mod registry;
pub mod error;
pub mod validation;
pub mod builtin;

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-export core types from cogni-core
pub use cogni_core::{Tool, ToolCall, ToolResult, ToolChoice, Function};

// Re-export main types
pub use executor::{ToolExecutor, AsyncToolFunction, SyncToolFunction};
pub use registry::ToolRegistry;
pub use error::{ToolError, ToolErrorKind};
pub use validation::ToolValidator;