//! Error types for the Cogni framework.

use thiserror::Error;

/// The main error type for the Cogni framework.
#[derive(Error, Debug)]
pub enum Error {
    /// Errors from language model operations
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// Errors from tool operations
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// Errors from memory operations
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    /// Errors from chain execution
    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),
}

/// Error type for language model operations
#[derive(Debug, Error)]
pub enum LlmError {
    /// Error making HTTP request
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// Error parsing response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Error with configuration
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// API returned an error
    #[error("API error: {0}")]
    ApiError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimit,
}

/// Errors that can occur during tool operations.
#[derive(Error, Debug)]
pub enum ToolError {
    /// The tool execution failed
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid input provided to the tool
    #[error("Invalid tool input: {0}")]
    InvalidInput(String),

    /// Tool timed out
    #[error("Tool timed out after {0:?}")]
    Timeout(std::time::Duration),
}

/// Errors that can occur during memory operations.
#[derive(Error, Debug)]
pub enum MemoryError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
}

/// Errors that can occur during chain execution.
#[derive(Error, Debug)]
pub enum ChainError {
    /// A step in the chain failed
    #[error("Chain step failed: {0}")]
    StepFailed(String),

    /// Chain execution was cancelled
    #[error("Chain execution cancelled")]
    Cancelled,

    /// Chain timeout
    #[error("Chain timed out after {0:?}")]
    Timeout(std::time::Duration),
}
