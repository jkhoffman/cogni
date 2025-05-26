//! Logging middleware for request/response debugging

use crate::{BoxFuture, Layer, Service};
use cogni_core::{Error, Request, Response, StreamEvent};
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tracing::{debug, info, trace};

/// Logging middleware layer
#[derive(Debug, Clone, Default)]
pub struct LoggingLayer {
    /// Log level for the middleware
    pub level: LogLevel,
    /// Whether to log request/response content
    pub log_content: bool,
}

/// Log level for the middleware
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level logging
    Trace,
    /// Debug level logging
    #[default]
    Debug,
    /// Info level logging
    Info,
}

impl LoggingLayer {
    /// Create a new logging layer with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific log level
    pub fn with_level(level: LogLevel) -> Self {
        Self {
            level,
            log_content: false,
        }
    }

    /// Enable content logging
    pub fn with_content(mut self) -> Self {
        self.log_content = true;
        self
    }
}

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService {
            inner,
            level: self.level,
            log_content: self.log_content,
        }
    }
}

/// Logging middleware service
#[derive(Clone)]
pub struct LoggingService<S> {
    inner: S,
    level: LogLevel,
    log_content: bool,
}

impl<S> Service<Request> for LoggingService<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, request: Request) -> Self::Future {
        // Log the request
        match self.level {
            LogLevel::Trace => trace!(
                messages = request.messages.len(),
                model = %request.model,
                tools = request.tools.len(),
                "Processing LLM request"
            ),
            LogLevel::Debug => debug!(
                messages = request.messages.len(),
                model = %request.model,
                tools = request.tools.len(),
                "Processing LLM request"
            ),
            LogLevel::Info => info!(
                messages = request.messages.len(),
                model = %request.model,
                "Processing LLM request"
            ),
        }

        if self.log_content {
            for (i, msg) in request.messages.iter().enumerate() {
                debug!(
                    index = i,
                    role = ?msg.role,
                    content_type = match &msg.content {
                        cogni_core::Content::Text(_) => "text",
                        cogni_core::Content::Image(_) => "image",
                        cogni_core::Content::Audio(_) => "audio",
                        cogni_core::Content::Multiple(_) => "multiple",
                    },
                    "Request message"
                );

                if self.level == LogLevel::Trace {
                    if let cogni_core::Content::Text(text) = &msg.content {
                        trace!(content = %text, "Message content");
                    }
                }
            }
        }

        // Call the inner service
        let start_time = Instant::now();
        let level = self.level;
        let log_content = self.log_content;
        let mut inner = self.inner.clone();
        let fut = inner.call(request);

        Box::pin(async move {
            let response = fut.await?;
            let duration = start_time.elapsed();

            // Log the response
            match level {
                LogLevel::Trace => trace!(
                    content_length = response.content.len(),
                    tool_calls = response.tool_calls.len(),
                    model = ?response.metadata.model,
                    usage = ?response.metadata.usage,
                    finish_reason = ?response.metadata.finish_reason,
                    duration_ms = duration.as_millis(),
                    "Received LLM response"
                ),
                LogLevel::Debug => debug!(
                    content_length = response.content.len(),
                    tool_calls = response.tool_calls.len(),
                    usage = ?response.metadata.usage,
                    duration_ms = duration.as_millis(),
                    "Received LLM response"
                ),
                LogLevel::Info => info!(
                    content_length = response.content.len(),
                    tool_calls = response.tool_calls.len(),
                    duration_ms = duration.as_millis(),
                    "Received LLM response"
                ),
            }

            if log_content && level == LogLevel::Trace {
                trace!(content = %response.content, "Response content");

                for (i, call) in response.tool_calls.iter().enumerate() {
                    trace!(
                        index = i,
                        id = %call.id,
                        name = %call.name,
                        "Tool call"
                    );
                }
            }

            Ok(response)
        })
    }
}

/// A stream wrapper that logs events
pub struct LoggingStream {
    inner: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>,
    level: LogLevel,
    start_time: Instant,
}

