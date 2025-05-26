//! Error types for state management

use std::io;
use thiserror::Error;
use uuid::Uuid;

/// Result type for state operations
pub type StateResult<T> = Result<T, StateError>;

/// Errors that can occur during state operations
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// State not found
    #[error("State not found: {0}")]
    NotFound(Uuid),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Lock contention error
    #[error("Lock contention: {0}")]
    LockContention(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl StateError {
    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a lock contention error
    pub fn lock_contention(msg: impl Into<String>) -> Self {
        Self::LockContention(msg.into())
    }

    /// Create an invalid state error
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self::InvalidState(msg.into())
    }

    /// Create a configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let id = Uuid::new_v4();
        let err = StateError::NotFound(id);
        assert_eq!(err.to_string(), format!("State not found: {}", id));

        let err = StateError::Storage("Database connection failed".to_string());
        assert_eq!(err.to_string(), "Storage error: Database connection failed");

        let err = StateError::LockContention("Resource busy".to_string());
        assert_eq!(err.to_string(), "Lock contention: Resource busy");

        let err = StateError::InvalidState("Corrupted data".to_string());
        assert_eq!(err.to_string(), "Invalid state: Corrupted data");

        let err = StateError::Configuration("Missing API key".to_string());
        assert_eq!(err.to_string(), "Configuration error: Missing API key");
    }

    #[test]
    fn test_error_constructors() {
        let err = StateError::storage("test storage error");
        assert!(matches!(err, StateError::Storage(msg) if msg == "test storage error"));

        let err = StateError::lock_contention("test lock error");
        assert!(matches!(err, StateError::LockContention(msg) if msg == "test lock error"));

        let err = StateError::invalid_state("test state error");
        assert!(matches!(err, StateError::InvalidState(msg) if msg == "test state error"));

        let err = StateError::configuration("test config error");
        assert!(matches!(err, StateError::Configuration(msg) if msg == "test config error"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let state_err: StateError = io_err.into();

        match state_err {
            StateError::Io(err) => {
                assert_eq!(err.kind(), io::ErrorKind::NotFound);
            }
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let state_err: StateError = json_err.into();

        assert!(matches!(state_err, StateError::Serialization(_)));
    }

    #[test]
    fn test_state_result_type() {
        fn success_fn() -> StateResult<String> {
            Ok("success".to_string())
        }

        fn error_fn() -> StateResult<String> {
            Err(StateError::NotFound(Uuid::new_v4()))
        }

        assert!(success_fn().is_ok());
        assert!(error_fn().is_err());
    }

    #[test]
    fn test_error_debug() {
        let err = StateError::Storage("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Storage"));
        assert!(debug_str.contains("test"));
    }
}
