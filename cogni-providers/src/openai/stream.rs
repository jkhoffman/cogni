//! Streaming implementation for OpenAI

use crate::openai::parser::OpenAIParser;
use crate::traits::StreamEventParser;
use cogni_core::{Error, StreamEvent};
use futures::Stream;
use reqwest_eventsource::{Event, EventSource};
use std::pin::Pin;
use std::task::{Context, Poll};

/// OpenAI streaming response
pub struct OpenAIStream {
    inner: EventSource,
    parser: OpenAIParser,
}

impl OpenAIStream {
    /// Create a new OpenAI stream
    pub fn new(event_source: EventSource) -> Self {
        Self {
            inner: event_source,
            parser: OpenAIParser,
        }
    }
}

impl Stream for OpenAIStream {
    type Item = Result<StreamEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(Event::Open))) => {
                // Connection opened, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Ok(Event::Message(msg)))) => {
                // Parse the message data
                match self.parser.parse_event(&format!("data: {}", msg.data)) {
                    Ok(Some(event)) => Poll::Ready(Some(Ok(event))),
                    Ok(None) => {
                        // No event from this message, continue polling
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Some(Err(e))),
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(Error::Network {
                message: format!("EventSource error: {}", e),
                source: None,
            }))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
