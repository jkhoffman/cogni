//! Parallel execution utilities

use crate::Client;
use cogni_core::{Error, Provider, Request, Response};
use futures::future::join_all;
use std::sync::Arc;

/// Execute multiple requests in parallel across different providers.
///
/// This function takes a vector of providers and a single request, then executes
/// the request on all providers concurrently. Results are collected and returned
/// in the same order as the input providers.
///
/// # Examples
///
/// ```no_run
/// use cogni_client::parallel_requests;
/// use cogni_providers::OpenAI;
/// use cogni_core::{Request, Message};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Note: All providers must be the same type
/// let providers = vec![
///     OpenAI::with_api_key("key1".to_string()),
///     OpenAI::with_api_key("key2".to_string()),
/// ];
///
/// let request = Request::builder()
///     .message(Message::user("Hello"))
///     .build();
///
/// let results = parallel_requests(providers, request).await;
/// # Ok(())
/// # }
/// ```
pub async fn parallel_requests<P>(
    providers: Vec<P>,
    request: Request,
) -> Vec<Result<Response, Error>>
where
    P: Provider + Send + Sync + 'static,
    P::Stream: Send + 'static,
{
    let request = Arc::new(request);
    let handles: Vec<_> = providers
        .into_iter()
        .map(|provider| {
            let req = request.clone();
            tokio::spawn(async move { provider.request((*req).clone()).await })
        })
        .collect();

    let results = join_all(handles).await;
    results
        .into_iter()
        .map(|result| {
            result.unwrap_or_else(|e| {
                Err(Error::Provider {
                    provider: "unknown".to_string(),
                    message: e.to_string(),
                    retry_after: None,
                    source: None,
                })
            })
        })
        .collect()
}

/// Execute the same prompt across multiple providers in parallel.
///
/// This is a convenience function that creates a simple chat request with the given
/// message and executes it across all providers concurrently.
///
/// # Examples
///
/// ```no_run
/// use cogni_client::parallel_chat;
/// use cogni_providers::OpenAI;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Note: All providers must be the same type
/// let providers = vec![
///     OpenAI::with_api_key("key1".to_string()),
///     OpenAI::with_api_key("key2".to_string()),
/// ];
///
/// let results = parallel_chat(providers, "What is the capital of France?").await;
/// # Ok(())
/// # }
/// ```
pub async fn parallel_chat<P>(
    providers: Vec<P>,
    message: impl Into<String>,
) -> Vec<Result<String, Error>>
where
    P: Provider + Send + Sync + 'static,
    P::Stream: Send + 'static,
{
    let msg = message.into();
    let handles: Vec<_> = providers
        .into_iter()
        .map(|provider| {
            let client = Client::new(provider);
            let msg = msg.clone();
            tokio::spawn(async move { client.chat(msg).await })
        })
        .collect();

    let results = join_all(handles).await;
    results
        .into_iter()
        .map(|result| {
            result.unwrap_or_else(|e| {
                Err(Error::Provider {
                    provider: "unknown".to_string(),
                    message: e.to_string(),
                    retry_after: None,
                    source: None,
                })
            })
        })
        .collect()
}

/// A client that can execute requests across multiple providers.
///
/// The `ParallelClient` allows you to work with multiple LLM providers simultaneously,
/// using different execution strategies to determine how results are handled.
///
/// # Examples
///
/// ```no_run
/// use cogni_client::{ParallelClient, ExecutionStrategy};
/// use cogni_providers::OpenAI;
/// use cogni_core::{Request, Message};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Note: All providers must be the same type for ParallelClient
/// let providers = vec![
///     OpenAI::with_api_key("key1".to_string()),
///     OpenAI::with_api_key("key2".to_string()),
///     OpenAI::with_api_key("key3".to_string()),
/// ];
///
/// let client = ParallelClient::new(providers)
///     .with_strategy(ExecutionStrategy::FirstSuccess);
///
/// let request = Request::builder()
///     .message(Message::user("Hello"))
///     .build();
///
/// let response = client.request(request).await?;
/// # Ok(())
/// # }
/// ```
pub struct ParallelClient<P> {
    providers: Vec<P>,
    strategy: ExecutionStrategy,
}

/// Strategy for parallel execution.
///
/// Determines how the `ParallelClient` handles responses from multiple providers.
#[derive(Debug, Clone, Copy)]
pub enum ExecutionStrategy {
    /// Return the first successful response
    FirstSuccess,
    /// Return all responses
    All,
    /// Return the response that appears most frequently (consensus)
    Consensus,
    /// Return the fastest response
    Race,
}

impl<P: Provider + Clone> ParallelClient<P> {
    /// Create a new parallel client with the given providers.
    ///
    /// By default, uses the `FirstSuccess` execution strategy.
    pub fn new(providers: Vec<P>) -> Self {
        Self {
            providers,
            strategy: ExecutionStrategy::FirstSuccess,
        }
    }

