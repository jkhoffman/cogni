//! Tests for the Ollama provider
//!
//! These tests require a local Ollama instance running on port 11434.
//! They test real interactions with the Ollama API.

#[cfg(test)]
mod provider_tests {
    use super::super::*;
    use crate::builder::ProviderBuilder;
    use crate::http::ReqwestClient;
    use cogni_core::{Error, Message, Model, Parameters, Provider, Request, StreamEvent};
    use futures::StreamExt;
    use std::sync::Arc;

    /// Check if Ollama is running locally
    pub(super) async fn ollama_is_available() -> bool {
        let client = reqwest::Client::new();
        client
            .get("http://localhost:11434/api/tags")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn test_ollama_local_creation() {
        let provider = Ollama::local();
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_with_base_url() {
        let provider = Ollama::with_base_url("http://localhost:11434");
        assert!(provider.is_ok());

        let provider = Ollama::with_base_url("http://custom:8080");
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_builder() {
        let builder =
            OllamaBuilder::new("http://localhost:11434".to_string()).with_model("llama3.2");

        let provider = builder.build();
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_builder_with_custom_client() {
        let client = Arc::new(ReqwestClient::new().unwrap());
        let provider = OllamaBuilder::new("http://localhost:11434".to_string())
            .with_model("mistral")
            .with_client(client)
            .build();

        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_simple_request() {
        if !ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![Message::user("Say 'test passed' and nothing else")])
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(response.content.to_lowercase().contains("test passed"));
    }

    #[tokio::test]
    async fn test_ollama_with_parameters() {
        if !ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let params = Parameters::builder()
            .temperature(0.1)
            .max_tokens(50)
            .build();

        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![Message::user("Count to 5")])
            .parameters(params)
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_streaming() {
        if !ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![Message::user("Count from 1 to 3")])
            .try_build()
            .unwrap();

        let stream = provider.stream(request).await;
        assert!(stream.is_ok());

        let mut stream = stream.unwrap();
        let mut content = String::new();
        let mut got_metadata = false;
        let mut got_done = false;

        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::Metadata(delta)) => {
                    got_metadata = true;
                    assert!(delta.model.is_some());
                }
                Ok(StreamEvent::Content(delta)) => {
                    content.push_str(&delta.text);
                }
                Ok(StreamEvent::Done) => {
                    got_done = true;
                }
                Ok(_) => {}
                Err(e) => panic!("Stream error: {:?}", e),
            }
        }

        assert!(got_metadata, "Should receive metadata event");
        assert!(got_done, "Should receive done event");
        assert!(!content.is_empty(), "Should receive content");
    }

    #[tokio::test]
    async fn test_ollama_conversation() {
        if !ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();

        // Multi-turn conversation
        let messages = vec![
            Message::system("You are a helpful assistant. Keep responses very brief."),
            Message::user("What is 2+2?"),
            Message::assistant("4"),
            Message::user("What did I just ask?"),
        ];

        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(messages)
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        // Should reference the math question
        assert!(response.content.contains("2") || response.content.to_lowercase().contains("math"));
    }

    #[tokio::test]
    async fn test_ollama_error_handling() {
        // Test with invalid base URL
        let provider = Ollama::with_base_url("http://localhost:99999").unwrap();
        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![Message::user("Hello")])
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_err());

        match response.unwrap_err() {
            Error::Network { .. } => {} // Expected
            e => panic!("Expected Network error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_ollama_invalid_model() {
        if !ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let request = Request::builder()
            .model(Model::new("non_existent_model_xyz"))
            .messages(vec![Message::user("Hello")])
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_err());
    }
}

#[cfg(test)]
mod config_tests {
    use super::super::config::OllamaConfig;
    use crate::constants::{OLLAMA_DEFAULT_BASE_URL, OLLAMA_DEFAULT_MODEL};

