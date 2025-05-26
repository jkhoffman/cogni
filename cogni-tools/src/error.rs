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
                write!(
                    f,
                    "Validation failed for tool '{}': {}",
                    tool,
                    errors.join(", ")
                )
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
            ToolError::InvalidArguments { source, .. }
            | ToolError::ExecutionFailed { source, .. }
            | ToolError::Network { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn StdError + 'static)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::NotFound { name: "calculator".to_string() };
        assert_eq!(err.to_string(), "Tool not found: calculator");

        let err = ToolError::InvalidArguments {
            tool: "math".to_string(),
            message: "Missing operands".to_string(),
            source: None,
        };
        assert_eq!(err.to_string(), "Invalid arguments for tool 'math': Missing operands");

        let err = ToolError::ExecutionFailed {
            tool: "api".to_string(),
            message: "Connection refused".to_string(),
            source: None,
        };
        assert_eq!(err.to_string(), "Tool execution failed for 'api': Connection refused");

        let err = ToolError::JsonError {
            message: "Invalid JSON".to_string(),
            source: serde_json::from_str::<String>("invalid").unwrap_err(),
        };
        assert_eq!(err.to_string(), "JSON error: Invalid JSON");

        let err = ToolError::ValidationFailed {
            tool: "schema".to_string(),
            errors: vec!["Field 'name' is required".to_string(), "Invalid type".to_string()],
        };
        assert_eq!(err.to_string(), "Validation failed for tool 'schema': Field 'name' is required, Invalid type");

        let err = ToolError::Network {
            message: "DNS lookup failed".to_string(),
            source: None,
        };
        assert_eq!(err.to_string(), "Network error: DNS lookup failed");

        let err = ToolError::Timeout {
            tool: "slow_api".to_string(),
            duration: std::time::Duration::from_secs(30),
        };
        assert_eq!(err.to_string(), "Tool 'slow_api' timed out after 30s");
    }

    #[test]
    fn test_tool_error_kind() {
        let test_cases = vec![
            (ToolError::NotFound { name: "test".to_string() }, ToolErrorKind::NotFound),
            (ToolError::InvalidArguments { tool: "test".to_string(), message: "test".to_string(), source: None }, ToolErrorKind::InvalidArguments),
            (ToolError::ExecutionFailed { tool: "test".to_string(), message: "test".to_string(), source: None }, ToolErrorKind::ExecutionFailed),
            (ToolError::JsonError { message: "test".to_string(), source: serde_json::from_str::<String>("invalid").unwrap_err() }, ToolErrorKind::JsonError),
            (ToolError::ValidationFailed { tool: "test".to_string(), errors: vec![] }, ToolErrorKind::ValidationFailed),
            (ToolError::Network { message: "test".to_string(), source: None }, ToolErrorKind::Network),
            (ToolError::Timeout { tool: "test".to_string(), duration: std::time::Duration::from_secs(1) }, ToolErrorKind::Timeout),
        ];

        for (error, expected_kind) in test_cases {
            assert_eq!(error.kind(), expected_kind);
        }
    }

    #[test]
    fn test_tool_error_source() {
        // Error without source
        let err = ToolError::NotFound { name: "test".to_string() };
        assert!(err.source().is_none());

        // Error with io::Error source
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let err = ToolError::InvalidArguments {
            tool: "test".to_string(),
            message: "test".to_string(),
            source: Some(Box::new(io_err)),
        };
        assert!(err.source().is_some());

        // ExecutionFailed with source
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let err = ToolError::ExecutionFailed {
            tool: "test".to_string(),
            message: "test".to_string(),
            source: Some(Box::new(io_err)),
        };
        assert!(err.source().is_some());

        // Network error with source
        let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        let err = ToolError::Network {
            message: "test".to_string(),
            source: Some(Box::new(io_err)),
        };
        assert!(err.source().is_some());

        // JsonError always has source
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let err = ToolError::JsonError {
            message: "test".to_string(),
            source: json_err,
        };
        assert!(err.source().is_some());

        // Errors without source field
        let err = ToolError::ValidationFailed { tool: "test".to_string(), errors: vec![] };
        assert!(err.source().is_none());

        let err = ToolError::Timeout { tool: "test".to_string(), duration: std::time::Duration::from_secs(1) };
        assert!(err.source().is_none());
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
        let tool_err: ToolError = json_err.into();

        match tool_err {
            ToolError::JsonError { message, source } => {
                assert!(!message.is_empty());
                assert_eq!(message, source.to_string());
            }
            _ => panic!("Expected JsonError"),
        }
    }

    #[test]
    fn test_result_type() {
        fn success_fn() -> Result<String> {
            Ok("success".to_string())
        }

        fn error_fn() -> Result<String> {
            Err(ToolError::NotFound { name: "missing".to_string() })
        }

        assert!(success_fn().is_ok());
        assert!(error_fn().is_err());
    }

    #[test]
    fn test_error_debug() {
        let err = ToolError::NotFound { name: "test".to_string() };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_kind_equality() {
        assert_eq!(ToolErrorKind::NotFound, ToolErrorKind::NotFound);
        assert_ne!(ToolErrorKind::NotFound, ToolErrorKind::Timeout);
    }

    #[test]
    fn test_validation_failed_empty_errors() {
        let err = ToolError::ValidationFailed {
            tool: "empty".to_string(),
            errors: vec![],
        };
        assert_eq!(err.to_string(), "Validation failed for tool 'empty': ");
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ToolError>();
        assert_sync::<ToolError>();
    }
}
