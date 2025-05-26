//! Provider-specific error types

use cogni_core::Error as CoreError;
use std::time::Duration;

/// Convert provider errors to core errors
pub fn to_core_error(
    provider: impl Into<String>,
    message: impl Into<String>,
    retry_after: Option<Duration>,
) -> CoreError {
    CoreError::Provider {
        provider: provider.into(),
        message: message.into(),
        retry_after,
        source: None,
    }
}

/// Convert network errors to core errors
pub fn network_error(error: reqwest::Error) -> CoreError {
    CoreError::Network {
        message: error.to_string(),
        source: Some(Box::new(error)),
    }
}

/// Convert serialization errors to core errors
pub fn serialization_error(error: serde_json::Error) -> CoreError {
    CoreError::Serialization {
        message: error.to_string(),
        source: Some(Box::new(error)),
    }
}
