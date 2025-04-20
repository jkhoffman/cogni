//! Anthropic provider for the Cogni framework.
//!
//! This crate provides an implementation of the `LanguageModel` trait for Anthropic's Claude models.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::error::LlmError;
use cogni_core::traits::llm::{GenerateOptions, LanguageModel};
use futures::Stream;
use futures::TryStreamExt;
use pin_project::pin_project;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::instrument;

/// A stream of chat responses from the Anthropic API.
/// This struct implements the Stream trait to provide asynchronous access to streaming responses.
#[pin_project]
pub struct ChatStream {
    #[pin]
    es: EventSource,
}

impl Stream for ChatStream {
    type Item = Result<String, LlmError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Loop until we have a definitive result (Ready) or the underlying source is Pending.
        loop {
            match this.es.as_mut().poll_next(cx) {
                // Use poll_next directly, no need for unpin
                std::task::Poll::Ready(Some(Ok(event))) => match event {
                    Event::Message(msg) => {
                        match serde_json::from_str::<AnthropicStreamEvent>(&msg.data) {
                            Ok(AnthropicStreamEvent::ContentBlockDelta { delta, .. }) => {
                                // Found text, return it immediately.
                                return std::task::Poll::Ready(Some(Ok(delta.text)));
                            }
                            Ok(AnthropicStreamEvent::MessageStop { .. }) => {
                                // End of stream signaled by API.
                                return std::task::Poll::Ready(None);
                            }
                            Ok(AnthropicStreamEvent::Error { error }) => {
                                // API reported an error.
                                return std::task::Poll::Ready(Some(Err(LlmError::ApiError(
                                    format!(
                                        "Anthropic stream error ({}): {}",
                                        error.error_type, error.message
                                    ),
                                ))));
                            }
                            Ok(_) => {
                                // Other valid events (MessageStart, Ping, etc.), ignore and poll again.
                                continue;
                            }
                            Err(e) => {
                                // Parsing error.
                                tracing::warn!(
                                    "Failed to parse Anthropic message data: {}, data: {}",
                                    e,
                                    msg.data
                                );
                                return std::task::Poll::Ready(Some(Err(
                                    LlmError::InvalidResponse(format!(
                                        "Failed to parse stream message data: {}",
                                        e
                                    )),
                                )));
                            }
                        }
                    }
                    Event::Open => {
                        // SSE stream opened, ignore and poll again.
                        continue;
                    }
                },
                std::task::Poll::Ready(Some(Err(e))) => {
                    // Error from the underlying reqwest_eventsource stream.
                    // Check if it's just the stream ending normally.
                    if e.to_string().contains("Stream ended") {
                        return std::task::Poll::Ready(None);
                    }
                    // Otherwise, it's an API/network error.
                    return std::task::Poll::Ready(Some(Err(LlmError::ApiError(e.to_string()))));
                }
                std::task::Poll::Ready(None) => {
                    // Underlying EventSource stream ended.
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Pending => {
                    // Underlying source is not ready, propagate Pending.
                    return std::task::Poll::Pending;
                }
            }
        }
    }
}

/// Configuration for the Anthropic client.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// The API key for authentication
    pub api_key: String,
    /// Base URL for the API (defaults to "<https://api.anthropic.com/v1>")
    pub base_url: String,
    /// The model to use
    pub model: String,
    /// Optional system prompt to use
    pub system_prompt: Option<String>,
    /// HTTP client configuration
    pub client: Option<Client>,
}

impl AnthropicConfig {
    /// Create a new configuration with the given API key and model.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            system_prompt: None,
            client: None,
        }
    }

    /// Set a system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
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
    /// The role of the message sender ("user" or "assistant")
    pub role: String,
    /// The content of the message
    pub content: String, // Anthropic's API might use a list of content blocks, simplify for now
}

/// Request body for the Anthropic Messages API.
#[derive(Debug, Serialize)]
struct AnthropicMessagesRequest {
    model: String,
    messages: Vec<ChatMessage>,
    system: Option<String>, // Optional system prompt
    max_tokens: u32,        // Required by Anthropic API
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    stream: bool,
}

/// Response body for the Anthropic Messages API (non-streaming).
#[derive(Debug, Deserialize, Serialize)]
struct AnthropicMessagesResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String, // e.g., "message"
    role: String, // e.g., "assistant"
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: AnthropicUsageInfo,
}

