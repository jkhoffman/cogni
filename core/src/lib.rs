//! Core traits and implementations for the Cogni framework.

pub mod agent;
pub mod chain;
pub mod error;
pub mod prompt;
pub mod traits;

pub use chain::{Chain, ChainConfig, ChainError, ChainMetrics, ChainStep, StepType};
pub use error::{AgentError, LlmError, ToolError};
pub use prompt::{PromptArgs, PromptError, PromptTemplate};
pub use traits::{
    agent::{Agent, AgentConfig, AgentInput, AgentOutput},
    llm::{GenerateOptions, LanguageModel},
    tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
};
