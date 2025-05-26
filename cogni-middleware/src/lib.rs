//! Middleware system for cross-cutting concerns
//!
//! This module provides a Tower-inspired middleware system that works with
//! the constraints of Rust's type system and async traits.

#![warn(missing_docs)]

use cogni_core::{Error, Provider, Request, Response};
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;

pub mod cache;
pub mod logging;
pub mod rate_limit;
pub mod retry;
pub mod state;

// Re-export middleware implementations
pub use cache::{CacheLayer, CacheService};
pub use logging::{LogLevel, LoggingLayer, LoggingService};
pub use rate_limit::{RateLimitLayer, RateLimitService};
pub use retry::{RetryConfig, RetryLayer, RetryService};
pub use state::{StateConfig, StateLayer, StateService};

/// Type alias for boxed futures
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Type alias for boxed streams
pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

/// A service that can process LLM requests
///
/// This is inspired by Tower's Service trait but adapted for our use case
pub trait Service<R> {
    /// The response type
    type Response;
    /// The error type
    type Error;
    /// The future returned by the service
    type Future: Future<Output = Result<Self::Response, Self::Error>> + Send;

    /// Process a request
    fn call(&mut self, request: R) -> Self::Future;
}

/// Layer trait for composing middleware
pub trait Layer<S> {
    /// The wrapped service
    type Service;

    /// Wrap a service with this layer
    fn layer(&self, service: S) -> Self::Service;
}

/// A boxed service for type erasure
pub struct BoxService<Req, Res, Err> {
    inner: Box<
        dyn Service<Req, Response = Res, Error = Err, Future = BoxFuture<Result<Res, Err>>> + Send,
    >,
}

impl<Req, Res, Err> BoxService<Req, Res, Err> {
    /// Create a new boxed service
    pub fn new<S>(service: S) -> Self
    where
        S: Service<Req, Response = Res, Error = Err> + Send + 'static,
        S::Future: Send + 'static,
    {
        Self {
            inner: Box::new(ServiceWrapper { service }),
        }
    }
}

/// Wrapper to make any service work with boxing
struct ServiceWrapper<S> {
    service: S,
}

impl<S, Req> Service<Req> for ServiceWrapper<S>
where
    S: Service<Req> + Send,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, request: Req) -> Self::Future {
        Box::pin(self.service.call(request))
    }
}

impl<Req, Res, Err> Service<Req> for BoxService<Req, Res, Err>
where
    Req: 'static,
{
    type Response = Res;
    type Error = Err;
    type Future = BoxFuture<Result<Res, Err>>;

    fn call(&mut self, request: Req) -> Self::Future {
        self.inner.call(request)
    }
}

/// Adapter to convert a Provider into a Service
#[derive(Clone)]
pub struct ProviderService<P> {
    provider: P,
}

impl<P> ProviderService<P> {
    /// Create a new provider service
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> Service<Request> for ProviderService<P>
where
    P: Provider + Clone + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Response, Error>>;

    fn call(&mut self, request: Request) -> Self::Future {
        let provider = self.provider.clone();
        Box::pin(async move { provider.request(request).await })
    }
}

/// Extension trait to easily apply middleware to providers
pub trait ProviderExt: Provider + Sized {
    /// Wrap this provider with a service adapter
    fn into_service(self) -> ProviderService<Self> {
        ProviderService::new(self)
    }

    /// Apply a layer to this provider
    fn layer<L>(self, layer: L) -> L::Service
    where
        L: Layer<ProviderService<Self>>,
    {
        layer.layer(self.into_service())
    }
}

impl<P: Provider> ProviderExt for P {}

/// Stack multiple layers together
pub struct Stack<Inner, Outer> {
    inner: Inner,
    outer: Outer,
}

impl<Inner, Outer> Stack<Inner, Outer> {
    /// Create a new stack
    pub fn new(inner: Inner, outer: Outer) -> Self {
        Self { inner, outer }
    }
}

impl<S, Inner, Outer> Layer<S> for Stack<Inner, Outer>
where
    Inner: Layer<S>,
    Outer: Layer<Inner::Service>,
{
    type Service = Outer::Service;

    fn layer(&self, service: S) -> Self::Service {
        let inner = self.inner.layer(service);
        self.outer.layer(inner)
    }
}

/// Identity layer that does nothing
pub struct Identity;

impl<S> Layer<S> for Identity {
    type Service = S;

    fn layer(&self, service: S) -> Self::Service {
        service
    }
}

/// Builder for composing layers
pub struct ServiceBuilder<L> {
    layer: L,
}

impl ServiceBuilder<Identity> {
    /// Create a new service builder
    pub fn new() -> Self {
        Self { layer: Identity }
    }
}

impl<L> ServiceBuilder<L> {
    /// Add a layer to the stack
    pub fn layer<T>(self, layer: T) -> ServiceBuilder<Stack<L, T>> {
        ServiceBuilder {
            layer: Stack::new(self.layer, layer),
        }
    }

    /// Build the service with the given inner service
    pub fn service<S>(self, service: S) -> L::Service
    where
        L: Layer<S>,
    {
        self.layer.layer(service)
    }
}

impl Default for ServiceBuilder<Identity> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use cogni_core::ResponseMetadata;

    /// A simple echo service for testing
    #[derive(Clone)]
    pub struct EchoService;

    impl Service<Request> for EchoService {
        type Response = Response;
        type Error = Error;
        type Future = BoxFuture<Result<Response, Error>>;

        fn call(&mut self, request: Request) -> Self::Future {
            Box::pin(async move {
                let content = request.messages
                    .last()
                    .and_then(|m| m.content.as_text())
                    .unwrap_or("No message");
                
                Ok(Response {
                    content: format!("Echo: {}", content),
                    tool_calls: vec![],
                    metadata: ResponseMetadata::default(),
                })
            })
        }
    }
}
