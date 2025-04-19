//! Core traits and types for the Cogni LLM orchestration framework.
//!
//! This crate provides the foundational traits and types that define the
//! interfaces for language models, tools, and memory storage in the Cogni framework.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod tool;

pub use error::Error;
pub use llm::LanguageModel;
pub use memory::MemoryStore;
pub use prompt::{MissingPlaceholderError, PromptArgs, PromptTemplate};
pub use tool::Tool;
