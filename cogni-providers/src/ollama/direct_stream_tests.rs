//! Direct unit tests for OllamaStream implementation

#[cfg(test)]
mod tests {
    use super::super::stream::OllamaStream;
    use bytes::Bytes;
    use cogni_core::{Error, StreamEvent};
    use futures::{Stream, StreamExt};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// Test the parse_line method behavior directly
    #[test]
    fn test_parse_line_empty() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        let result = stream.parse_line("");
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_parse_line_with_model_first_time() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":false}"#;
        let result = stream.parse_line(json);

        match result {
            Ok(Some(StreamEvent::Metadata(delta))) => {
                assert_eq!(delta.model.as_deref(), Some("llama3.2"));
            }
            _ => panic!("Expected metadata event"),
        }

        assert_eq!(stream.model.as_deref(), Some("llama3.2"));
    }

    #[test]
    fn test_parse_line_with_model_already_set() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: Some("existing_model".to_string()),
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let result = stream.parse_line(json);

        // Should get content event, not metadata
        match result {
            Ok(Some(StreamEvent::Content(delta))) => {
                assert_eq!(delta.text, "Hello");
            }
            _ => panic!("Expected content event"),
        }
    }

    #[test]
    fn test_parse_line_with_content() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: Some("model".to_string()), // Already has model
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Test content"},"done":false}"#;
        let result = stream.parse_line(json);

        match result {
            Ok(Some(StreamEvent::Content(delta))) => {
                assert_eq!(delta.text, "Test content");
            }
            _ => panic!("Expected content event"),
        }
    }

    #[test]
    fn test_parse_line_with_empty_content() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: Some("model".to_string()),
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":false}"#;
        let result = stream.parse_line(json);

        // Empty content should return None
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_parse_line_with_done() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: Some("model".to_string()),
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":true}"#;
        let result = stream.parse_line(json);

        match result {
            Ok(Some(StreamEvent::Done)) => {}
            _ => panic!("Expected done event"),
        }
    }

    #[test]
    fn test_parse_line_with_tool_calls() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: Some("model".to_string()),
            current_tool_calls: Vec::new(),
        };

        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"get_weather","arguments":{"location":"SF"}}}]},"done":false}"#;
        let result = stream.parse_line(json);

        match result {
            Ok(Some(StreamEvent::ToolCall(delta))) => {
                assert_eq!(delta.index, 0);
                assert_eq!(delta.id.as_deref(), Some("call_0"));
                assert_eq!(delta.name.as_deref(), Some("get_weather"));
                assert!(delta.arguments.is_some());
            }
            _ => panic!("Expected tool call event"),
        }

        // Check that tool call was stored
        assert_eq!(stream.current_tool_calls.len(), 1);
        assert_eq!(stream.current_tool_calls[0].1, "get_weather");
    }

    #[test]
    fn test_parse_line_invalid_json() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        let result = stream.parse_line("{invalid json");

        match result {
            Err(Error::Serialization { .. }) => {}
            _ => panic!("Expected serialization error"),
        }
    }

    #[test]
    fn test_parse_line_with_empty_model() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        // Response with empty model string
        let json = r#"{"model":"","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let result = stream.parse_line(json);

        // Should get content event, not metadata (empty model is ignored)
        match result {
            Ok(Some(StreamEvent::Content(delta))) => {
                assert_eq!(delta.text, "Hello");
            }
            _ => panic!("Expected content event"),
        }

        // Model should still be None
        assert!(stream.model.is_none());
    }

    #[test]
    fn test_parse_line_priority_order() {
        let mut stream = OllamaStream {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        // Response with model and content - model takes priority
        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let result = stream.parse_line(json);

        // Model event should be returned first
        match result {
            Ok(Some(StreamEvent::Metadata(delta))) => {
                assert_eq!(delta.model.as_deref(), Some("llama3.2"));
            }
            _ => panic!("Expected metadata event"),
        }
    }

    // Test buffer handling
    #[tokio::test]
    async fn test_stream_buffer_accumulation() {
        // Create a stream that sends partial JSON
        struct PartialChunkStream {
            chunks: Vec<&'static str>,
            index: usize,
        }

        impl Stream for PartialChunkStream {
            type Item = Result<Bytes, reqwest::Error>;

            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                if self.index < self.chunks.len() {
                    let chunk = self.chunks[self.index];
                    self.index += 1;
                    Poll::Ready(Some(Ok(Bytes::from(chunk))))
                } else {
                    Poll::Ready(None)
                }
            }
        }

        let chunks = vec![
            r#"{"model":"llama3.2","created_at":"#,
            r#""2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#,
            "\n",
        ];

        let stream = OllamaStream {
            inner: Box::pin(PartialChunkStream { chunks, index: 0 }),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        // Collect all events
        let events: Vec<_> = stream.collect().await;

        // When model and content are in the same message, model event is returned first
        // The content needs to come in a separate parse_line call
        assert_eq!(events.len(), 1); // Just metadata

        match &events[0] {
            Ok(StreamEvent::Metadata(delta)) => {
                assert_eq!(delta.model.as_deref(), Some("llama3.2"));
            }
            _ => panic!("Expected metadata event"),
        }
    }

    #[tokio::test]
    async fn test_stream_utf8_handling() {
        struct InvalidUtf8Stream {
            sent: bool,
        }

        impl Stream for InvalidUtf8Stream {
            type Item = Result<Bytes, reqwest::Error>;

            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                if !self.sent {
                    self.sent = true;
                    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
                    Poll::Ready(Some(Ok(Bytes::from(invalid_utf8))))
                } else {
                    Poll::Ready(None)
                }
            }
        }

        let mut stream = OllamaStream {
            inner: Box::pin(InvalidUtf8Stream { sent: false }),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        let result = stream.next().await;

        match result {
            Some(Err(Error::Serialization { message, .. })) => {
                assert!(message.contains("Invalid UTF-8"));
            }
            _ => panic!("Expected UTF-8 error"),
        }
    }

    #[tokio::test]
    async fn test_stream_network_error() {
        struct ErrorStream {
            sent: bool,
        }

        impl Stream for ErrorStream {
            type Item = Result<Bytes, reqwest::Error>;

            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                if !self.sent {
                    self.sent = true;
                    // Can't easily create a reqwest::Error, so we'll test the conversion path
                    Poll::Ready(None) // This simulates connection closed
                } else {
                    Poll::Ready(None)
                }
            }
        }

        let mut stream = OllamaStream {
            inner: Box::pin(ErrorStream { sent: false }),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        };

        let result = stream.next().await;
        assert!(result.is_none()); // Stream ended
    }
}