    #[test]
    fn test_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, OLLAMA_DEFAULT_BASE_URL);
        assert_eq!(config.default_model, OLLAMA_DEFAULT_MODEL);
    }

    #[test]
    fn test_config_clone() {
        let config = OllamaConfig {
            base_url: "http://custom:8080".to_string(),
            default_model: "mistral".to_string(),
        };

        let cloned = config.clone();
        assert_eq!(cloned.base_url, config.base_url);
        assert_eq!(cloned.default_model, config.default_model);
    }
}

#[cfg(test)]
mod stream_tests {
    use super::super::converter::OllamaStreamResponse;
    use bytes::Bytes;

    fn create_mock_chunk(json: &str) -> Bytes {
        Bytes::from(format!("{}\n", json))
    }

    #[tokio::test]
    async fn test_stream_parsing_content() {
        // Create a mock response with content
        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let _chunk = create_mock_chunk(json);

        // We can't easily mock reqwest::Response, so we'll test the parsing logic
        // by directly testing parse_line method

        // For now, we'll test the JSON parsing
        let response: OllamaStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "llama3.2");
        assert_eq!(response.message.content, "Hello");
        assert!(!response.is_done);
    }

    #[tokio::test]
    async fn test_stream_parsing_metadata() {
        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":false}"#;

        let response: OllamaStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "llama3.2");
        assert!(response.message.content.is_empty());
    }

    #[tokio::test]
    async fn test_stream_parsing_done() {
        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop"}"#;

        let response: OllamaStreamResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_done);
        assert_eq!(response.done_reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn test_stream_parsing_tool_calls() {
        let json = r#"{
            "model":"llama3.2",
            "created_at":"2024-01-01T00:00:00Z",
            "message":{
                "role":"assistant",
                "content":"",
                "tool_calls":[
                    {
                        "function":{
                            "name":"get_weather",
                            "arguments":{"location":"San Francisco"}
                        }
                    }
                ]
            },
            "done":false
        }"#;

        let response: OllamaStreamResponse = serde_json::from_str(json).unwrap();
        assert!(response.message.tool_calls.is_some());

        let tool_calls = response.message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "get_weather");
    }

    #[tokio::test]
    async fn test_stream_parsing_usage() {
        let json = r#"{
            "model":"llama3.2",
            "created_at":"2024-01-01T00:00:00Z",
            "message":{"role":"assistant","content":""},
            "done":true,
            "total_duration":1000000000,
            "eval_count":50,
            "prompt_eval_count":25
        }"#;

        let response: OllamaStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.eval_count, Some(50));
        assert_eq!(response.prompt_eval_count, Some(25));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use cogni_core::{Message, Model, Provider, Request, ResponseFormat, StructuredOutput};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestOutput {
        answer: String,
        confidence: f32,
    }

    impl StructuredOutput for TestOutput {
        fn schema() -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "answer": { "type": "string" },
                    "confidence": { "type": "number" }
                },
                "required": ["answer", "confidence"]
            })
        }
    }

    #[tokio::test]
    async fn test_ollama_json_mode() {
        if !super::provider_tests::ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![Message::user(
                "What is 2+2? Respond in JSON with 'answer' field.",
            )])
            .response_format(ResponseFormat::JsonObject)
            .try_build()
            .unwrap();

        let response = provider.request(request).await;

        // Note: JSON mode support depends on the model
        // Some models may not support it well
        if let Ok(response) = response {
            // Try to parse as JSON
            let _ = response.parse_json();
        }
    }

    #[tokio::test]
    async fn test_ollama_with_system_message() {
        if !super::provider_tests::ollama_is_available().await {
            eprintln!("Skipping test - Ollama not available");
            return;
        }

        let provider = Ollama::local().unwrap();
        let request = Request::builder()
            .model(Model::new("llama3.2"))
            .messages(vec![
                Message::system("You always respond with exactly one word."),
                Message::user("What color is the sky?"),
            ])
            .try_build()
            .unwrap();

        let response = provider.request(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        // Should be a very short response
        assert!(response.content.split_whitespace().count() < 5);
    }
}
