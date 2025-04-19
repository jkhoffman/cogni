//! OpenAI provider for the Cogni framework.
//!
//! This crate provides an implementation of the `LanguageModel` trait for OpenAI's GPT models.

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

/// A stream of chat completion tokens from the OpenAI API.
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

/// Configuration for the OpenAI client.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// The API key for authentication
    pub api_key: String,
    /// Base URL for the API (defaults to "https://api.openai.com/v1")
    pub base_url: String,
    /// The model to use
    pub model: String,
    /// HTTP client configuration
    pub client: Option<Client>,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: String::new(),
            client: None,
        }
    }
}

impl OpenAiConfig {
    /// Create a new configuration with the given API key and model.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
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

/// Request body for chat completions.
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

/// Response from the chat completions API.
/// Note: Some fields are kept for API compatibility and debugging even if not actively used.
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    /// The unique identifier for this completion. Kept for debugging purposes.
    #[allow(dead_code)]
    id: String,
    /// The generated completions.
    choices: Vec<ChatCompletionChoice>,
}

/// A choice in a chat completion response.
/// Note: Some fields are kept for API compatibility and debugging even if not actively used.
#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    /// The generated message.
    message: ChatMessage,
    /// The reason why the completion finished. Kept for debugging purposes.
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

/// A chunk in a streaming chat completion response.
#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChatCompletionStreamChoice>,
}

/// A choice in a streaming chat completion response.
#[derive(Debug, Deserialize)]
struct ChatCompletionStreamChoice {
    delta: ChatMessageDelta,
}

/// A delta in a streaming chat message.
#[derive(Debug, Deserialize)]
struct ChatMessageDelta {
    content: Option<String>,
}

/// The OpenAI language model client.
pub struct OpenAiClient {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiClient {
    /// Create a new OpenAI client with the given configuration.
    pub fn new(mut config: OpenAiConfig) -> Result<Self, LlmError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|e| LlmError::ConfigError(e.to_string()))?,
        );

        let client = config.client.take().unwrap_or_else(|| {
            Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build HTTP client")
        });

        Ok(Self { client, config })
    }

    /// Build a chat completion request from messages and options.
    fn build_request(
        &self,
        messages: Vec<ChatMessage>,
        opts: &GenerateOptions,
        stream: bool,
    ) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: opts.max_tokens,
            temperature: opts.temperature,
            stream,
        }
    }

    async fn process_stream_response(
        response: reqwest::Response,
    ) -> Result<impl Stream<Item = Result<String, LlmError>> + Send + 'static, LlmError> {
        let stream = futures::stream::unfold(response, |mut response| async move {
            match response.chunk().await {
                Ok(Some(chunk)) => Self::parse_stream_chunk(&chunk).map(|result| (result, response)),
                Ok(None) => None,
                Err(e) => Some((Err(LlmError::RequestFailed(e)), response)),
            }
        });

        Ok(stream)
    }

    fn parse_stream_chunk(bytes: &[u8]) -> Option<Result<String, LlmError>> {
        let text = match String::from_utf8(bytes.to_vec()) {
            Ok(text) => text,
            Err(e) => return Some(Err(LlmError::InvalidResponse(e.to_string()))),
        };

        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line["data: ".len()..];
                if data == "[DONE]" {
                    return None;
                }

                match serde_json::from_str::<ChatCompletionChunk>(data) {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                return Some(Ok(content.to_string()));
                            }
                        }
                    }
                    Err(e) => return Some(Err(LlmError::InvalidResponse(e.to_string()))),
                }
            }
        }

        None
    }
}

#[async_trait]
impl LanguageModel for OpenAiClient {
    type Prompt = Vec<ChatMessage>;
    type Response = ChatMessage;
    type TokenStream = ChatStream;

    #[instrument(name = "openai_generate", skip_all, err)]
    async fn generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        let request = self.build_request(prompt, &opts, false);
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(LlmError::RequestFailed)?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .map_err(LlmError::RequestFailed)?;
            return Err(LlmError::ApiError(error));
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        completion
            .choices
            .first()
            .map(|choice| choice.message.clone())
            .ok_or_else(|| LlmError::InvalidResponse("No completion choices returned".to_string()))
    }

    #[instrument(name = "openai_stream_generate", skip_all, err)]
    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        let request = self.build_request(prompt, &opts, true);
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(LlmError::RequestFailed)?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .map_err(LlmError::RequestFailed)?;
            return Err(LlmError::ApiError(error));
        }

        let stream = Self::process_stream_response(response).await?;
        Ok(Box::pin(ChatStream {
            inner: Box::pin(stream),
        }))
    }

    fn name(&self) -> &'static str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    #[tokio::test]
    async fn test_generate_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test",
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Hello, world!"
                    },
                    "finish_reason": "stop"
                }]
            })))
            .mount(&mock_server)
            .await;

        let config = OpenAiConfig::new("test_key", "gpt-4").with_base_url(mock_server.uri());

        let client = OpenAiClient::new(config).unwrap();

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

    #[tokio::test]
    async fn test_stream_generate_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_string(concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\", \"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"!\"}}]}\n\n",
                "data: [DONE]\n\n"
            )))
            .mount(&mock_server)
            .await;

        let config = OpenAiConfig::new("test_key", "gpt-4").with_base_url(mock_server.uri());
        let client = OpenAiClient::new(config).unwrap();

        let prompt = vec![ChatMessage {
            role: "user".to_string(),
            content: "Say hello".to_string(),
        }];

        let mut stream = client
            .stream_generate(prompt, GenerateOptions::default())
            .await
            .unwrap();

        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk.unwrap());
        }

        assert_eq!(result, "Hello, world!");
    }
}
