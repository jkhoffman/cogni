//! Ollama provider implementation
//!
//! This module provides integration with Ollama's local API, supporting
//! chat completions and streaming responses for locally-hosted models.
//! It implements the core `Provider` trait for self-hosted LLM inference.

use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::sync::Arc;

use crate::builder::ProviderBuilder;
use crate::http::{HttpClient, ReqwestClient};
use crate::ollama::{
    config::OllamaConfig, converter::OllamaConverter, parser::OllamaParser, stream::OllamaStream,
};
use crate::traits::{RequestConverter, ResponseParser};

/// Ollama provider for local model inference
///
/// This provider supports:
/// - Local Llama, Mistral, and other models
/// - Tool/function calling (model-dependent)
/// - Streaming responses
/// - Structured output with JSON schemas
/// - Custom endpoints for remote Ollama instances
///
/// # Example
///
/// ```no_run
/// use cogni_providers::Ollama;
///
/// // Create for local instance
/// let provider = Ollama::local();
///
/// // Or with custom endpoint
/// let provider = Ollama::with_base_url("http://remote-server:11434");
///
/// // Or with full configuration and custom client
/// use cogni_providers::ollama::OllamaConfig;
/// use cogni_providers::http::{HttpClient, ReqwestClient};
/// use std::sync::Arc;
///
/// let config = OllamaConfig {
///     base_url: "http://localhost:11434".to_string(),
///     default_model: "llama3.2".to_string(),
/// };
/// let client = Arc::new(ReqwestClient::new().expect("Failed to create client"));
/// let provider = Ollama::new(config, client);
/// ```
#[derive(Clone)]
pub struct Ollama {
    config: OllamaConfig,
    client: Arc<dyn HttpClient>,
    converter: OllamaConverter,
    parser: OllamaParser,
}

impl Ollama {
    /// Create a new Ollama provider with the given configuration and client
    pub fn new(config: OllamaConfig, client: Arc<dyn HttpClient>) -> Self {
        Self {
            config,
            client,
            converter: OllamaConverter,
            parser: OllamaParser,
        }
    }

    /// Create a new Ollama provider with default local configuration
    pub fn local() -> Self {
        let client = Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"));
        Self::new(OllamaConfig::default(), client)
    }

    /// Create a new Ollama provider with a custom base URL
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let config = OllamaConfig {
            base_url: base_url.into(),
            ..Default::default()
        };
        let client = Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"));
        Self::new(config, client)
    }

    /// Create headers for Ollama requests (minimal headers needed)
    fn create_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

#[async_trait]
impl Provider for Ollama {
    type Stream = OllamaStream;

    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(false);

        let headers = self.create_headers()?;
        let url = format!("{}/api/chat", self.config.base_url);

        let response_value = self.client.post(&url, headers, body).await?;

        self.parser.parse_response(response_value).await
    }

    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(true);

        let headers = self.create_headers()?;
        let url = format!("{}/api/chat", self.config.base_url);

        let response = self.client.post_raw(&url, headers, body).await?;

        Ok(OllamaStream::new(response))
    }
}

/// Builder for creating Ollama provider instances.
///
/// This builder allows configuring the Ollama provider with custom settings
/// and HTTP clients using a fluent interface pattern.
///
/// # Example
///
/// ```rust,no_run
/// use cogni_providers::ollama::OllamaBuilder;
/// use cogni_providers::builder::ProviderBuilder;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let my_custom_client = cogni_providers::http::ReqwestClient::new()?;
/// let provider = OllamaBuilder::new("http://localhost:11434".to_string())
///     .with_model("llama2")
///     .with_client(Arc::new(my_custom_client))
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct OllamaBuilder {
    base_url: String,
    model: Option<String>,
    client: Option<Arc<dyn HttpClient>>,
}

impl OllamaBuilder {
    /// Creates a new builder with the specified base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            model: None,
            client: None,
        }
    }

    /// Sets the default model for the provider.
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }
}

impl ProviderBuilder for OllamaBuilder {
    type Provider = Ollama;

    fn with_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.client = Some(client);
        self
    }

    fn build(self) -> Result<Self::Provider, Error> {
        let config = OllamaConfig {
            base_url: self.base_url,
            default_model: self.model.unwrap_or_else(|| "llama2".to_string()),
        };

        let client = self.client.unwrap_or_else(|| {
            Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"))
        });

        Ok(Ollama::new(config, client))
    }
}
