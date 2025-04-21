//! Error types for the Cogni framework.
//!
//! This module provides a comprehensive error handling system with:
//! - Specific error types for different operations
//! - Error context for better debugging
//! - Retry policies for transient failures
//! - Error reporting interfaces

use std::{
    collections::HashMap,
    fmt::{self, Display},
    time::Duration,
};
use thiserror::Error;
use time::OffsetDateTime;

/// Error context containing additional information about an error.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The source of the error (e.g., component name)
    pub source: String,
    /// The operation being performed
    pub operation: String,
    /// Timestamp when the error occurred
    pub timestamp: time::OffsetDateTime,
    /// Additional metadata about the error
    pub metadata: HashMap<String, String>,
    pub message: String,
    pub details: Option<String>,
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}::{} at {}]",
            self.source,
            self.operation,
            self.timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        )?;
        if !self.metadata.is_empty() {
            write!(f, " {{")?;
            for (i, (key, value)) in self.metadata.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}={}", key, value)?;
            }
            write!(f, "}}")?;
        }
        Ok(())
    }
}

impl ErrorContext {
    /// Create a new error context.
    pub fn new(source: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            operation: operation.into(),
            timestamp: time::OffsetDateTime::now_utc(),
            metadata: HashMap::new(),
            message: String::new(),
            details: None,
        }
    }

    /// Add metadata to the error context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            source: String::new(),
            operation: String::new(),
            timestamp: time::OffsetDateTime::now_utc(),
            metadata: HashMap::new(),
            message: String::new(),
            details: None,
        }
    }
}

/// Retry policy for handling transient failures.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Calculate the delay for a given retry attempt.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay.as_secs_f64() * self.backoff_factor.powi(attempt as i32);
        Duration::from_secs_f64(delay.min(self.max_delay.as_secs_f64()))
    }
}

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
    /// The LLM request timed out
    #[error("LLM request timed out after {duration_secs} seconds at {timestamp}")]
    Timeout {
        duration_secs: u64,
        timestamp: OffsetDateTime,
    },
    /// The LLM returned an error
    #[error("LLM error: {0}")]
    LlmError(String),
    /// Failed to parse LLM response
    #[error("Failed to parse LLM response: {0}")]
    ParseError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Other error: {0}")]
    Other(String),
    #[error("API error: {0}")]
    ApiError(String),
}

/// Errors that can occur during agent operations.
#[derive(Error, Debug)]
pub enum AgentError {
    /// The agent request timed out
    #[error("Agent request timed out after {0} seconds")]
    Timeout(u64),

    /// The agent initialization failed
    #[error("Agent initialization failed: {0}")]
    InitializationFailed(String),

    /// The agent shutdown failed
    #[error("Agent shutdown failed: {0}")]
    ShutdownFailed(String),

    /// The agent execution failed
    #[error("Agent execution failed: {message}")]
    ExecutionFailed {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
        /// Whether the error is retryable
        retryable: bool,
    },

    /// Tool selection failed
    #[error("Tool selection failed: {0}")]
    ToolSelectionFailed(String),

    /// Planning failed
    #[error("Planning failed: {0}")]
    PlanningFailed(String),

    /// An underlying LLM error occurred
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// An underlying Tool error occurred
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// An underlying Memory error occurred
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    /// A chain error occurred
    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),
}

/// Errors that can occur during tool operations.
#[derive(Error, Debug)]
pub enum ToolError {
    /// The tool request timed out
    #[error("Tool request timed out after {0} seconds")]
    Timeout(u64),
    /// The tool returned an error
    #[error("Tool error: {0}")]
    ToolError(String),
    /// Failed to parse tool response
    #[error("Failed to parse tool response: {0}")]
    ParseError(String),
    /// Tool execution failed
    #[error("Tool execution failed: {message}")]
    ExecutionFailed {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
        /// Whether the error is retryable
        retryable: bool,
    },

    /// Tool initialization failed
    #[error("Tool initialization failed: {0}")]
    InitializationFailed(String),

    /// Tool shutdown failed
    #[error("Tool shutdown failed: {0}")]
    ShutdownFailed(String),
}

/// Errors that can occur during tool configuration validation.
#[derive(Error, Debug)]
pub enum ToolConfigError {
    /// A required configuration field is missing
    #[error("Missing configuration field: {field_name}")]
    MissingField { field_name: String },

    /// An invalid value was provided for a configuration field
    #[error("Invalid configuration value for field '{field_name}': {message}")]
    InvalidValue { field_name: String, message: String },

