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

    /// Storage errors (for state management)
    Storage(String),

    /// Response parsing errors
    ResponseError {
        /// Error message
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Network { message, .. } => write!(f, "Network error: {}", message),
            Error::Provider {
                provider, message, ..
            } => {
                write!(f, "Provider error ({}): {}", provider, message)
            }
            Error::Serialization { message, .. } => write!(f, "Serialization error: {}", message),
            Error::Validation(msg) => write!(f, "Validation error: {}", msg),
            Error::ToolExecution(msg) => write!(f, "Tool execution error: {}", msg),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Error::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            Error::Storage(msg) => write!(f, "Storage error: {}", msg),
            Error::ResponseError { message } => write!(f, "Response error: {}", message),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Network { source, .. }
            | Error::Provider { source, .. }
            | Error::Serialization { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn StdError + 'static)),
            _ => None,
        }
    }
}

/// Result type alias for Cogni operations
pub type Result<T> = std::result::Result<T, Error>;

// Common From implementations for error conversions
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Network {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let error = Error::Network {
            message: "Connection refused".into(),
            source: None,
        };
        assert_eq!(error.to_string(), "Network error: Connection refused");

        let error = Error::Provider {
            provider: "openai".into(),
            message: "Rate limit exceeded".into(),
            retry_after: Some(Duration::from_secs(60)),
            source: None,
        };
        assert_eq!(
            error.to_string(),
            "Provider error (openai): Rate limit exceeded"
        );

        let error = Error::Serialization {
            message: "Invalid JSON".into(),
            source: None,
        };
        assert_eq!(error.to_string(), "Serialization error: Invalid JSON");

        let error = Error::Validation("Missing required field".into());
        assert_eq!(
            error.to_string(),
            "Validation error: Missing required field"
        );

        let error = Error::ToolExecution("Tool not found".into());
        assert_eq!(error.to_string(), "Tool execution error: Tool not found");

        let error = Error::Timeout;
        assert_eq!(error.to_string(), "Operation timed out");

        let error = Error::Authentication("Invalid API key".into());
        assert_eq!(error.to_string(), "Authentication error: Invalid API key");

        let error = Error::Configuration("Invalid model name".into());
        assert_eq!(error.to_string(), "Configuration error: Invalid model name");

        let error = Error::Storage("Failed to save state".into());
        assert_eq!(error.to_string(), "Storage error: Failed to save state");

        let error = Error::ResponseError {
            message: "Failed to parse response".into(),
        };
        assert_eq!(
            error.to_string(),
            "Response error: Failed to parse response"
        );
    }

    #[test]
    fn test_error_source() {
        // Error without source
        let error = Error::Network {
            message: "Connection failed".into(),
            source: None,
        };
        assert!(error.source().is_none());

        // Error with source
        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        let error = Error::Network {
            message: "Connection failed".into(),
            source: Some(Box::new(io_error)),
        };
        assert!(error.source().is_some());

        // Provider error with source
        let io_error = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let error = Error::Provider {
            provider: "anthropic".into(),
            message: "Request timed out".into(),
            retry_after: None,
            source: Some(Box::new(io_error)),
        };
        assert!(error.source().is_some());

        // Serialization error with source
        let json_error = serde_json::from_str::<String>("invalid").unwrap_err();
        let error = Error::Serialization {
            message: "JSON parse error".into(),
            source: Some(Box::new(json_error)),
        };
        assert!(error.source().is_some());

        // Errors without source field
        let error = Error::Validation("test".into());
        assert!(error.source().is_none());

        let error = Error::Timeout;
        assert!(error.source().is_none());
    }

    #[test]
    fn test_error_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        let error: Error = io_error.into();

        match error {
            Error::Network { message, source } => {
                assert!(message.contains("Connection refused"));
                assert!(source.is_some());
            }
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_error_from_serde_json_error() {
        let json_error = serde_json::from_str::<String>("invalid json").unwrap_err();
        let error: Error = json_error.into();

        match error {
            Error::Serialization { message, source } => {
                assert!(!message.is_empty());
                assert!(source.is_some());
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_provider_error_with_retry_after() {
        let error = Error::Provider {
            provider: "openai".into(),
            message: "Rate limit exceeded".into(),
            retry_after: Some(Duration::from_secs(30)),
            source: None,
        };

        match error {
            Error::Provider { retry_after, .. } => {
                assert_eq!(retry_after, Some(Duration::from_secs(30)));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_error_debug_format() {
        let error = Error::Network {
            message: "Test error".into(),
            source: None,
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Network"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("success".to_string())
        }

        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");

        fn failing_function() -> Result<String> {
            Err(Error::Validation("Test failure".into()))
        }

        let result = failing_function();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Error>();
        assert_sync::<Error>();
    }

    #[test]
    fn test_nested_error_source() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let error: Error = io_error.into();

        let wrapper_error = Error::Storage(format!("Failed to save: {}", error));

        // Check the wrapper error message includes the nested error
        assert!(wrapper_error.to_string().contains("Failed to save"));
    }
}
