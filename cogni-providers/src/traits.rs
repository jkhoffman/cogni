//! Common traits for provider implementations

use async_trait::async_trait;
use cogni_core::{Error, Request, Response, StreamEvent};
use serde_json::Value;

/// Convert requests to provider-specific format
#[async_trait]
pub trait RequestConverter: Send + Sync {
    /// Convert a generic request to provider-specific JSON
    async fn convert_request(&self, request: Request) -> Result<Value, Error>;
}

/// Parse responses from provider-specific format
#[async_trait]
pub trait ResponseParser: Send + Sync {
    /// Parse provider-specific JSON into a generic response
    async fn parse_response(&self, value: Value) -> Result<Response, Error>;
}

/// Parse streaming events from provider-specific format
pub trait StreamEventParser: Send + Sync {
    /// Parse a line of streaming data into an event
    fn parse_event(&self, data: &str) -> Result<Option<StreamEvent>, Error>;
}