    /// A general validation error
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
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

/// Error type for chain execution
#[derive(Debug, Error)]
pub struct ChainError {
    /// Error variant
    #[source]
    pub kind: ChainErrorKind,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

/// Chain error variants
#[derive(Debug, Error)]
pub enum ChainErrorKind {
    /// Chain execution timed out
    #[error("Chain execution timed out after {duration:?} in {step_type} step")]
    Timeout {
        /// Duration after which the timeout occurred
        duration: Duration,
        /// Type of step that timed out
        step_type: &'static str,
    },

    /// Chain execution was cancelled
    #[error("Chain execution was cancelled")]
    Cancelled,

    /// Error in parallel chain execution
    #[error("Parallel chain error: {message}")]
    ParallelError {
        /// Error message
        message: String,
        /// Results from successful parallel executions
        successful_results: Vec<Box<dyn std::any::Any + Send>>,
    },

    /// Chain execution failed
    #[error("Chain execution failed: {message}")]
    Failed {
        /// Error message
        message: String,
    },

    /// An underlying LLM error occurred during a chain step
    #[error("LLM step failed: {0}")]
    Llm(#[from] LlmError),

    /// An underlying Tool error occurred during a chain step
    #[error("Tool step failed: {0}")]
    Tool(#[from] ToolError),

    /// An underlying Memory error occurred during a chain step
    #[error("Memory step failed: {0}")]
    Memory(#[from] MemoryError),

    /// An invalid step transition occurred
    #[error("Invalid chain step transition: {0}")]
    InvalidTransition(String),
}

impl ChainError {
    /// Create a new chain error from a specific kind.
    pub fn new(kind: ChainErrorKind) -> Self {
        Self { kind }
    }

    /// Create a new chain error directly from an underlying LLM error.
    pub fn from_llm(error: LlmError) -> Self {
        Self::new(ChainErrorKind::Llm(error))
    }

    /// Create a new chain error directly from an underlying Tool error.
    pub fn from_tool(error: ToolError) -> Self {
        Self::new(ChainErrorKind::Tool(error))
    }

    /// Create a new chain error directly from an underlying Memory error.
    pub fn from_memory(error: MemoryError) -> Self {
        Self::new(ChainErrorKind::Memory(error))
    }

    /// Create a new timeout error
    pub fn timeout(duration: Duration, step_type: &'static str) -> Self {
        Self::new(ChainErrorKind::Timeout {
            duration,
            step_type,
        })
    }

    /// Create a new cancelled error
    pub fn cancelled() -> Self {
        Self::new(ChainErrorKind::Cancelled)
    }

    /// Create a new parallel error
    pub fn parallel_error(
        message: String,
        successful_results: Vec<Box<dyn std::any::Any + Send>>,
    ) -> Self {
        Self::new(ChainErrorKind::ParallelError {
            message,
            successful_results,
        })
    }

    /// Create a new failed error
    pub fn failed(message: String) -> Self {
        Self::new(ChainErrorKind::Failed { message })
    }
}

/// Interface for error reporting.
pub trait ErrorReporter: Send + Sync {
    /// Report an error.
    fn report_error(&self, error: &Error, context: &ErrorContext);

    /// Report a warning.
    fn report_warning(&self, message: &str, context: &ErrorContext);

    /// Flush any buffered reports.
    fn flush(&self);
}

/// A no-op error reporter that does nothing.
#[derive(Debug, Default)]
pub struct NoopErrorReporter;

impl ErrorReporter for NoopErrorReporter {
    fn report_error(&self, _error: &Error, _context: &ErrorContext) {}
    fn report_warning(&self, _message: &str, _context: &ErrorContext) {}
    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("math_tool", "matrix_multiply")
            .with_metadata("matrix_size", "100x100")
            .with_metadata("operation_id", "123");

        assert_eq!(context.source, "math_tool");
        assert_eq!(context.operation, "matrix_multiply");
        assert_eq!(
            context.metadata.get("matrix_size"),
            Some(&"100x100".to_string())
        );
        assert_eq!(
            context.metadata.get("operation_id"),
            Some(&"123".to_string())
        );
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();

        let delay1 = policy.delay_for_attempt(0);
        let delay2 = policy.delay_for_attempt(1);
        let delay3 = policy.delay_for_attempt(2);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
        assert!(delay3 <= policy.max_delay);
    }

    #[test]
    fn test_tool_error_creation() {
        let context = ErrorContext::new("search_tool", "web_search").with_metadata("query", "test");

        let error = ToolError::ExecutionFailed {
            context: context.clone(),
            message: "API request failed".to_string(),
            retryable: true,
        };

        match error {
            ToolError::ExecutionFailed { retryable, .. } => {
                assert!(retryable);
            }
            _ => panic!("Wrong error variant"),
        }
    }
}
