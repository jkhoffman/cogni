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
//!
//! Each module is feature-gated to allow for minimal builds when only
//! specific functionality is needed.

#[cfg(feature = "llm")]
pub mod llm;

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "prompt")]
pub mod prompt;

#[cfg(feature = "tool")]
pub mod tool;

#[cfg(feature = "chain")]
pub mod chain;

pub mod builder;

// Re-exports for convenience
#[cfg(feature = "llm")]
pub use llm::LanguageModel;

#[cfg(feature = "memory")]
pub use memory::MemoryStore;

#[cfg(feature = "prompt")]
pub use prompt::{PromptArgs, PromptTemplate};

#[cfg(feature = "tool")]
pub use tool::Tool;

#[cfg(feature = "chain")]
pub use chain::Chain;
