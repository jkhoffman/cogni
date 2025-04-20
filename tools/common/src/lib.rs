#![allow(invalid_reference_casting)]
//! Common utilities for Cogni tools.
//!
//! This crate provides common utilities that can be used by tools:
//! - HTTP client with advanced features like rate limiting and retries
//! - Rate limiting utilities for tool operations
//! - Caching utilities to reduce redundant operations

pub mod cache;
pub mod http;
pub mod rate_limiter;

// Re-export main types for easy access
pub use cache::{CacheConfig, CacheError, CacheMetrics, ToolCache};
pub use http::{HttpClient, HttpClientConfig, HttpError};
pub use rate_limiter::{DomainStrategy, RateLimitError, RateLimiterConfig, ToolRateLimiter};
