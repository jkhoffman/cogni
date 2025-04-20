//! Core traits and types for the Cogni LLM orchestration framework.
//!
//! This crate provides the foundational traits and types that define the
//! interfaces for language models, tools, and memory storage in the Cogni framework.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod chain;
pub mod error;
pub mod traits;

// Re-export commonly used types
pub use chain::{Chain, ChainConfig, ChainError, ChainMetrics, ChainStep};
pub use error::Error;
pub use traits::{
    llm::{GenerateOptions, LanguageModel},
    memory::MemoryStore,
    prompt::{PromptArgs, PromptTemplate},
    tool::Tool,
};
