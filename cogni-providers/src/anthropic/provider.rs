//! Anthropic provider implementation

use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response};
use reqwest::Client;
use reqwest_eventsource::RequestBuilderExt;

use crate::anthropic::{
    config::AnthropicConfig,
    converter::{to_anthropic_request, AnthropicResponse},
    parser::parse_response,
    stream::AnthropicStream,
};

/// Anthropic Claude provider implementation
#[derive(Debug, Clone)]
pub struct Anthropic {
    config: AnthropicConfig,
    client: Client,
}

impl Anthropic {
    /// Create a new Anthropic provider with the given configuration
    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Create a new Anthropic provider with just an API key
    pub fn with_api_key(api_key: String) -> Self {
        let config = AnthropicConfig {
            api_key,
            ..Default::default()
        };
        Self::new(config)
    }
}

#[async_trait]
impl Provider for Anthropic {
    type Stream = AnthropicStream;

    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut anthropic_request = to_anthropic_request(&request);
        anthropic_request.stream = Some(false);

        let response = self
            .client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| Error::Network {
                message: e.to_string(),
                source: None,
            })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                message: error_text,
                retry_after: None,
                source: None,
            });
        }

        let anthropic_response: AnthropicResponse =
            response.json().await.map_err(|e| Error::Serialization {
                message: e.to_string(),
                source: None,
            })?;

        parse_response(anthropic_response)
    }

    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let mut anthropic_request = to_anthropic_request(&request);
        anthropic_request.stream = Some(true);

        let event_source = self
            .client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .eventsource()
            .map_err(|e| Error::Network {
                message: e.to_string(),
                source: None,
            })?;

        Ok(AnthropicStream::new(event_source))
    }
}
