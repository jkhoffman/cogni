//! Core traits and implementations for the Cogni framework.

pub mod agent;
pub mod chain;
pub mod error;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod tool;
pub mod traits;

pub use chain::{Chain, ChainConfig, ChainError, ChainMetrics, ChainStep, SimpleChain, StepType};
pub use error::{AgentError, LlmError, MemoryError, ToolConfigError, ToolError};
pub use memory::{InMemoryMemory, MemoryEntry, MemoryStore, Role, SessionId};
pub use prompt::{PromptArgs, PromptError, PromptTemplate};
pub use tool::{SimpleTool, ToolCapability, ToolConfig, ToolExample, ToolSpec};
pub use traits::{
    agent::{Agent, AgentConfig, AgentInput, AgentOutput},
    chain::ChainExecutor,
    llm::{GenerateOptions, LanguageModel},
    memory::MemoryStore as MemoryStoreTrait,
    tool::{
        Tool, ToolCapability as ToolCapabilityTrait, ToolConfig as ToolConfigTrait,
        ToolSpec as ToolSpecTrait,
    },
};
