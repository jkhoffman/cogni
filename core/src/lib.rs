//! Core traits and implementations for the Cogni framework.

pub mod chain;
pub mod error;
pub mod prompt;
pub mod traits;

pub use chain::{Chain, ChainConfig, ChainError, ChainMetrics, ChainStep, StepType};
pub use error::{LlmError, ToolError};
pub use prompt::{PromptArgs, PromptError, PromptTemplate};
pub use traits::{
    llm::{GenerateOptions, LanguageModel},
    tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
};
