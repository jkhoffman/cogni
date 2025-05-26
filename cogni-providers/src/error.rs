//! Provider-specific error types

use cogni_core::Error as CoreError;
use reqwest::StatusCode;
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

/// Convert network errors with context
pub fn network_error_with_context(error: reqwest::Error, context: &str) -> CoreError {
    CoreError::Network {
        message: format!("{}: {}", context, error),
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

/// Convert serialization errors with context
pub fn serialization_error_with_context(error: serde_json::Error, context: &str) -> CoreError {
    CoreError::Serialization {
        message: format!("{}: {}", context, error),
        source: Some(Box::new(error)),
    }
}

/// Create provider error from HTTP status
pub fn provider_error_from_status(provider: &str, status: StatusCode, body: &str) -> CoreError {
    let retry_after = match status {
        StatusCode::TOO_MANY_REQUESTS => Some(Duration::from_secs(60)),
        _ => None,
    };

    let message = match status {
        StatusCode::BAD_REQUEST => format!("Bad request: {}", body),
        StatusCode::UNAUTHORIZED => format!("Authentication failed: {}", body),
        StatusCode::FORBIDDEN => format!("Access forbidden: {}", body),
        StatusCode::NOT_FOUND => format!("Resource not found: {}", body),
        StatusCode::TOO_MANY_REQUESTS => format!("Rate limited: {}", body),
        StatusCode::INTERNAL_SERVER_ERROR => format!("Server error: {}", body),
        StatusCode::BAD_GATEWAY => format!("Bad gateway: {}", body),
        StatusCode::SERVICE_UNAVAILABLE => format!("Service unavailable: {}", body),
        _ => format!("HTTP {} error: {}", status.as_u16(), body),
    };

    to_core_error(provider, message, retry_after)
}

/// Parse retry-after header
pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
}
