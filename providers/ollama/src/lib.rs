//! Ollama provider for the Cogni framework.
//!
//! This crate provides an implementation of the `LanguageModel` trait for Ollama's local models.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::{
    error::LlmError,
    llm::{GenerateOptions, LanguageModel},
};
use futures::Stream;
use pin_project::pin_project;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::instrument;

/// A stream of chat responses from the Ollama API.
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

/// Configuration for the Ollama client.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// The model to use (e.g., "llama2", "mistral")
    model: String,
    /// Base URL for the API (defaults to "http://localhost:11434")
    base_url: String,
    /// HTTP client configuration
    client: Option<Client>,
}

impl OllamaConfig {
    /// Create a new configuration with the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: "http://localhost:11434".to_string(),
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

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
    done: bool,
}

/// The Ollama language model client.
pub struct OllamaClient {
    config: OllamaConfig,
    client: Client,
}

impl OllamaClient {
    /// Create a new Ollama client with the given configuration.
    pub fn new(mut config: OllamaConfig) -> Result<Self, LlmError> {
        let client = config.client.take().unwrap_or_else(|| {
            Client::builder()
                .build()
                .expect("Failed to build HTTP client")
        });

        Ok(Self { config, client })
    }

    fn build_request(&self, messages: Vec<ChatMessage>, stream: bool) -> GenerateRequest {
        // Convert chat messages to a single prompt string
        let prompt = messages
            .into_iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n");

        GenerateRequest {
            model: self.config.model.clone(),
            prompt,
            stream,
        }
    }

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

    fn parse_stream_chunk(bytes: &[u8]) -> Option<Result<String, LlmError>> {
        match serde_json::from_slice::<GenerateResponse>(bytes) {
            Ok(response) => {
                if response.done {
                    None
                } else {
                    Some(Ok(response.response))
                }
            }
            Err(e) => Some(Err(LlmError::InvalidResponse(e.to_string()))),
        }
    }
}

#[async_trait]
impl LanguageModel for OllamaClient {
    type Prompt = Vec<ChatMessage>;
    type Response = ChatMessage;
    type TokenStream = ChatStream;

    #[instrument(name = "ollama_generate", skip_all, err)]
    async fn generate(
        &self,
        prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        let request = self.build_request(prompt, false);
        let url = format!("{}/api/generate", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(LlmError::RequestFailed)?;

        if !response.status().is_success() {
            let error = response.text().await.map_err(LlmError::RequestFailed)?;
            return Err(LlmError::ApiError(error));
        }

        let generate_response: GenerateResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content: generate_response.response,
        })
    }

    #[instrument(name = "ollama_stream_generate", skip_all, err)]
    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        let request = self.build_request(prompt, true);
        let url = format!("{}/api/generate", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(LlmError::RequestFailed)?;

        if !response.status().is_success() {
            let error = response.text().await.map_err(LlmError::RequestFailed)?;
            return Err(LlmError::ApiError(error));
        }

        let stream = Self::process_stream_response(response).await?;
        Ok(Box::pin(ChatStream {
            inner: Box::pin(stream),
        }))
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    #[tokio::test]
    async fn test_client_creation() {
        let config = OllamaConfig::new("llama2");
        let client = OllamaClient::new(config).unwrap();
        assert_eq!(client.name(), "ollama");
    }

    #[tokio::test]
    async fn test_generate_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "response": "Hello, world!",
                "done": true
            })))
            .mount(&mock_server)
            .await;

        let config = OllamaConfig::new("llama2").with_base_url(mock_server.uri());
        let client = OllamaClient::new(config).unwrap();

        let prompt = vec![ChatMessage {
            role: "user".to_string(),
            content: "Say hello".to_string(),
        }];

        let response = client
            .generate(prompt, GenerateOptions::default())
            .await
            .unwrap();
        assert_eq!(response.content, "Hello, world!");
    }
}
