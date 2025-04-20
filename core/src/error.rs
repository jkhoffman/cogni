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
        }
    }

    /// Add metadata to the error context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
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
    #[error("Tool execution failed: {context}")]
    ExecutionFailed {
        /// Error context
        context: ErrorContext,
        /// Underlying error message
        message: String,
        /// Whether the error is retryable
        retryable: bool,
    },

    /// Invalid input provided to the tool
    #[error("Invalid tool input: {context}")]
    InvalidInput {
        /// Error context
        context: ErrorContext,
        /// Validation errors
        validation_errors: Vec<String>,
    },

    /// Tool timed out
    #[error("Tool timed out after {duration:?}: {context}")]
    Timeout {
        /// Error context
        context: ErrorContext,
        /// Duration after which the timeout occurred
        duration: Duration,
    },

    /// Resource not found
    #[error("Resource not found: {context}")]
    NotFound {
        /// Error context
        context: ErrorContext,
        /// Resource identifier
        resource_id: String,
    },

    /// Permission denied
    #[error("Permission denied: {context}")]
    PermissionDenied {
        /// Error context
        context: ErrorContext,
        /// Required permission
        required_permission: String,
    },

    /// Resource exhausted
    #[error("Resource exhausted: {context}")]
    ResourceExhausted {
        /// Error context
        context: ErrorContext,
        /// Resource type
        resource_type: String,
        /// Current usage
        current_usage: u64,
        /// Resource limit
        limit: u64,
    },

    /// Network error
    #[error("Network error: {context}")]
    NetworkError {
        /// Error context
        context: ErrorContext,
        /// Underlying error
        error: reqwest::Error,
        /// Whether to retry the request
        retryable: bool,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {context}")]
    RateLimit {
        /// Error context
        context: ErrorContext,
        /// Time until reset
        reset_after: Duration,
    },
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
    Timeout(Duration),
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
