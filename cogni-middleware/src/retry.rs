//! Retry middleware for handling transient failures

use crate::{BoxFuture, Layer, Service};
use cogni_core::{Error, Request, Response};
use std::time::Duration;
use tracing::{debug, warn};

/// Retry middleware layer
#[derive(Debug, Clone)]
pub struct RetryLayer {
    config: RetryConfig,
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryLayer {
    /// Create a new retry layer with default configuration
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }
}

impl Default for RetryLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for RetryLayer {
    type Service = RetryService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RetryService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Retry middleware service
#[derive(Clone)]
pub struct RetryService<S> {
    inner: S,
    config: RetryConfig,
}

impl<S> Service<Request> for RetryService<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, request: Request) -> Self::Future {
        let inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            let mut attempt = 0;
            let mut _last_error = None;

            loop {
                let mut service = inner.clone();
                match service.call(request.clone()).await {
                    Ok(response) => {
                        if attempt > 0 {
                            debug!(attempt = attempt + 1, "Request succeeded after retries");
                        }
                        return Ok(response);
                    }
                    Err(error) => {
                        attempt += 1;

                        if !Self::should_retry(&error) {
                            debug!(
                                error = %error,
                                "Error is not retryable"
                            );
                            return Err(error);
                        }

                        if attempt >= config.max_attempts {
                            warn!(
                                attempts = attempt,
                                error = %error,
                                "Max retry attempts reached"
                            );
                            return Err(error);
                        }

                        let backoff = Self::calculate_backoff(&config, attempt - 1);
                        warn!(
                            attempt = attempt,
                            backoff_ms = backoff.as_millis(),
                            error = %error,
                            "Request failed, retrying"
                        );

                        _last_error = Some(error);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        })
    }
}

impl<S> RetryService<S> {
    /// Check if an error should trigger a retry
    fn should_retry(error: &Error) -> bool {
        match error {
            Error::Network { .. } => true,
            Error::Provider { retry_after, .. } => retry_after.is_some(),
            Error::Timeout => true,
            Error::Serialization { .. } => false,
            Error::Validation(_) => false,
            Error::ToolExecution(_) => false,
            Error::Authentication(_) => false,
            Error::Configuration(_) => false,
            _ => false, // Unknown error types are not retryable by default
        }
    }

    /// Calculate backoff duration for a given attempt
    fn calculate_backoff(config: &RetryConfig, attempt: u32) -> Duration {
        let base = config.initial_backoff.as_millis() as f64;
        let backoff_ms = base * config.backoff_multiplier.powi(attempt as i32);
        let backoff = Duration::from_millis(backoff_ms as u64);

        std::cmp::min(backoff, config.max_backoff)
    }
}

