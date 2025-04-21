//! Core traits for the Cogni framework.
//!
//! This module contains all the core traits that define the interfaces
//! for language models, tools, memory storage, and prompts. Each trait
//! is organized into its own submodule for better maintainability and
//! separation of concerns.
//!
//! # Module Organization
//!
//! - `llm`: Language model traits and types
//! - `memory`: Memory storage traits and types
//! - `prompt`: Prompt template traits and types
//! - `tool`: Tool traits and types
//! - `chain`: Chain execution traits and types
//! - `builder`: Builder traits for constructing components
//! - `agent`: Agent traits and types
//!
//! Each module is feature-gated to allow for minimal builds when only
//! specific functionality is needed.

pub mod agent;
pub mod builder;
pub mod chain;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod tool;

// Re-export commonly used traits and types
pub use agent::{Agent, AgentConfig, AgentInput, AgentOutput, ToolSelector};
pub use llm::{GenerateOptions, LanguageModel};
pub use memory::MemoryStore;
pub use prompt::{PromptArgs, PromptTemplate};
pub use tool::{Tool, ToolCapability, ToolConfig, ToolSpec};