/// Content block in an Anthropic response.
#[derive(Debug, Deserialize, Serialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String, // e.g., "text"
    text: String,
}

/// Usage information from the Anthropic API.
#[derive(Debug, Deserialize, Serialize)]
struct AnthropicUsageInfo {
    input_tokens: u32,
    output_tokens: u32,
}

/// Represents events in the Anthropic streaming response.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamEvent {
    MessageStart {
        message: AnthropicMessagesResponse,
    },
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: AnthropicTextDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: AnthropicUsageDelta,
        usage: AnthropicUsageInfo,
    },
    MessageStop {},
    Ping {}, // Keepalive event
    Error {
        error: AnthropicApiError,
    },
}

/// Text delta within a streaming event.
#[derive(Debug, Deserialize)]
struct AnthropicTextDelta {
    #[allow(dead_code)] // Type might be useful later for differentiation
    #[serde(rename = "type")]
    delta_type: String, // e.g., "text_delta"
    text: String,
}

/// Usage delta within a streaming event.
#[allow(dead_code)] // Potentially useful for tracking stream tokens
#[derive(Debug, Deserialize)]
struct AnthropicUsageDelta {
    output_tokens: u32,
}

/// Error details from the Anthropic API.
#[derive(Debug, Deserialize, Serialize)]
struct AnthropicApiError {
    #[allow(dead_code)] // Type might be useful later for differentiation
    #[serde(rename = "type")]
    error_type: String, // e.g., "error"
    message: String,
}

/// Helper struct to parse the 'data' field of a 'content_block_delta' event.
#[derive(Debug, Deserialize)]
struct ContentBlockDeltaData {
    // We only care about the nested delta
    delta: AnthropicTextDelta,
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
}

#[async_trait]
impl LanguageModel for AnthropicClient {
    type Prompt = Vec<ChatMessage>;
    type Response = ChatMessage;
    type TokenStream = ChatStream;

    #[instrument(name = "anthropic_generate", skip_all, err)]
    async fn generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        let request_body = AnthropicMessagesRequest {
            model: self.config.model.clone(),
            messages: prompt,
            system: self.config.system_prompt.clone(),
            max_tokens: opts.max_tokens.unwrap_or(1024), // Anthropic requires max_tokens
            temperature: opts.temperature,
            stream: false,
            // Add other optional parameters if needed (top_p, top_k, stop_sequences)
            stop_sequences: None,
            top_p: None,
            top_k: None,
        };

        let url = format!("{}/messages", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(LlmError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            // Try to parse AnthropicApiError
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            let specific_error = serde_json::from_str::<AnthropicApiError>(&error_text)
                .map(|e| e.message)
                .unwrap_or(error_text);

            return Err(LlmError::ApiError(format!(
                "Anthropic API error ({status}): {specific_error}"
            )));
        }

        let response_body: AnthropicMessagesResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Log token usage
        tracing::debug!(
            input_tokens = response_body.usage.input_tokens,
            output_tokens = response_body.usage.output_tokens,
            "Anthropic token usage"
        );

        // For simplicity, concatenate text from all content blocks.
        // A more robust implementation might handle different block types or structure.
        let combined_content = response_body
            .content
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<&str>>()
            .join("");

        Ok(ChatMessage {
            role: response_body.role, // Should be "assistant"
            content: combined_content,
        })
    }

    #[instrument(name = "anthropic_stream_generate", skip_all, err)]
    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        let request_body = AnthropicMessagesRequest {
            model: self.config.model.clone(),
            messages: prompt,
            system: self.config.system_prompt.clone(),
            max_tokens: opts.max_tokens.unwrap_or(1024), // Required
            temperature: opts.temperature,
            stream: true,
            stop_sequences: None,
            top_p: None,
            top_k: None,
        };

        let url = format!("{}/messages", self.config.base_url);

        // The internal client self.client is already configured with default headers (API key, version)
        // in the `new` function. No need to set them again here.
        let request_builder = self
            .client
            .post(&url)
            // .headers(headers) // Remove explicit header setting
            .json(&request_body);

