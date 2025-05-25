//! Rate limiting middleware to control request frequency

use crate::{Service, Layer, BoxFuture};
use cogni_core::{Request, Response, Error};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
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
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<RwLock<TokenBucket>>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response, Error = Error>,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;
    
    fn call(&mut self, request: Request) -> Self::Future {
        let limiter = self.limiter.clone();
        let fut = self.inner.call(request);
        
        Box::pin(async move {
            // Acquire a token, waiting if necessary
            limiter.write().await.acquire().await;
            
            let requests_in_window = limiter.read().await.requests_in_window();
            debug!(
                requests_in_window = requests_in_window,
                "Rate limit token acquired"
            );
            
            // Execute the request
            fut.await
        })
    }
}

/// Re-export for convenience
pub use RateLimitLayer as RateLimitMiddleware;