//! Common utilities for provider implementations

use cogni_core::Error;
use reqwest::Response;

/// Check HTTP response status and convert to appropriate error
pub async fn check_response_status(
    response: Response,
    provider_name: &str,
) -> Result<Response, Error> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| format!("HTTP {} error", status.as_u16()));

        Err(Error::Provider {
            provider: provider_name.to_string(),
            message: format!("HTTP {}: {}", status, error_text),
            retry_after: None,
            source: None,
        })
    }
}

/// Convert a reqwest error to a network error
pub fn to_network_error(err: reqwest::Error) -> Error {
    Error::Network {
        message: err.to_string(),
        source: Some(Box::new(err)),
    }
}

/// Convert a serde_json error to a serialization error
pub fn to_serialization_error(err: serde_json::Error) -> Error {
    Error::Serialization {
        message: err.to_string(),
        source: Some(Box::new(err)),
    }
}

/// Helper to set stream field on a request
pub fn set_stream_field<T>(request: T, stream: bool) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    // This is a bit hacky but avoids duplicating the logic in each provider
    if let Ok(mut value) = serde_json::to_value(&request) {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(stream));
        }
        if let Ok(updated) = serde_json::from_value(value) {
            return updated;
        }
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestRequest {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        stream: Option<bool>,
    }

    #[test]
    fn test_set_stream_field() {
        let request = TestRequest {
            message: "test".to_string(),
            stream: None,
        };

        let updated = set_stream_field(request, true);
        assert_eq!(updated.stream, Some(true));
    }

    #[test]
    fn test_set_stream_field_with_existing_value() {
        let request = TestRequest {
            message: "test".to_string(),
            stream: Some(false),
        };

        let updated = set_stream_field(request, true);
        assert_eq!(updated.stream, Some(true));
    }

    // Note: We can't easily test check_response_status without http crate or a real server
    // Those functions are tested indirectly through provider integration tests

    // Note: to_network_error requires a reqwest::Error which is hard to construct in tests
    // This function is tested indirectly through provider integration tests

    #[test]
    fn test_to_serialization_error() {
        let json_err = serde_json::from_str::<TestRequest>("invalid json").unwrap_err();
        let ser_err = to_serialization_error(json_err);

        match ser_err {
            Error::Serialization { message, source } => {
                assert!(!message.is_empty());
                assert!(source.is_some());
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_set_stream_field_with_non_object() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct SimpleValue(String);

        let value = SimpleValue("test".to_string());
        let result = set_stream_field(value, true);

        // Should return the original value unchanged
        assert_eq!(result, SimpleValue("test".to_string()));
    }

    #[test]
    fn test_set_stream_field_preserves_other_fields() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct ComplexRequest {
            message: String,
            count: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            stream: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            optional: Option<String>,
        }

        let request = ComplexRequest {
            message: "test".to_string(),
            count: 42,
            stream: None,
            optional: Some("value".to_string()),
        };

        let updated = set_stream_field(request, false);
        assert_eq!(updated.message, "test");
        assert_eq!(updated.count, 42);
        assert_eq!(updated.stream, Some(false));
        assert_eq!(updated.optional, Some("value".to_string()));
    }
}
