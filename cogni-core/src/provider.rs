//! Core provider trait for LLM interactions

use crate::error::Result;
use crate::types::request::Request;
use crate::types::response::Response;
use crate::types::stream::StreamEvent;
use async_trait::async_trait;

/// The fundamental trait for LLM interactions
///
/// This trait defines the core interface that all LLM providers must implement.
/// It supports both request/response and streaming interactions.
#[async_trait]
pub trait Provider: Send + Sync {
    /// The stream type returned by this provider
    type Stream: futures_core::Stream<Item = Result<StreamEvent>> + Send + Unpin;

    /// Send a request and get a complete response
    ///
    /// This method is used for non-streaming interactions where you want to
    /// receive the complete response at once.
    async fn request(&self, request: Request) -> Result<Response>;

    /// Send a request and get a stream of events
    ///
    /// This method is used for streaming interactions where the response is
    /// received incrementally as a series of events.
    async fn stream(&self, request: Request) -> Result<Self::Stream>;
}
