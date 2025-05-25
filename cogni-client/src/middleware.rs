//! Middleware integration for the high-level client

use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response, StreamEvent};
use cogni_middleware::Service;
use futures::Stream;
use std::pin::Pin;

/// A provider that wraps a middleware service
///
/// This allows using middleware-wrapped services with the high-level Client API
/// 
/// Note: The middleware services must implement Clone to work with the Provider trait.
/// This is a limitation of the current design where Provider methods take &self.
pub struct MiddlewareProvider<S> {
    service: S,
}

impl<S> MiddlewareProvider<S> {
    /// Create a new middleware provider from a service
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S> Provider for MiddlewareProvider<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + Sync + 'static,
    S::Future: Send,
{
    type Stream = Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>;

    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut service = self.service.clone();
        service.call(request).await
    }

    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        // For now, streaming through middleware is not supported
        // We convert the response to a single-event stream
        let response = self.request(request).await?;
        
        // Convert response to stream events
        let events = vec![
            Ok(StreamEvent::Content(cogni_core::ContentDelta {
                text: response.content,
            })),
            Ok(StreamEvent::Done),
        ];
        
        Ok(Box::pin(futures::stream::iter(events)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{ContentDelta, ResponseMetadata, StreamEvent};
    use futures::stream;

    // Mock provider for testing
    #[derive(Clone)]
    struct MockProvider;

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        type Stream = Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>;

        async fn request(&self, _request: Request) -> Result<Response, Error> {
            Ok(Response {
                content: "Mock response".to_string(),
                tool_calls: vec![],
                metadata: ResponseMetadata::default(),
            })
        }

        async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
            let events = vec![
                Ok(StreamEvent::Content(ContentDelta {
                    text: "Mock ".to_string(),
                })),
                Ok(StreamEvent::Content(ContentDelta {
                    text: "stream".to_string(),
                })),
                Ok(StreamEvent::Done),
            ];
            Ok(Box::pin(stream::iter(events)))
        }
    }

    #[tokio::test]
    async fn test_middleware_provider_basic() {
        use cogni_middleware::ProviderService;
        
        let provider = MockProvider;
        let service = ProviderService::new(provider);
        let middleware_provider = MiddlewareProvider::new(service);
        
        let response = middleware_provider
            .request(Request {
                messages: vec![cogni_core::Message::user("Test")],
                model: cogni_core::Model::default(),
                parameters: cogni_core::Parameters::default(),
                tools: vec![],
            })
            .await
            .unwrap();
            
        assert_eq!(response.content, "Mock response");
    }
}