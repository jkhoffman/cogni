//! Prompt handling for the Cogni framework.

// Re-export everything from traits::prompt
pub use crate::traits::prompt::{MissingPlaceholderError, PromptArgs, PromptError, PromptTemplate};

// The rest of the implementation is now moved to traits::prompt.rs
