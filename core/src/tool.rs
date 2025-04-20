//! Tool interface for the Cogni framework.
//!
//! This module provides the core trait and types for implementing tools that can be used by agents.
//! Tools are self-contained units of functionality that can be invoked by agents to perform specific
//! tasks, such as mathematical computations, code execution, or web searches.
//!
//! # Tool Lifecycle
//!
//! Tools follow a defined lifecycle:
//! 1. Creation - Tool is instantiated with its configuration
//! 2. Initialization - Tool performs any necessary setup (e.g., connecting to services)
//! 3. Operation - Tool is ready to handle invocations
//! 4. Shutdown - Tool performs cleanup (e.g., closing connections)
//!
//! # Implementing a Tool
//!
//! To implement a tool:
//! 1. Define your input and output types that implement the required traits
//! 2. Implement the `Tool` trait for your type
//! 3. Provide a configuration type that implements `ToolConfig`
//! 4. Implement the required methods
//!
//! ```rust,no_run
//! use cogni_core::tool::{Tool, ToolSpec, ToolConfig, ToolCapability};
//! use serde::{Serialize, Deserialize};
//! use async_trait::async_trait;
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct MyToolConfig {
//!     api_key: String,
//!     timeout: u64,
//! }
//!
//! impl ToolConfig for MyToolConfig {
//!     fn validate(&self) -> Result<(), String> {
//!         if self.api_key.is_empty() {
//!             return Err("API key cannot be empty".into());
//!         }
//!         if self.timeout == 0 {
//!             return Err("Timeout must be greater than 0".into());
//!         }
//!         Ok(())
//!     }
//! }
//!
//! struct MyTool {
//!     config: MyToolConfig,
//!     client: Option<reqwest::Client>,
//! }
//!
//! #[async_trait]
//! impl Tool for MyTool {
//!     type Input = String;
//!     type Output = String;
//!     type Config = MyToolConfig;
//!
//!     async fn initialize(&mut self) -> Result<(), crate::error::ToolError> {
//!         self.client = Some(reqwest::Client::new());
//!         Ok(())
//!     }
//!
//!     async fn shutdown(&mut self) -> Result<(), crate::error::ToolError> {
//!         self.client = None;
//!         Ok(())
//!     }
//!
//!     fn capabilities(&self) -> Vec<ToolCapability> {
//!         vec![
//!             ToolCapability::Stateless,
//!             ToolCapability::ThreadSafe,
//!             ToolCapability::NetworkAccess,
//!         ]
//!     }
//!
//!     // ... other required methods ...
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Debug;

use crate::error::ToolError;

/// Configuration for a tool.
///
/// This trait should be implemented by types that provide configuration
/// for tools. It enables validation of configuration parameters before
/// a tool is initialized.
pub trait ToolConfig: Debug + Send + Sync {
    /// Validate the configuration.
    ///
    /// This method should check that all required fields are present and
    /// that their values are valid.
    ///
    /// # Returns
    /// - `Ok(())` if the configuration is valid
    /// - `Err(String)` with a description of what is invalid
    fn validate(&self) -> Result<(), String>;
}

/// Capabilities that a tool can declare.
///
/// Tools use these to indicate their characteristics and requirements
/// to the framework. This helps with resource allocation and scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCapability {
    /// Tool maintains no internal state between invocations
    Stateless,
    /// Tool can be safely used from multiple threads
    ThreadSafe,
    /// Tool requires network access
    NetworkAccess,
    /// Tool performs file system operations
    FileSystem,
    /// Tool requires significant CPU resources
    CpuIntensive,
    /// Tool requires significant memory
    MemoryIntensive,
    /// Tool performs cryptographic operations
    Cryptographic,
    /// Tool requires GPU access
    GpuAccess,
}

/// Specification for a tool, including its name, description, and schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// The name of the tool
    pub name: String,

    /// A description of what the tool does
    pub description: String,

    /// JSON schema for the tool's input
    pub input_schema: serde_json::Value,

    /// JSON schema for the tool's output
    pub output_schema: serde_json::Value,

    /// Example uses of the tool
    pub examples: Vec<ToolExample>,
}

/// An example use of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    /// Description of this example
    pub description: String,

    /// Example input
    pub input: serde_json::Value,

    /// Example output
    pub output: serde_json::Value,
}

/// A trait representing a tool that can be invoked by an agent.
///
/// Tools are the primary way for agents to interact with external systems
/// and perform specific tasks. Each tool should be focused on a specific
/// capability (e.g., mathematical computation, code execution, web search).
///
/// # Type Parameters
/// - `Input`: The type of input the tool accepts
/// - `Output`: The type of output the tool produces
/// - `Config`: The type containing tool configuration
///
/// # Thread Safety
/// Tools must be `Send + Sync` to support concurrent usage. If your tool
/// maintains state, ensure it is properly synchronized.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The type of input accepted by this tool
    type Input: DeserializeOwned + Send + Sync;

    /// The type of output returned by this tool
    type Output: Serialize + Send + Sync;

    /// The type containing tool configuration
    type Config: ToolConfig;

    /// Initialize the tool.
    ///
    /// This method is called after the tool is created but before it is used.
    /// Use this to set up any necessary resources (e.g., network connections).
    ///
    /// # Errors
    /// Returns `ToolError` if initialization fails.
    async fn initialize(&mut self) -> Result<(), ToolError> {
        Ok(()) // Default no-op implementation
    }

    /// Shut down the tool.
    ///
    /// This method is called when the tool is being disposed of.
    /// Use this to clean up any resources (e.g., close connections).
    ///
    /// # Errors
    /// Returns `ToolError` if shutdown fails.
    async fn shutdown(&mut self) -> Result<(), ToolError> {
        Ok(()) // Default no-op implementation
    }

    /// Get the capabilities of this tool.
    ///
    /// This method should return a list of capabilities that describe
    /// the tool's characteristics and requirements.
    fn capabilities(&self) -> Vec<ToolCapability>;

    /// Invoke the tool with the given input
    ///
    /// This is the main method that implements the tool's functionality.
    /// It should be idempotent if possible.
    ///
    /// # Arguments
    /// * `input` - The input to the tool, of type `Self::Input`
    ///
    /// # Returns
    /// Returns `Result<Self::Output, ToolError>` containing either:
    /// - The tool's output on success
    /// - A `ToolError` on failure
    ///
    /// # Errors
    /// May return `ToolError` if:
    /// - The input is invalid
    /// - The tool encounters an internal error
    /// - The tool times out
    /// - Required resources are unavailable
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError>;

    /// Get the specification for this tool
    ///
    /// This method should return a `ToolSpec` that describes:
    /// - The tool's name and description
    /// - The schema for its input and output
    /// - Example uses of the tool
    fn spec(&self) -> ToolSpec;
}
