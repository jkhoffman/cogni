//! OpenAI provider implementation
//!
//! This module provides integration with OpenAI's API, supporting both
//! chat completions and streaming responses. It implements the core `Provider`
//! trait and uses the standardized request/response conversion pipeline.

use crate::http::{create_headers, HttpClient, ReqwestClient};
use crate::openai::{
    config::OpenAIConfig, converter::OpenAIConverter, parser::OpenAIParser, stream::OpenAIStream,
};
use crate::traits::{RequestConverter, ResponseParser};
use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response};
use std::sync::Arc;

/// OpenAI provider for chat completions
///
/// This provider supports:
/// - GPT-4 and GPT-3.5 models
/// - Function/tool calling
/// - Structured output with JSON mode
/// - Streaming responses
/// - Custom Azure OpenAI deployments
///
/// # Example
///
/// ```no_run
/// use cogni_providers::OpenAI;
///
/// // Create with API key
/// let provider = OpenAI::with_api_key("your-api-key");
///
/// // Or with custom configuration and client
/// use cogni_providers::openai::OpenAIConfig;
/// use cogni_providers::http::{HttpClient, ReqwestClient};
/// use std::sync::Arc;
///
/// let config = OpenAIConfig::new("your-api-key")
///     .with_organization("org-id");
/// let client = Arc::new(ReqwestClient::new().expect("Failed to create client"));
/// let provider = OpenAI::new(config, client);
/// ```
#[derive(Clone)]
pub struct OpenAI {
    client: Arc<dyn HttpClient>,
    config: OpenAIConfig,
    converter: OpenAIConverter,
    parser: OpenAIParser,
}

impl OpenAI {
    /// Create a new OpenAI provider with the given configuration and client
    pub fn new(config: OpenAIConfig, client: Arc<dyn HttpClient>) -> Self {
        Self {
            client,
            config,
            converter: OpenAIConverter,
            parser: OpenAIParser,
        }
    }

    /// Create a new OpenAI provider with just an API key
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let client = Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"));
        Self::new(OpenAIConfig::new(api_key), client)
    }
}

#[async_trait]
impl Provider for OpenAI {
    type Stream = OpenAIStream;

    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(false);

        let headers = create_headers(&self.config.api_key, None)?;
        let response = self
            .client
            .post(&self.config.chat_url(), headers, body)
            .await?;

        self.parser.parse_response(response).await
    }

    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(true);

        let headers = create_headers(&self.config.api_key, None)?;
        let url = self.config.chat_url();

        let event_source = self.client.post_event_stream(&url, headers, body).await?;

        Ok(OpenAIStream::new(event_source))
    }
}
