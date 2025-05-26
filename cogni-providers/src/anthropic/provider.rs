//! Anthropic provider implementation
//!
//! This module provides integration with Anthropic's Claude API, supporting
//! chat completions and streaming responses. It implements the core `Provider`
//! trait with automatic tool-calling workarounds for structured output.

use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::sync::Arc;

use crate::anthropic::{
    config::AnthropicConfig, converter::AnthropicConverter, parser::AnthropicParser,
    stream::AnthropicStream,
};
use crate::http::{HttpClient, ReqwestClient};
use crate::traits::{RequestConverter, ResponseParser};

/// Anthropic Claude provider for chat completions
///
/// This provider supports:
/// - Claude 3 family models (Opus, Sonnet, Haiku)
/// - Tool/function calling
/// - Streaming responses
/// - Structured output via tool calling workaround
/// - Custom API endpoints
///
/// # Example
///
/// ```no_run
/// use cogni_providers::Anthropic;
///
/// // Create with API key
/// let provider = Anthropic::with_api_key("your-api-key");
///
/// // Or with custom configuration and client
/// use cogni_providers::anthropic::AnthropicConfig;
/// use cogni_providers::http::{HttpClient, ReqwestClient};
/// use std::sync::Arc;
///
/// let config = AnthropicConfig {
///     api_key: "your-api-key".to_string(),
///     base_url: "https://api.anthropic.com".to_string(),
///     default_model: "claude-3-sonnet-20240229".to_string(),
/// };
/// let client = Arc::new(ReqwestClient::new().expect("Failed to create client"));
/// let provider = Anthropic::new(config, client);
/// ```
#[derive(Clone)]
pub struct Anthropic {
    config: AnthropicConfig,
    client: Arc<dyn HttpClient>,
    converter: AnthropicConverter,
    parser: AnthropicParser,
}

impl Anthropic {
    /// Create a new Anthropic provider with the given configuration and client
    pub fn new(config: AnthropicConfig, client: Arc<dyn HttpClient>) -> Self {
        Self {
            config,
            client,
            converter: AnthropicConverter,
            parser: AnthropicParser,
        }
    }

    /// Create a new Anthropic provider with just an API key
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let config = AnthropicConfig {
            api_key: api_key.into(),
            ..Default::default()
        };
        let client = Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"));
        Self::new(config, client)
    }

    /// Create Anthropic-specific headers
    fn create_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();

        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| Error::Configuration(format!("Invalid API key: {}", e)))?,
        );

        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Ok(headers)
    }
}

#[async_trait]
impl Provider for Anthropic {
    type Stream = AnthropicStream;

    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(false);

        let headers = self.create_headers()?;
        let url = format!("{}/v1/messages", self.config.base_url);

        let response_value = self.client.post(&url, headers, body).await?;

        self.parser.parse_response(response_value).await
    }

    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(true);

        let headers = self.create_headers()?;
        let url = format!("{}/v1/messages", self.config.base_url);

        let event_source = self.client.post_event_stream(&url, headers, body).await?;

        Ok(AnthropicStream::new(event_source))
    }
}