        // Create EventSource from RequestBuilder
        let es = request_builder
            .eventsource()
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        Ok(Box::pin(ChatStream { es }))
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }

    fn create_test_config(server: &MockServer) -> AnthropicConfig {
        AnthropicConfig::new("test_key", "claude-test").with_base_url(server.uri())
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = AnthropicConfig::new("test_key", "claude-3-opus-20240229");
        let client = AnthropicClient::new(config).unwrap();
        assert_eq!(client.name(), "anthropic");
    }

    #[tokio::test]
    async fn test_generate_success() {
        let server = setup_mock_server().await;
        let config = create_test_config(&server);
        let client = AnthropicClient::new(config).unwrap();

        let mock_response = AnthropicMessagesResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![AnthropicContentBlock {
                content_type: "text".to_string(),
                text: "Hello!".to_string(),
            }],
            model: "claude-test".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: AnthropicUsageInfo {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        let prompt = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }];
        let result = client.generate(prompt, GenerateOptions::default()).await;

        assert!(result.is_ok());
        let response_message = result.unwrap();
        assert_eq!(response_message.role, "assistant");
        assert_eq!(response_message.content, "Hello!");
    }

    #[tokio::test]
    async fn test_stream_generate_success() {
        // Initialize tracing for test output
        let _ = fmt::try_init();

        let server = setup_mock_server().await;
        let config = create_test_config(&server);

        // Create AnthropicClient with the test config
        let anthropic_client = AnthropicClient::new(config.clone()).unwrap();

        // Mock SSE stream data with proper SSE format (no escaped backslashes)
        let stream_body = "event: message_start\ndata: {\"type\": \"message_start\", \"message\": {\"id\": \"msg_stream\", \"type\": \"message\", \"role\": \"assistant\", \"content\": [], \"model\": \"claude-test\", \"usage\": {\"input_tokens\": 8, \"output_tokens\": 1}}}\n\nevent: content_block_start\ndata: {\"type\": \"content_block_start\", \"index\": 0, \"content_block\": {\"type\": \"text\", \"text\": \"\"}}\n\nevent: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello\"}}\n\nevent: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" world!\"}}\n\nevent: content_block_stop\ndata: {\"type\": \"content_block_stop\", \"index\": 0}\n\nevent: message_delta\ndata: {\"type\": \"message_delta\", \"delta\": {\"output_tokens\": 10}, \"usage\": {\"input_tokens\": 8, \"output_tokens\": 10}}\n\nevent: message_stop\ndata: {\"type\": \"message_stop\"}\n";

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(stream_body, "text/event-stream"))
            .mount(&server)
            .await;

        // Call stream_generate using the AnthropicClient instance
        let stream_result = anthropic_client
            .stream_generate(
                vec![ChatMessage {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                }],
                GenerateOptions::default(),
            )
            .await;

        assert!(
            stream_result.is_ok(),
            "stream_generate failed: {:?}",
            stream_result.err()
        );
        let mut stream = stream_result.unwrap();

        let mut full_response = String::new();
        // Add a timeout to prevent hanging
        let test_timeout = std::time::Duration::from_secs(5);
        match tokio::time::timeout(test_timeout, async {
            while let Some(chunk) = stream.try_next().await? {
                full_response.push_str(&chunk);
            }
            // try_next returns Result, so wrap the final response in Ok
            Ok::<_, LlmError>(full_response)
        })
        .await
        {
            // Handle the outer timeout result
            Ok(Ok(final_response)) => {
                // Both timeout and stream succeeded
                assert_eq!(final_response, "Hello world!");
            }
            Ok(Err(e)) => {
                // Stream failed
                panic!("Stream processing failed: {:?}", e);
            }
            Err(_) => {
                // Timeout occurred
                panic!("Stream processing timed out after {:?}", test_timeout);
            }
        }
    }

    #[tokio::test]
    async fn test_generate_api_error() {
        let server = setup_mock_server().await;
        let config = create_test_config(&server);
        let client = AnthropicClient::new(config).unwrap();

        let error_body = AnthropicApiError {
            error_type: "invalid_request_error".to_string(),
            message: "Invalid API key".to_string(),
        };

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
            .mount(&server)
            .await;

        let prompt = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }];
        let result = client.generate(prompt, GenerateOptions::default()).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            LlmError::ApiError(msg) => {
                assert!(msg.contains("Anthropic API error (401 Unauthorized)"));
                assert!(msg.contains("Invalid API key"));
            }
            _ => panic!("Expected ApiError"),
        }
    }
}
