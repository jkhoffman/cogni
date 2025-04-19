//! Tool interface for the Cogni framework.

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Debug;

use crate::error::ToolError;

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
#[async_trait]
pub trait Tool: Send + Sync {
    /// The type of input accepted by this tool
    type Input: DeserializeOwned + Send + Sync;

    /// The type of output returned by this tool
    type Output: Serialize + Send + Sync;

    /// Invoke the tool with the given input
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError>;

    /// Get the specification for this tool
    fn spec(&self) -> ToolSpec;
}
