//! Agent traits for the Cogni framework.
//!
//! This module defines the core traits for implementing agents that can:
//! 1. Process user inputs
//! 2. Select and use appropriate tools
//! 3. Leverage language models for reasoning
//! 4. Use memory for context and history
//! 5. Implement planning capabilities
//!
//! # Implementing an Agent
//!
//! To implement an agent:
//! 1. Define your input and output types that implement the required traits
//! 2. Implement the `Agent` trait for your type
//! 3. Provide a configuration type that implements `AgentConfig`
//! 4. Implement the required methods

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::{AgentError, LlmError, ToolConfigError};
use crate::traits::llm::LanguageModel;
use crate::traits::memory::MemoryStore;
use crate::traits::tool::Tool;

/// Configuration for an agent.
///
/// This trait should be implemented by types that provide configuration
/// for agents. It enables validation of configuration parameters before
/// an agent is initialized.
pub trait AgentConfig: Debug + Send + Sync {
    /// Validate the configuration.
    ///
    /// This method should check that all required fields are present and
    /// that their values are valid.
    ///
    /// # Returns
    /// - `Ok(())` if the configuration is valid
    /// - `Err(ToolConfigError)` describing the validation failure
    fn validate(&self) -> Result<(), ToolConfigError>;
}

/// Input to an agent, including the user's message and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    /// The primary input message or query
    pub message: String,

    /// A unique identifier for the conversation
    pub conversation_id: Option<String>,

    /// Additional context or parameters
    pub context: serde_json::Value,
}

/// Output from an agent, including the response and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// The primary response message
    pub message: String,

    /// Information about tools that were used
    pub tool_uses: Vec<ToolUse>,

    /// Additional data or results
    pub data: serde_json::Value,
}

/// Record of a tool being used during agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Name of the tool
    pub tool_name: String,

    /// Input provided to the tool
    pub input: serde_json::Value,

    /// Output received from the tool
    pub output: serde_json::Value,

    /// When the tool was used
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Tool selection strategy for an agent.
///
/// This trait is used by agents to select appropriate tools
/// based on the input and context.
#[async_trait]
pub trait ToolSelector: Send + Sync {
    /// Select appropriate tools for the given input.
    ///
    /// # Arguments
    /// * `input` - The input to the agent
    /// * `context` - Additional context for tool selection
    ///
    /// # Returns
    /// A vector of tool names to consider using
    async fn select_tools(
        &self,
        input: &str,
        context: &serde_json::Value,
    ) -> Result<Vec<String>, AgentError>;
}

/// Core trait for agent implementations.
///
/// Agents are responsible for processing user inputs, reasoning about them,
/// selecting and using appropriate tools, and producing responses.
#[async_trait]
pub trait Agent: Send + Sync {
    /// The type of configuration used by this agent
    type Config: AgentConfig;

    /// The type of input this agent accepts
    type Input: Send + 'static;

    /// The type of output this agent produces
    type Output: Send + 'static;

    /// Try to create a new instance of the agent with the given configuration.
    ///
    /// This is used by the builder pattern to construct agent instances.
    /// Implementations should validate the configuration and perform any
    /// necessary setup that doesn't require async operations.
    ///
    /// # Errors
    /// Returns `ToolConfigError` if creation fails.
    fn try_new(config: Self::Config) -> Result<Self, ToolConfigError>
    where
        Self: Sized;

    /// Initialize the agent.
    ///
    /// This method is called after the agent is created but before it is used.
    /// Use this to set up any necessary resources.
    ///
    /// # Errors
    /// Returns `AgentError` if initialization fails.
    async fn initialize(&mut self) -> Result<(), AgentError>;

    /// Shut down the agent.
    ///
    /// This method is called when the agent is being disposed of.
    /// Use this to clean up any resources.
    ///
    /// # Errors
    /// Returns `AgentError` if shutdown fails.
    async fn shutdown(&mut self) -> Result<(), AgentError>;

    /// Execute the agent with the given input.
    ///
    /// This method is used by the Chain system to execute the agent as a step.
    ///
    /// # Arguments
    /// * `input` - The input to the agent
    ///
    /// # Returns
    /// The output from the agent, or an error
    async fn execute(&self, input: Self::Input) -> Result<Self::Output, AgentError>;

    /// Process an input and generate a response.
    ///
    /// This is the main method that implements the agent's functionality.
    ///
    /// # Arguments
    /// * `input` - The input to process
    /// * `llm` - The language model to use
    /// * `tools` - The available tools, represented as name-to-tool mapping
    /// * `memory` - The memory store to use
    ///
    /// # Returns
    /// Returns `Result<AgentOutput, AgentError>` containing either:
    /// - The agent's output on success
    /// - An `AgentError` on failure
    async fn process(
        &self,
        input: AgentInput,
        llm: Arc<
            dyn LanguageModel<
                Prompt = String,
                Response = String,
                TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>,
            >,
        >,
        tools: &std::collections::HashMap<
            String,
            Arc<dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>>,
        >,
        memory: Arc<dyn MemoryStore>,
    ) -> Result<AgentOutput, AgentError>;

    /// Generate a plan for processing an input.
    ///
    /// This method is used to create a structured plan for how the agent
    /// will approach solving a problem or answering a question.
    ///
    /// # Arguments
    /// * `input` - The input to process
    /// * `llm` - The language model to use
    /// * `memory` - The memory store to use
    ///
    /// # Returns
    /// Returns a plan as a structured JSON object or an error
    async fn plan(
        &self,
        input: &str,
        llm: Arc<
            dyn LanguageModel<
                Prompt = String,
                Response = String,
                TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>,
            >,
        >,
        memory: Arc<dyn MemoryStore>,
    ) -> Result<serde_json::Value, AgentError>;
}

/// Type alias for a reference-counted agent.
pub type AgentArc =
    Arc<dyn Agent<Config = (), Input = serde_json::Value, Output = serde_json::Value>>;

/// Implementation of AgentConfig for () to make it usable as a default config
impl AgentConfig for () {
    fn validate(&self) -> Result<(), ToolConfigError> {
        Ok(())
    }
}
