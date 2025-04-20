//! Core traits for the Cogni framework.
//!
//! This module contains all the core traits that define the interfaces
//! for language models, tools, memory storage, and prompts.

pub mod llm;
pub mod memory;
pub mod prompt;
pub mod tool;

pub use llm::LanguageModel;
pub use memory::MemoryStore;
pub use prompt::{PromptArgs, PromptTemplate};
pub use tool::Tool;
