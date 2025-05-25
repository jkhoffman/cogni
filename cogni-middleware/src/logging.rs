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
pub struct LoggingService<S> {
    inner: S,
    level: LogLevel,
    log_content: bool,
}

impl<S> Service<Request> for LoggingService<S>
where
    S: Service<Request, Response = Response, Error = Error>,
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
        let fut = self.inner.call(request);

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
