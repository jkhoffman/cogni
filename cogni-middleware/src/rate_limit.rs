//! Rate limiting middleware to control request frequency

use crate::{BoxFuture, Layer, Service};
use cogni_core::{Error, Request, Response};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Rate limiting middleware layer
#[derive(Debug, Clone)]
pub struct RateLimitLayer {
    /// Rate limiter instance
    limiter: Arc<RwLock<TokenBucket>>,
}

/// Token bucket rate limiter
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum number of tokens
    capacity: usize,
    /// Current number of tokens
    tokens: f64,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
    /// Window for tracking requests
    window: Duration,
    /// Request timestamps within the window
    request_times: VecDeque<Instant>,
}

impl TokenBucket {
    /// Create a new token bucket
    pub fn new(capacity: usize, refill_rate: f64, window: Duration) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
            window,
            request_times: VecDeque::new(),
        }
    }

    /// Try to acquire a token
    pub async fn try_acquire(&mut self) -> bool {
        self.refill();
        self.clean_old_requests();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.request_times.push_back(Instant::now());
            true
        } else {
            false
        }
    }

    /// Wait until a token is available
    pub async fn acquire(&mut self) {
        loop {
            if self.try_acquire().await {
                return;
            }

            // Calculate wait time
            let tokens_needed = 1.0 - self.tokens;
            let wait_seconds = tokens_needed / self.refill_rate;
            let wait_duration = Duration::from_secs_f64(wait_seconds);

            debug!(
                wait_ms = wait_duration.as_millis(),
                tokens = self.tokens,
                "Rate limited, waiting for token"
            );

            tokio::time::sleep(wait_duration).await;
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = elapsed.as_secs_f64() * self.refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Remove requests outside the tracking window
    fn clean_old_requests(&mut self) {
        let cutoff = Instant::now() - self.window;
        while let Some(&front) = self.request_times.front() {
            if front < cutoff {
                self.request_times.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the number of requests in the current window
    pub fn requests_in_window(&self) -> usize {
        self.request_times.len()
    }
}

impl RateLimitLayer {
    /// Create a new rate limiter with requests per second
    pub fn new(requests_per_second: f64) -> Self {
        let limiter = TokenBucket::new(
            requests_per_second.ceil() as usize,
            requests_per_second,
            Duration::from_secs(1),
        );

        Self {
            limiter: Arc::new(RwLock::new(limiter)),
        }
    }

    /// Create with custom token bucket configuration
    pub fn with_token_bucket(capacity: usize, refill_rate: f64, window: Duration) -> Self {
        let limiter = TokenBucket::new(capacity, refill_rate, window);

        Self {
            limiter: Arc::new(RwLock::new(limiter)),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// Rate limiting service
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<RwLock<TokenBucket>>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, request: Request) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Acquire a token, waiting if necessary
            limiter.write().await.acquire().await;

            let requests_in_window = limiter.read().await.requests_in_window();
            debug!(
                requests_in_window = requests_in_window,
                "Rate limit token acquired"
            );

            // Execute the request
            inner.call(request).await
        })
    }
}

/// Re-export for convenience
pub use RateLimitLayer as RateLimitMiddleware;

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Message, Model, Request, Response, ResponseMetadata};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::{sleep, Instant as TokioInstant};

    /// Mock service for testing
    #[derive(Clone)]
    struct MockService {
        response: Response,
        call_count: Arc<AtomicUsize>,
        delay: Option<Duration>,
    }

    impl Service<Request> for MockService {
        type Response = Response;
        type Error = Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

        fn call(&mut self, _request: Request) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let response = self.response.clone();
            let delay = self.delay;

            Box::pin(async move {
                if let Some(d) = delay {
                    sleep(d).await;
                }
                Ok(response)
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
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10, 5.0, Duration::from_secs(2));
        assert_eq!(bucket.capacity, 10);
        assert_eq!(bucket.tokens, 10.0);
        assert_eq!(bucket.refill_rate, 5.0);
        assert_eq!(bucket.window, Duration::from_secs(2));
        assert!(bucket.request_times.is_empty());
    }

    #[tokio::test]
    async fn test_token_bucket_try_acquire() {
        let mut bucket = TokenBucket::new(3, 1.0, Duration::from_secs(1));

        // Should be able to acquire 3 tokens immediately
        assert!(bucket.try_acquire().await);
        assert!(bucket.try_acquire().await);
        assert!(bucket.try_acquire().await);

        // Should fail on the 4th attempt
        assert!(!bucket.try_acquire().await);

        // Wait for refill
        sleep(Duration::from_millis(1100)).await;

        // Should be able to acquire again
        assert!(bucket.try_acquire().await);
    }

    #[tokio::test]
    async fn test_token_bucket_acquire_with_wait() {
        let mut bucket = TokenBucket::new(1, 2.0, Duration::from_secs(1));

        // Use up the token
        assert!(bucket.try_acquire().await);

        // This should wait for approximately 0.5 seconds
        let start = TokioInstant::now();
        bucket.acquire().await;
        let elapsed = start.elapsed();

        // Should have waited at least 400ms (allowing some tolerance)
        assert!(elapsed >= Duration::from_millis(400));
        assert!(elapsed < Duration::from_millis(700));
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10, 5.0, Duration::from_secs(1));

        // Use some tokens
        bucket.tokens = 3.0;

        // Simulate time passing
        bucket.last_refill = Instant::now() - Duration::from_secs(1);
        bucket.refill();

        // Should have refilled 5 tokens
        assert!((bucket.tokens - 8.0).abs() < 0.1);

        // Use all tokens
        bucket.tokens = 0.0;

        // Simulate more time passing
        bucket.last_refill = Instant::now() - Duration::from_secs(3);
        bucket.refill();

        // Should be capped at capacity
        assert_eq!(bucket.tokens, 10.0);
    }

    #[test]
    fn test_token_bucket_clean_old_requests() {
        let mut bucket = TokenBucket::new(10, 5.0, Duration::from_secs(2));

        let now = Instant::now();
        // Add some old requests
        bucket.request_times.push_back(now - Duration::from_secs(3));
        bucket.request_times.push_back(now - Duration::from_secs(2));
        bucket
            .request_times
            .push_back(now - Duration::from_millis(1500));
        bucket
            .request_times
            .push_back(now - Duration::from_millis(500));

        bucket.clean_old_requests();

        // Should only keep requests within the 2-second window
        assert_eq!(bucket.request_times.len(), 2);
        assert_eq!(bucket.requests_in_window(), 2);
    }

    #[test]
    fn test_rate_limit_layer_creation() {
        let _layer = RateLimitLayer::new(10.0);
        // Just verify it creates without panicking

        let _layer = RateLimitLayer::with_token_bucket(20, 15.0, Duration::from_secs(3));
        // Just verify it creates without panicking
    }

    #[tokio::test]
    async fn test_rate_limit_service_basic() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
            delay: None,
        };

        let layer = RateLimitLayer::new(10.0);
        let mut rate_limited = layer.layer(mock_service.clone());

        // Should be able to make multiple requests quickly
        for _ in 0..5 {
            let request = create_test_request();
            let response = rate_limited.call(request).await.unwrap();
            assert_eq!(response.content, "test response");
        }

        assert_eq!(mock_service.call_count.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_rate_limit_service_throttling() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
            delay: None,
        };

        // Very low rate limit: 2 requests per second
        let layer = RateLimitLayer::new(2.0);
        let mut rate_limited = layer.layer(mock_service);

        let start = TokioInstant::now();

        // Make 4 requests
        for _ in 0..4 {
            let request = create_test_request();
            rate_limited.call(request).await.unwrap();
        }

        let elapsed = start.elapsed();

        // Should have taken at least 1 second (2 immediate, then wait ~0.5s each for the next 2)
        assert!(elapsed >= Duration::from_millis(900));
    }

    #[tokio::test]
    async fn test_rate_limit_service_concurrent() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
            delay: Some(Duration::from_millis(50)),
        };

        let layer = RateLimitLayer::new(5.0);
        let rate_limited = Arc::new(tokio::sync::Mutex::new(layer.layer(mock_service.clone())));

        let start = TokioInstant::now();
        let mut handles = vec![];

        // Spawn 10 concurrent requests
        for _ in 0..10 {
            let service = rate_limited.clone();
            let handle = tokio::spawn(async move {
                let request = create_test_request();
                let mut service = service.lock().await;
                service.call(request).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let elapsed = start.elapsed();

        // With 5 req/s and 10 requests, should take at least 1 second
        assert!(elapsed >= Duration::from_secs(1));
        assert_eq!(mock_service.call_count.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_rate_limit_service_error_propagation() {
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

        let layer = RateLimitLayer::new(10.0);
        let mut rate_limited = layer.layer(ErrorService);

        let request = create_test_request();
        let result = rate_limited.call(request).await;

        assert!(result.is_err());
        if let Err(Error::Network { message, .. }) = result {
            assert_eq!(message, "Connection failed");
        } else {
            panic!("Expected Network error");
        }
    }

    #[test]
    fn test_rate_limit_layer_clone() {
        let layer1 = RateLimitLayer::new(15.0);
        let layer2 = layer1.clone();

        // Both should share the same limiter
        assert!(Arc::ptr_eq(&layer1.limiter, &layer2.limiter));
    }

    #[tokio::test]
    async fn test_rate_limit_with_custom_bucket() {
        let mock_service = MockService {
            response: create_test_response(),
            call_count: Arc::new(AtomicUsize::new(0)),
            delay: None,
        };

        // Custom bucket: 3 tokens, 6 tokens/sec refill, 1 second window
        let layer = RateLimitLayer::with_token_bucket(3, 6.0, Duration::from_secs(1));
        let mut rate_limited = layer.layer(mock_service);

        let start = TokioInstant::now();

        // Make 6 requests
        for _ in 0..6 {
            let request = create_test_request();
            rate_limited.call(request).await.unwrap();
        }

        let elapsed = start.elapsed();

        // Should complete in about 0.5 seconds
        // (3 immediate, wait ~0.17s, get 1, wait ~0.17s, get 1, wait ~0.17s, get 1)
        assert!(elapsed >= Duration::from_millis(400));
        assert!(elapsed < Duration::from_millis(700));
    }
}
