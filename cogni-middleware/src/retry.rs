//! Retry middleware for handling transient failures

use crate::{Service, Layer, BoxFuture};
use cogni_core::{Request, Response, Error};
use std::time::Duration;
use tracing::{warn, debug};

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
                            debug!(
                                attempt = attempt + 1,
                                "Request succeeded after retries"
                            );
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