impl Stream for LoggingStream {
    type Item = Result<StreamEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                if self.level == LogLevel::Trace {
                    match &event {
                        StreamEvent::Content(delta) => {
                            trace!(text_length = delta.text.len(), "Stream content delta");
                        }
                        StreamEvent::ToolCall(delta) => {
                            trace!(
                                index = delta.index,
                                has_id = delta.id.is_some(),
                                has_name = delta.name.is_some(),
                                has_args = delta.arguments.is_some(),
                                "Stream tool call delta"
                            );
                        }
                        StreamEvent::Metadata(delta) => {
                            trace!(
                                has_model = delta.model.is_some(),
                                has_id = delta.id.is_some(),
                                custom_fields = delta.custom.len(),
                                "Stream metadata delta"
                            );
                        }
                        StreamEvent::Done => {
                            let duration = self.start_time.elapsed();
                            debug!(duration_ms = duration.as_millis(), "Stream completed");
                        }
                    }
                }
                Poll::Ready(Some(Ok(event)))
            }
            other => other,
        }
    }
}

/// Re-export the layer for convenience
pub use LoggingLayer as LoggingMiddleware;

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{
        Content, FinishReason, Message, Model, Request, Response, ResponseMetadata, Role, Usage,
    };
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tracing_test::traced_test;

    /// Mock service for testing
    #[derive(Clone)]
    struct MockService {
        response: Response,
        call_count: Arc<AtomicUsize>,
    }

    impl Service<Request> for MockService {
        type Response = Response;
        type Error = Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

        fn call(&mut self, _request: Request) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let response = self.response.clone();
            Box::pin(async move {
                // Simulate some async work
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Ok(response)
            })
        }
    }

    fn create_test_request() -> Request {
        Request::builder()
            .model(Model("test-model".into()))
            .message(Message::user("Hello, AI!"))
            .message(Message::assistant("Hello! How can I help you?"))
            .build()
    }

    fn create_test_response() -> Response {
        Response {
            content: "I'm here to help!".to_string(),
            tool_calls: Vec::new(),
            metadata: ResponseMetadata {
                model: Some("test-model".into()),
                id: Some("test-123".into()),
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
                finish_reason: Some(FinishReason::Stop),
                custom: Default::default(),
            },
        }
    }

    #[test]
    fn test_logging_layer_creation() {
        let layer = LoggingLayer::new();
        assert_eq!(layer.level, LogLevel::Debug);
        assert!(!layer.log_content);

        let layer = LoggingLayer::with_level(LogLevel::Info);
        assert_eq!(layer.level, LogLevel::Info);
        assert!(!layer.log_content);

        let layer = LoggingLayer::with_level(LogLevel::Trace).with_content();
        assert_eq!(layer.level, LogLevel::Trace);
        assert!(layer.log_content);
    }

    #[test]
    fn test_log_level_default() {
        let level = LogLevel::default();
        assert_eq!(level, LogLevel::Debug);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_debug_level() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Debug);
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        let response = logging_service.call(request).await.unwrap();

        assert_eq!(response.content, "I'm here to help!");

        // Check that debug logs were emitted
        assert!(logs_contain("Processing LLM request"));
        assert!(logs_contain("Received LLM response"));
        assert!(logs_contain("messages=2"));
        assert!(logs_contain("model=test-model"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_info_level() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Info);
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        let response = logging_service.call(request).await.unwrap();

        assert_eq!(response.content, "I'm here to help!");

        // Info logs should not include tools count
        assert!(logs_contain("Processing LLM request"));
        assert!(logs_contain("messages=2"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_with_content() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Debug).with_content();
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        logging_service.call(request).await.unwrap();

        // Should log request message details
        assert!(logs_contain("Request message"));
        assert!(logs_contain("role=User"));
        assert!(logs_contain("content_type=\"text\""));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_trace_with_content() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Trace).with_content();
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        let _response = logging_service.call(request).await.unwrap();

        // Should log actual content at trace level
        assert!(logs_contain("Hello, AI!"));
        assert!(logs_contain("Response content"));
        assert!(logs_contain("I'm here to help!"));
    }

    #[tokio::test]
    async fn test_logging_service_error_propagation() {
        #[derive(Clone)]
        struct ErrorService;

        impl Service<Request> for ErrorService {
            type Response = Response;
            type Error = Error;
            type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

            fn call(&mut self, _request: Request) -> Self::Future {
                Box::pin(async move {
                    Err(Error::Network {
                        message: "Connection failed".into(),
                        source: None,
                    })
                })
            }
        }

        let layer = LoggingLayer::new();
        let mut logging_service = layer.layer(ErrorService);

        let request = create_test_request();
        let result = logging_service.call(request).await;

        assert!(result.is_err());
        if let Err(Error::Network { message, .. }) = result {
            assert_eq!(message, "Connection failed");
        } else {
            panic!("Expected Network error");
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_with_tools() {
        use cogni_core::{Tool, ToolCall};

        let mut response = create_test_response();
        response.tool_calls = vec![ToolCall {
            id: "tool-1".into(),
            name: "calculator".into(),
            arguments: r#"{"operation": "add", "a": 1, "b": 2}"#.into(),
        }];

        let mock_service = MockService {
            response,
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Trace).with_content();
        let mut logging_service = layer.layer(mock_service);

        let mut request = create_test_request();
        request.tools.push(Tool {
            name: "calculator".into(),
            description: "Performs calculations".into(),
            function: cogni_core::Function {
                parameters: serde_json::json!({}),
                returns: None,
            },
        });

        let response = logging_service.call(request).await.unwrap();

        assert_eq!(response.tool_calls.len(), 1);
        assert!(logs_contain("tools=1"));
        assert!(logs_contain("tool_calls=1"));
        assert!(logs_contain("Tool call"));
        assert!(logs_contain("name=calculator"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_usage_tracking() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Debug);
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        logging_service.call(request).await.unwrap();

        // Should log usage information
        assert!(logs_contain("usage=Some(Usage"));
        assert!(logs_contain("prompt_tokens: 10"));
        assert!(logs_contain("completion_tokens: 5"));
        assert!(logs_contain("total_tokens: 15"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_duration() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::new();
        let mut logging_service = layer.layer(mock_service);

        let request = create_test_request();
        logging_service.call(request).await.unwrap();

        // Should log duration
        assert!(logs_contain("duration_ms="));
    }

    #[test]
    fn test_logging_layer_clone() {
        let layer1 = LoggingLayer::with_level(LogLevel::Info).with_content();
        let layer2 = layer1.clone();

        assert_eq!(layer1.level, layer2.level);
        assert_eq!(layer1.log_content, layer2.log_content);
    }

    // Tests for different content types
    #[tokio::test]
    #[traced_test]
    async fn test_logging_service_different_content_types() {
        use cogni_core::{Audio, Image};

        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
        };

        let layer = LoggingLayer::with_level(LogLevel::Debug).with_content();
        let mut logging_service = layer.layer(mock_service);

        let request = Request::builder()
            .model(Model("test-model".into()))
            .message(Message {
                role: Role::User,
                content: Content::Image(Image {
                    url: Some("https://example.com/image.jpg".into()),
                    data: None,
                    mime_type: "image/jpeg".into(),
                }),
                metadata: Default::default(),
            })
            .message(Message {
                role: Role::User,
                content: Content::Audio(Audio {
                    data: "base64-audio".into(),
                    mime_type: "audio/mpeg".into(),
                }),
                metadata: Default::default(),
            })
            .message(Message {
                role: Role::User,
                content: Content::Multiple(vec![
                    Content::Text("Look at this:".into()),
                    Content::Image(Image {
                        url: Some("https://example.com/pic.jpg".into()),
                        data: None,
                        mime_type: "image/jpeg".into(),
                    }),
                ]),
                metadata: Default::default(),
            })
            .build();

        logging_service.call(request).await.unwrap();

        assert!(logs_contain("content_type=\"image\""));
        assert!(logs_contain("content_type=\"audio\""));
        assert!(logs_contain("content_type=\"multiple\""));
    }
}
