//! Core types used throughout the Cogni library

pub mod message;
pub mod request;
pub mod response;
pub mod stream;
pub mod tool;

// Common type aliases
/// A model identifier (e.g., "gpt-4", "claude-3")
pub type ModelId = String;

/// A conversation or request ID
pub type ConversationId = String;