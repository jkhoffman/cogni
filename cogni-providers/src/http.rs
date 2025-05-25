//! HTTP client abstraction and utilities

use crate::error;
use bytes::Bytes;
use cogni_core::Error;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use std::pin::Pin;

/// Type alias for response streams
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

/// HTTP client abstraction
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    /// Send a POST request
    async fn post(&self, url: &str, headers: HeaderMap, body: Value) -> Result<Value, Error>;

    /// Send a streaming POST request
    async fn post_stream(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Value,
    ) -> Result<ResponseStream, Error>;
}

/// Default HTTP client implementation using reqwest
pub struct ReqwestClient {
    client: reqwest::Client,
}

impl ReqwestClient {
    /// Create a new HTTP client
    pub fn new() -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(error::network_error)?;

        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl HttpClient for ReqwestClient {
    async fn post(&self, url: &str, headers: HeaderMap, body: Value) -> Result<Value, Error> {
        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(error::network_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Network {
                message: format!("HTTP {}: {}", status, text),
                source: None,
            });
        }

        response.json().await.map_err(error::network_error)
    }

    async fn post_stream(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Value,
    ) -> Result<ResponseStream, Error> {
        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(error::network_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Network {
                message: format!("HTTP {}: {}", status, text),
                source: None,
            });
        }

        Ok(Box::pin(response.bytes_stream()))
    }
}

/// Helper to create common headers
pub fn create_headers(api_key: &str, additional: Option<HeaderMap>) -> Result<HeaderMap, Error> {
    let mut headers = HeaderMap::new();

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| Error::Configuration(format!("Invalid API key: {}", e)))?,
    );

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if let Some(additional) = additional {
        headers.extend(additional);
    }

    Ok(headers)
}