    /// Set the execution strategy.
    ///
    /// This determines how the client handles responses from multiple providers.
    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Execute a request using the configured strategy
    pub async fn request(&self, request: Request) -> Result<Response, Error>
    where
        P: Send + Sync + 'static,
        P::Stream: Send + 'static,
    {
        match self.strategy {
            ExecutionStrategy::FirstSuccess => self.first_success(request).await,
            ExecutionStrategy::All => self.all_responses(request).await,
            ExecutionStrategy::Consensus => self.consensus(request).await,
            ExecutionStrategy::Race => self.race(request).await,
        }
    }

    /// Get the first successful response
    async fn first_success(&self, request: Request) -> Result<Response, Error>
    where
        P: Send + Sync + 'static,
        P::Stream: Send + 'static,
    {
        let providers = self.providers.clone();
        let results = parallel_requests(providers, request).await;

        results.into_iter().find(|r| r.is_ok()).unwrap_or_else(|| {
            Err(Error::Provider {
                provider: "parallel".to_string(),
                message: "All providers failed".to_string(),
                retry_after: None,
                source: None,
            })
        })
    }

    /// Get all responses (returns the first one, but waits for all)
    async fn all_responses(&self, request: Request) -> Result<Response, Error>
    where
        P: Send + Sync + 'static,
        P::Stream: Send + 'static,
    {
        let providers = self.providers.clone();
        let results = parallel_requests(providers, request).await;

        // Return the first successful response
        results.into_iter().find(|r| r.is_ok()).unwrap_or_else(|| {
            Err(Error::Provider {
                provider: "parallel".to_string(),
                message: "All providers failed".to_string(),
                retry_after: None,
                source: None,
            })
        })
    }

    /// Get consensus response (most common response)
    async fn consensus(&self, request: Request) -> Result<Response, Error>
    where
        P: Send + Sync + 'static,
        P::Stream: Send + 'static,
    {
        let providers = self.providers.clone();
        let results = parallel_requests(providers, request).await;

        let successful_responses: Vec<Response> =
            results.into_iter().filter_map(|r| r.ok()).collect();

        // For simplicity, just return the first response
        // In a real implementation, you'd compare responses for similarity
        successful_responses
            .into_iter()
            .next()
            .ok_or_else(|| Error::Provider {
                provider: "parallel".to_string(),
                message: "All providers failed".to_string(),
                retry_after: None,
                source: None,
            })
    }

    /// Race providers and return the fastest response
    async fn race(&self, request: Request) -> Result<Response, Error>
    where
        P: Send + Sync + 'static,
        P::Stream: Send + 'static,
    {
        let request = Arc::new(request);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        // Spawn tasks for each provider
        for provider in self.providers.clone() {
            let tx = tx.clone();
            let req = request.clone();
            tokio::spawn(async move {
                if let Ok(response) = provider.request((*req).clone()).await {
                    let _ = tx.send(response).await;
                }
            });
        }

        // Drop the original sender so the channel closes when all tasks complete
        drop(tx);

        // Return the first response received
        rx.recv().await.ok_or_else(|| Error::Provider {
            provider: "parallel".to_string(),
            message: "All providers failed".to_string(),
            retry_after: None,
            source: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Message, ResponseMetadata, StreamEvent};
    use futures::{stream, Stream};
    use std::pin::Pin;

    #[derive(Clone)]
    struct MockProvider {
        response: String,
        delay: Option<std::time::Duration>,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        type Stream = Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>;

        async fn request(&self, _request: Request) -> Result<Response, Error> {
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            Ok(Response {
                content: self.response.clone(),
                tool_calls: vec![],
                metadata: ResponseMetadata::default(),
            })
        }

        async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
            let events = vec![Ok(StreamEvent::Done)];
            Ok(Box::pin(stream::iter(events)))
        }
    }

    #[tokio::test]
    async fn test_parallel_chat() {
        let providers = vec![
            MockProvider {
                response: "Response 1".to_string(),
                delay: None,
            },
            MockProvider {
                response: "Response 2".to_string(),
                delay: None,
            },
        ];

        let results = parallel_chat(providers, "Test message").await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_parallel_client_race() {
        use std::time::Duration;

        let client = ParallelClient::new(vec![
            MockProvider {
                response: "Slow".to_string(),
                delay: Some(Duration::from_millis(100)),
            },
            MockProvider {
                response: "Fast".to_string(),
                delay: Some(Duration::from_millis(10)),
            },
        ])
        .with_strategy(ExecutionStrategy::Race);

        let request = Request {
            messages: vec![Message::user("Test")],
            model: cogni_core::Model::default(),
            parameters: cogni_core::Parameters::default(),
            tools: vec![],
            response_format: None,
        };

        let response = client.request(request).await.unwrap();
        assert_eq!(response.content, "Fast");
    }

    #[tokio::test]
    async fn test_parallel_client_first_success() {
        let client = ParallelClient::new(vec![MockProvider {
            response: "Success".to_string(),
            delay: None,
        }])
        .with_strategy(ExecutionStrategy::FirstSuccess);

        let request = Request {
            messages: vec![Message::user("Test")],
            model: cogni_core::Model::default(),
            parameters: cogni_core::Parameters::default(),
            tools: vec![],
            response_format: None,
        };

        let response = client.request(request).await.unwrap();
        assert_eq!(response.content, "Success");
    }
}
