//! Error types for the Cogni library

use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;

/// The main error type for all Cogni operations
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Network-related errors
    Network {
        /// Error message
        message: String,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// Provider-specific errors
    Provider {
        /// Provider name (e.g., "openai", "anthropic")
        provider: String,
        /// Error message
        message: String,
        /// Time to wait before retrying (for rate limits)
        retry_after: Option<Duration>,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// Serialization/deserialization errors
    Serialization {
        /// Error message
        message: String,
        /// Underlying error if available
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    /// Validation errors
    Validation(String),
    
    /// Tool execution errors
    ToolExecution(String),
    
    /// Timeout errors
    Timeout,
    
    /// Authentication errors
    Authentication(String),
    
    /// Configuration errors
    Configuration(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Network { message, .. } => write!(f, "Network error: {}", message),
            Error::Provider { provider, message, .. } => {
                write!(f, "Provider error ({}): {}", provider, message)
            }
            Error::Serialization { message, .. } => write!(f, "Serialization error: {}", message),
            Error::Validation(msg) => write!(f, "Validation error: {}", msg),
            Error::ToolExecution(msg) => write!(f, "Tool execution error: {}", msg),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Error::Configuration(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Network { source, .. } |
            Error::Provider { source, .. } |
            Error::Serialization { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn StdError + 'static))
            }
            _ => None,
        }
    }
}

/// Result type alias for Cogni operations
pub type Result<T> = std::result::Result<T, Error>;