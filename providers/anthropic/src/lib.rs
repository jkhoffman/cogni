//! Anthropic provider for the Cogni framework.
//!
//! This crate provides an implementation of the `LanguageModel` trait for Anthropic's Claude models.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::{
    error::LlmError,
    llm::{GenerateOptions, LanguageModel},
};
use futures::Stream;
use pin_project::pin_project;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::instrument;

/// A stream of chat responses from the Anthropic API.
/// This struct implements the Stream trait to provide asynchronous access to streaming responses.
#[pin_project]
pub struct ChatStream {
    #[pin]
    inner: Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send + 'static>>,
}

impl Stream for ChatStream {
    type Item = Result<String, LlmError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

/// Configuration for the Anthropic client.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used in future implementations
pub struct AnthropicConfig {
    /// The API key for authentication
    api_key: String,
    /// The model to use (e.g., "claude-3-opus-20240229")
    model: String,
    /// Base URL for the API (defaults to "https://api.anthropic.com/v1")
    base_url: String,
    /// HTTP client configuration
    client: Option<Client>,
}

impl AnthropicConfig {
    /// Create a new configuration with the given API key and model.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: None,
        }
    }

    /// Set a custom base URL for the API.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set custom HTTP client configuration.
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }
}

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender
    pub role: String,
    /// The content of the message
    pub content: String,
}

/// The Anthropic language model client.
#[allow(dead_code)] // Fields will be used in future implementations
pub struct AnthropicClient {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicClient {
    /// Create a new Anthropic client with the given configuration.
    pub fn new(mut config: AnthropicConfig) -> Result<Self, LlmError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&config.api_key)
                .map_err(|e| LlmError::ConfigError(e.to_string()))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let client = config.client.take().unwrap_or_else(|| {
            Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build HTTP client")
        });

        Ok(Self { config, client })
    }

    #[allow(dead_code)] // Will be used when streaming is implemented
    async fn process_stream_response(
        response: reqwest::Response,
    ) -> Result<impl Stream<Item = Result<String, LlmError>> + Send + 'static, LlmError> {
        let stream = futures::stream::unfold(response, |mut response| async move {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    Self::parse_stream_chunk(&chunk).map(|result| (result, response))
                }
                Ok(None) => None,
                Err(e) => Some((Err(LlmError::RequestFailed(e)), response)),
            }
        });

        Ok(stream)
    }

    #[allow(dead_code)] // Will be used when streaming is implemented
    fn parse_stream_chunk(bytes: &[u8]) -> Option<Result<String, LlmError>> {
        match String::from_utf8(bytes.to_vec()) {
            Ok(text) => {
                // TODO: Implement Anthropic-specific stream parsing
                // This is a placeholder that needs to be updated with actual Anthropic streaming format
                Some(Ok(text))
            }
            Err(e) => Some(Err(LlmError::InvalidResponse(e.to_string()))),
        }
    }
}

#[async_trait]
impl LanguageModel for AnthropicClient {
    type Prompt = Vec<ChatMessage>;
    type Response = ChatMessage;
    type TokenStream = ChatStream;

    #[instrument(name = "anthropic_generate", skip_all, err)]
    async fn generate(
        &self,
        _prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        todo!("Implement Anthropic chat completion")
    }

    #[instrument(name = "anthropic_stream_generate", skip_all, err)]
    async fn stream_generate(
        &self,
        _prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        todo!("Implement Anthropic streaming chat completion")
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = AnthropicConfig::new("test_key", "claude-3-opus-20240229");
        let client = AnthropicClient::new(config).unwrap();
        assert_eq!(client.name(), "anthropic");
    }
}