/// Re-export for convenience
pub use RetryLayer as RetryMiddleware;

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Message, Model, Request, Response, ResponseMetadata};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::Instant;

    /// Mock service that can fail a specified number of times
    #[derive(Clone)]
    struct MockService {
        fail_count: Arc<AtomicUsize>,
        max_failures: usize,
        response: Response,
        error_type: ErrorType,
    }

    #[derive(Clone, Copy)]
    enum ErrorType {
        Network,
        Timeout,
        Validation,
        Authentication,
        ProviderWithRetry,
        ProviderWithoutRetry,
    }

    impl Service<Request> for MockService {
        type Response = Response;
        type Error = Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

        fn call(&mut self, _request: Request) -> Self::Future {
            let current = self.fail_count.fetch_add(1, Ordering::SeqCst);
            let response = self.response.clone();
            let error_type = self.error_type;
            let max_failures = self.max_failures;

            Box::pin(async move {
                if current < max_failures {
                    match error_type {
                        ErrorType::Network => Err(Error::Network {
                            message: "Connection failed".into(),
                            source: None,
                        }),
                        ErrorType::Timeout => Err(Error::Timeout),
                        ErrorType::Validation => Err(Error::Validation("Invalid input".into())),
                        ErrorType::Authentication => {
                            Err(Error::Authentication("Invalid token".into()))
                        }
                        ErrorType::ProviderWithRetry => Err(Error::Provider {
                            provider: "test".into(),
                            message: "Rate limited".into(),
                            retry_after: Some(Duration::from_millis(100)),
                            source: None,
                        }),
                        ErrorType::ProviderWithoutRetry => Err(Error::Provider {
                            provider: "test".into(),
                            message: "Invalid request".into(),
                            retry_after: None,
                            source: None,
                        }),
                    }
                } else {
                    Ok(response)
                }
            })
        }
    }

    fn create_test_request() -> Request {
        Request::builder()
            .model(Model("test-model".into()))
            .message(Message::user("test"))
            .build()
    }

    fn create_test_response() -> Response {
        Response {
            content: "test response".to_string(),
            tool_calls: Vec::new(),
            metadata: ResponseMetadata::default(),
        }
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_backoff, Duration::from_millis(100));
        assert_eq!(config.max_backoff, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_layer_creation() {
        let layer = RetryLayer::new();
        assert_eq!(layer.config.max_attempts, 3);

        let custom_config = RetryConfig {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 3.0,
        };
        let layer = RetryLayer::with_config(custom_config.clone());
        assert_eq!(layer.config.max_attempts, 5);
        assert_eq!(layer.config.initial_backoff, Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 0, // Don't fail
            response: create_test_response(),
            error_type: ErrorType::Network,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let response = retry_service.call(request).await.unwrap();

        assert_eq!(response.content, "test response");
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 2, // Fail twice, succeed on third
            response: create_test_response(),
            error_type: ErrorType::Network,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let response = retry_service.call(request).await.unwrap();

        assert_eq!(response.content, "test response");
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 5, // Always fail
            response: create_test_response(),
            error_type: ErrorType::Network,
        };

        let layer = RetryLayer::new(); // Default max_attempts = 3
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let result = retry_service.call(request).await;

        assert!(result.is_err());
        if let Err(Error::Network { message, .. }) = result {
            assert_eq!(message, "Connection failed");
        } else {
            panic!("Expected Network error");
        }
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 5, // Always fail
            response: create_test_response(),
            error_type: ErrorType::Validation,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let result = retry_service.call(request).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert_eq!(msg, "Invalid input");
        } else {
            panic!("Expected Validation error");
        }
        // Should not retry validation errors
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_timeout_error() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 2,
            response: create_test_response(),
            error_type: ErrorType::Timeout,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let response = retry_service.call(request).await.unwrap();

        assert_eq!(response.content, "test response");
        // Timeout errors should be retried
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_provider_error_with_retry_after() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 1,
            response: create_test_response(),
            error_type: ErrorType::ProviderWithRetry,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let response = retry_service.call(request).await.unwrap();

        assert_eq!(response.content, "test response");
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_provider_error_without_retry_after() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 5,
            response: create_test_response(),
            error_type: ErrorType::ProviderWithoutRetry,
        };

        let layer = RetryLayer::new();
        let mut retry_service = layer.layer(mock_service.clone());

        let request = create_test_request();
        let result = retry_service.call(request).await;

        assert!(result.is_err());
        // Should not retry provider errors without retry_after
        assert_eq!(mock_service.fail_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_calculate_backoff() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        };

        // First retry (attempt 0)
        let backoff = RetryService::<MockService>::calculate_backoff(&config, 0);
        assert_eq!(backoff, Duration::from_millis(100));

        // Second retry (attempt 1)
        let backoff = RetryService::<MockService>::calculate_backoff(&config, 1);
        assert_eq!(backoff, Duration::from_millis(200));

        // Third retry (attempt 2)
        let backoff = RetryService::<MockService>::calculate_backoff(&config, 2);
        assert_eq!(backoff, Duration::from_millis(400));

        // Large attempt should be capped at max_backoff
        let backoff = RetryService::<MockService>::calculate_backoff(&config, 10);
        assert_eq!(backoff, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_retry_backoff_timing() {
        let mock_service = MockService {
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_failures: 2, // Fail twice
            response: create_test_response(),
            error_type: ErrorType::Network,
        };

        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };

        let layer = RetryLayer::with_config(config);
        let mut retry_service = layer.layer(mock_service);

        let start = Instant::now();
        let request = create_test_request();
        let _response = retry_service.call(request).await.unwrap();
        let elapsed = start.elapsed();

        // Should have waited at least 50ms + 100ms = 150ms
        assert!(elapsed >= Duration::from_millis(150));
        // But not too much longer (accounting for execution time)
        assert!(elapsed < Duration::from_millis(300));
    }

    #[test]
    fn test_should_retry() {
        // Network errors should retry
        assert!(RetryService::<MockService>::should_retry(&Error::Network {
            message: "test".into(),
            source: None,
        }));

        // Timeout errors should retry
        assert!(RetryService::<MockService>::should_retry(&Error::Timeout));

        // Provider errors with retry_after should retry
        assert!(RetryService::<MockService>::should_retry(&Error::Provider {
            provider: "test".into(),
            message: "test".into(),
            retry_after: Some(Duration::from_secs(1)),
            source: None,
        }));

        // Provider errors without retry_after should not retry
        assert!(!RetryService::<MockService>::should_retry(&Error::Provider {
            provider: "test".into(),
            message: "test".into(),
            retry_after: None,
            source: None,
        }));

        // Validation errors should not retry
        assert!(!RetryService::<MockService>::should_retry(
            &Error::Validation("test".into())
        ));

        // Authentication errors should not retry
        assert!(!RetryService::<MockService>::should_retry(
            &Error::Authentication("test".into())
        ));

        // Configuration errors should not retry
        assert!(!RetryService::<MockService>::should_retry(
            &Error::Configuration("test".into())
        ));

        // Serialization errors should not retry
        assert!(!RetryService::<MockService>::should_retry(
            &Error::Serialization {
                message: "test".into(),
                source: None,
            }
        ));

        // Tool execution errors should not retry
        assert!(!RetryService::<MockService>::should_retry(
            &Error::ToolExecution("test".into())
        ));
    }

    #[test]
    fn test_retry_layer_clone() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 3.0,
        };
        let layer1 = RetryLayer::with_config(config.clone());
        let layer2 = layer1.clone();

        assert_eq!(layer1.config.max_attempts, layer2.config.max_attempts);
        assert_eq!(layer1.config.initial_backoff, layer2.config.initial_backoff);
        assert_eq!(layer1.config.max_backoff, layer2.config.max_backoff);
        assert_eq!(
            layer1.config.backoff_multiplier,
            layer2.config.backoff_multiplier
        );
    }
}
