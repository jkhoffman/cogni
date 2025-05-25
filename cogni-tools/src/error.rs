//! Error types for tool execution

use std::error::Error as StdError;
use std::fmt;

/// Error type for tool operations
#[derive(Debug)]
pub enum ToolError {
    /// Tool not found in registry
    NotFound {
        /// Tool name that was not found
        name: String,
    },
    
    /// Invalid arguments passed to tool
    InvalidArguments {
        /// Tool name
        tool: String,
        /// Error message
        message: String,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// Tool execution failed
    ExecutionFailed {
        /// Tool name
        tool: String,
        /// Error message
        message: String,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// JSON parsing/serialization error
    JsonError {
        /// Error message
        message: String,
        /// Underlying error
        source: serde_json::Error,
    },
    
    /// Validation error
    ValidationFailed {
        /// Tool name
        tool: String,
        /// Validation errors
        errors: Vec<String>,
    },
    
    /// Network error (for remote tools)
    Network {
        /// Error message
        message: String,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// Timeout
    Timeout {
        /// Tool name
        tool: String,
        /// Timeout duration
        duration: std::time::Duration,
    },
}

/// Error kind for categorizing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorKind {
    /// Tool not found
    NotFound,
    /// Invalid arguments
    InvalidArguments,
    /// Execution failed
    ExecutionFailed,
    /// JSON error
    JsonError,
    /// Validation failed
    ValidationFailed,
    /// Network error
    Network,
    /// Timeout
    Timeout,
}

impl ToolError {
    /// Get the error kind
    pub fn kind(&self) -> ToolErrorKind {
        match self {
            ToolError::NotFound { .. } => ToolErrorKind::NotFound,
            ToolError::InvalidArguments { .. } => ToolErrorKind::InvalidArguments,
            ToolError::ExecutionFailed { .. } => ToolErrorKind::ExecutionFailed,
            ToolError::JsonError { .. } => ToolErrorKind::JsonError,
            ToolError::ValidationFailed { .. } => ToolErrorKind::ValidationFailed,
            ToolError::Network { .. } => ToolErrorKind::Network,
            ToolError::Timeout { .. } => ToolErrorKind::Timeout,
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::NotFound { name } => {
                write!(f, "Tool not found: {}", name)
            }
            ToolError::InvalidArguments { tool, message, .. } => {
                write!(f, "Invalid arguments for tool '{}': {}", tool, message)
            }
            ToolError::ExecutionFailed { tool, message, .. } => {
                write!(f, "Tool execution failed for '{}': {}", tool, message)
            }
            ToolError::JsonError { message, .. } => {
                write!(f, "JSON error: {}", message)
            }
            ToolError::ValidationFailed { tool, errors } => {
                write!(f, "Validation failed for tool '{}': {}", tool, errors.join(", "))
            }
            ToolError::Network { message, .. } => {
                write!(f, "Network error: {}", message)
            }
            ToolError::Timeout { tool, duration } => {
                write!(f, "Tool '{}' timed out after {:?}", tool, duration)
            }
        }
    }
}

impl StdError for ToolError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ToolError::InvalidArguments { source, .. } |
            ToolError::ExecutionFailed { source, .. } |
            ToolError::Network { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn StdError + 'static))
            }
            ToolError::JsonError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        ToolError::JsonError {
            message: err.to_string(),
            source: err,
        }
    }
}

/// Result type for tool operations
pub type Result<T> = std::result::Result<T, ToolError>;