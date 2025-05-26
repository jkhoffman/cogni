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
