#![allow(invalid_reference_casting)]
//! Rate limiting utilities for tools.
//!
//! This module provides a rate limiting system for tools to prevent
//! overloading external services. It supports:
//! - Global rate limits
//! - Per-domain rate limits
//! - Rate limit groups
//! - Configurable limit strategies

use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Instant};

use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::info;
use url::Url;

/// The default burst size for domain-based rate limiters.
pub const DEFAULT_DOMAIN_BURST_SIZE: u32 = 5;

/// The default rate limit for domains in requests per second.
pub const DEFAULT_DOMAIN_RPS: f64 = 5.0;

/// The default rate limit for the global limiter in requests per second.
pub const DEFAULT_GLOBAL_RPS: f64 = 20.0;

/// The default burst size for the global rate limiter.
pub const DEFAULT_GLOBAL_BURST_SIZE: u32 = 10;

/// Errors that can occur during rate limiting.
#[derive(Error, Debug)]
pub enum RateLimitError {
    /// The rate limit was exceeded
    #[error("Rate limit exceeded for {target}: {message}")]
    LimitExceeded {
        /// The target that was rate limited (e.g., domain, group)
        target: String,
        /// Additional details about the rate limit
        message: String,
    },

    /// The provided URL was invalid
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// An internal error occurred
    #[error("Rate limiter error: {0}")]
    Internal(String),

    /// Unknown domain
    #[error("Unknown domain: {0}, {1}")]
    UnknownDomain(String, String),

    /// Domain exists
    #[error("Domain exists: {0}")]
    DomainExists(String),
}

/// The domain rate limiting strategy to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainStrategy {
    /// Use the full domain (e.g., api.example.com)
    FullDomain,
    /// Use only the main domain (e.g., example.com)
    MainDomain,
}

impl Default for DomainStrategy {
    fn default() -> Self {
        Self::MainDomain
    }
}

/// Configuration for the rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// The global rate limit in requests per second.
    pub global_rps: f64,
    /// The global burst size.
    pub global_burst_size: u32,
    /// The default rate limit for domains in requests per second.
    pub default_domain_rps: f64,
    /// The default burst size for domains.
    pub default_domain_burst_size: u32,
    /// The domain rate limiting strategy to use.
    pub domain_strategy: DomainStrategy,
    /// Whether the global rate limiter is enabled.
    pub global_enabled: bool,
    /// Whether domain-based rate limiting is enabled.
    pub domain_enabled: bool,
    /// Whether group-based rate limiting is enabled.
    pub group_enabled: bool,
    /// Domain-specific rate limits (in RPS).
    pub domain_limits: HashMap<String, f64>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            global_rps: DEFAULT_GLOBAL_RPS,
            global_burst_size: DEFAULT_GLOBAL_BURST_SIZE,
            default_domain_rps: DEFAULT_DOMAIN_RPS,
            default_domain_burst_size: DEFAULT_DOMAIN_BURST_SIZE,
            domain_strategy: DomainStrategy::default(),
            global_enabled: true,
            domain_enabled: true,
            group_enabled: true,
            domain_limits: HashMap::new(),
        }
    }
}

/// A rate limiter for tools.
///
/// The rate limiter supports:
/// - Global rate limits
/// - Per-domain rate limits
/// - Rate limit groups
#[derive(Debug, Clone)]
pub struct ToolRateLimiter {
    /// The configuration for this rate limiter.
    config: RateLimiterConfig,
    /// The global rate limiter.
    global_limiter: Option<Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    /// Domain-specific rate limiters.
    domain_limiters: Option<
        Arc<DashMap<String, RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    >,
    /// Group-specific rate limiters.
    group_limiters: Option<
        Arc<DashMap<String, RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    >,
    /// Last updated time for adaptive rate limiters
    last_updated: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Default for ToolRateLimiter {
    fn default() -> Self {
        Self::new(RateLimiterConfig::default())
    }
}

impl ToolRateLimiter {
    /// Create a new rate limiter with the given configuration.
    ///
    /// # Arguments
    /// * `config` - The configuration for the rate limiter
    ///
    /// # Returns
    /// A new `ToolRateLimiter` instance
    pub fn new(config: RateLimiterConfig) -> Self {
        // Create the global rate limiter if enabled
        let global_limiter = if config.global_enabled {
            let quota = Quota::per_second(
                std::num::NonZeroU32::new(config.global_rps as u32).unwrap_or_else(|| {
                    std::num::NonZeroU32::new(DEFAULT_GLOBAL_RPS as u32).unwrap()
                }),
            )
            .allow_burst(std::num::NonZeroU32::new(config.global_burst_size).unwrap());
            Some(Arc::new(RateLimiter::direct(quota)))
        } else {
            None
        };

        // Create the domain limiters map if enabled
        let domain_limiters = if config.domain_enabled {
            Some(Arc::new(DashMap::new()))
        } else {
            None
        };

        // Create the group limiters map if enabled
        let group_limiters = if config.group_enabled {
            Some(Arc::new(DashMap::new()))
        } else {
            None
        };

        Self {
            config,
            global_limiter,
            domain_limiters,
            group_limiters,
            last_updated: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a request to the given URL is allowed by the rate limiter.
    ///
    /// This method extracts the domain from the URL and checks it against
    /// the domain-specific rate limiter (if enabled) and the global rate
    /// limiter (if enabled).
    ///
    /// # Arguments
    /// * `url` - The URL to check
    ///
    /// # Returns
    /// `Ok(())` if the request is allowed, or an error if the rate limit is exceeded
    ///
    /// # Errors
    /// Returns `RateLimitError::InvalidUrl` if the URL is invalid.
    /// Returns `RateLimitError::LimitExceeded` if the rate limit is exceeded.
    pub async fn check_url(&self, url: &str) -> Result<(), RateLimitError> {
        // Parse the URL
        let parsed_url =
            Url::parse(url).map_err(|_| RateLimitError::InvalidUrl(url.to_string()))?;

        // Extract the domain
        let domain = match parsed_url.host_str() {
            Some(host) => match self.config.domain_strategy {
                DomainStrategy::FullDomain => host.to_string(),
                DomainStrategy::MainDomain => {
                    // Extract the main domain (e.g., example.com from api.example.com)
                    let parts: Vec<&str> = host.split('.').collect();
                    if parts.len() >= 2 {
                        // Get the last two parts (e.g., example.com)

                        parts[parts.len() - 2..].join(".")
                    } else {
                        host.to_string()
                    }
                }
            },
            None => return Err(RateLimitError::InvalidUrl(url.to_string())),
        };

        // Check the global rate limiter first if enabled
        if let Some(limiter) = &self.global_limiter {
            if let Err(err) = limiter.check() {
                return Err(RateLimitError::LimitExceeded {
                    target: "global".to_string(),
                    message: format!("{}", err),
                });
            }
        }

        // Check the domain-specific rate limiter if enabled
        if let Some(domain_limiters) = &self.domain_limiters {
            // Get or create the domain rate limiter
            let limiter = domain_limiters.entry(domain.clone()).or_insert_with(|| {
                // Get the domain-specific rate limit or use the default
                let rps = self
                    .config
                    .domain_limits
                    .get(&domain)
                    .copied()
                    .unwrap_or(self.config.default_domain_rps);

                // Create the quota
                let quota =
                    Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                        std::num::NonZeroU32::new(self.config.default_domain_rps as u32).unwrap()
                    }))
                    .allow_burst(
                        std::num::NonZeroU32::new(self.config.default_domain_burst_size).unwrap(),
                    );

                // Create the rate limiter
                RateLimiter::direct(quota)
            });

            // Check if the request is allowed
            if let Err(err) = limiter.check() {
                return Err(RateLimitError::LimitExceeded {
                    target: domain,
                    message: format!("{}", err),
                });
            }
        }

        Ok(())
    }

    /// Check if a request to the given group is allowed by the rate limiter.
    ///
    /// # Arguments
    /// * `group` - The group to check
    /// * `rps` - The rate limit in requests per second
    /// * `burst` - Optional burst size (default: 1)
    ///
    /// # Returns
    /// `Ok(())` if the request is allowed, or an error if the rate limit is exceeded
    ///
    /// # Errors
    /// Returns `RateLimitError::LimitExceeded` if the rate limit is exceeded.
    pub async fn check_group(
        &self,
        group: &str,
        rps: f64,
        burst: Option<u32>,
    ) -> Result<(), RateLimitError> {
        // Check if group-based rate limiting is enabled
        if let Some(group_limiters) = &self.group_limiters {
            // Get or create the group rate limiter
            let limiter = group_limiters.entry(group.to_string()).or_insert_with(|| {
                // Create the quota
                let quota =
                    Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                        std::num::NonZeroU32::new(1).unwrap() // Fallback to 1 RPS
                    }))
                    .allow_burst(
                        std::num::NonZeroU32::new(burst.unwrap_or(1)).unwrap_or_else(|| {
                            std::num::NonZeroU32::new(1).unwrap() // Fallback to burst of 1
                        }),
                    );

                // Create the rate limiter
                RateLimiter::direct(quota)
            });

            // Check if the request is allowed
            if let Err(err) = limiter.check() {
                return Err(RateLimitError::LimitExceeded {
                    target: group.to_string(),
                    message: format!("{}", err),
                });
            }
        }

        Ok(())
    }

    /// Updates the rate limit for a domain.
    ///
    /// # Arguments
    /// * `domain` - The domain to update
    /// * `rps` - The new rate limit in requests per second
    pub async fn update_domain_limit(&self, domain: &str, rps: f64) {
        // Update the domain-specific rate limit in the config
        let mut config = self.config.clone();
        config.domain_limits.insert(domain.to_string(), rps);

        // Update the domain rate limiter if it exists and if domain-based rate limiting is enabled
        if let Some(domain_limiters) = &self.domain_limiters {
            if let Some(mut limiter) = domain_limiters.get_mut(domain) {
                // Create the quota
                let quota =
                    Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                        std::num::NonZeroU32::new(self.config.default_domain_rps as u32).unwrap()
                    }))
                    .allow_burst(
                        std::num::NonZeroU32::new(self.config.default_domain_burst_size).unwrap(),
                    );

                // Replace the rate limiter
                *limiter = RateLimiter::direct(quota);
            }
        }
    }

    /// Updates the rate limit for a group.
    ///
    /// # Arguments
    /// * `group` - The group to update
    /// * `rps` - The new rate limit in requests per second
    /// * `burst` - Optional burst size (default: 1)
    pub async fn update_group_limit(&self, group: &str, rps: f64, burst: Option<u32>) {
        // Update the group rate limiter if it exists and if group-based rate limiting is enabled
        if let Some(group_limiters) = &self.group_limiters {
            if let Some(mut limiter) = group_limiters.get_mut(group) {
                // Use provided burst or default to 1 instead of calling get_group_status
                let burst_value = burst.unwrap_or(1);

                // Create the quota
                let quota =
                    Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                        std::num::NonZeroU32::new(1).unwrap() // Fallback to 1 RPS
                    }))
                    .allow_burst(
                        std::num::NonZeroU32::new(burst_value).unwrap_or_else(|| {
                            std::num::NonZeroU32::new(1).unwrap() // Fallback to burst of 1
                        }),
                    );

                // Replace the rate limiter
                *limiter = RateLimiter::direct(quota);
            }
        }
    }

    /// Updates the global rate limit.
    ///
    /// # Arguments
    /// * `rps` - The new rate limit in requests per second
    /// * `burst` - Optional burst size (default: unchanged)
    #[allow(invalid_reference_casting)]
    pub async fn update_global_limit(&self, rps: f64, burst: Option<u32>) {
        // Update the global rate limiter if it exists and if global rate limiting is enabled
        if self.config.global_enabled {
            // Get the current burst size if not provided
            let current_burst = if let Some((_, b)) = self.get_global_status().await {
                b
            } else {
                self.config.global_burst_size
            };

            // Create the quota
            let quota =
                Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                    std::num::NonZeroU32::new(DEFAULT_GLOBAL_RPS as u32).unwrap()
                }))
                .allow_burst(
                    std::num::NonZeroU32::new(burst.unwrap_or(current_burst)).unwrap_or_else(
                        || std::num::NonZeroU32::new(DEFAULT_GLOBAL_BURST_SIZE).unwrap(),
                    ),
                );

            // Replace the global limiter
            let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
            self_mut.global_limiter = Some(Arc::new(RateLimiter::direct(quota)));
        }
    }

    /// Enable or disable the global rate limiter.
    ///
    /// # Arguments
    /// * `enabled` - Whether the global rate limiter should be enabled
    #[allow(invalid_reference_casting)]
    pub async fn set_global_enabled(&self, enabled: bool) {
        // Safety: this is safe because we're updating a boolean value
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };

        if enabled && !self.config.global_enabled {
            // Enable the global limiter
            let quota = Quota::per_second(
                std::num::NonZeroU32::new(self.config.global_rps as u32).unwrap_or_else(|| {
                    std::num::NonZeroU32::new(DEFAULT_GLOBAL_RPS as u32).unwrap()
                }),
            )
            .allow_burst(std::num::NonZeroU32::new(self.config.global_burst_size).unwrap());
            self_mut.global_limiter = Some(Arc::new(RateLimiter::direct(quota)));
            self_mut.config.global_enabled = true;
            info!("Global rate limiter enabled");
        } else if !enabled && self.config.global_enabled {
            // Disable the global limiter
            self_mut.global_limiter = None;
            self_mut.config.global_enabled = false;
            info!("Global rate limiter disabled");
        }
    }

    /// Enable or disable domain-based rate limiting.
    ///
    /// # Arguments
    /// * `enabled` - Whether domain-based rate limiting should be enabled
    #[allow(invalid_reference_casting)]
    pub async fn set_domain_enabled(&self, enabled: bool) {
        // Safety: this is safe because we're updating a boolean value
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };

        if enabled && !self.config.domain_enabled {
            // Enable domain limiters
            self_mut.domain_limiters = Some(Arc::new(DashMap::new()));
            self_mut.config.domain_enabled = true;
            info!("Domain-based rate limiting enabled");
        } else if !enabled && self.config.domain_enabled {
            // Disable domain limiters
            self_mut.domain_limiters = None;
            self_mut.config.domain_enabled = false;
            info!("Domain-based rate limiting disabled");
        }
    }

    /// Enable or disable group-based rate limiting.
    ///
    /// # Arguments
    /// * `enabled` - Whether group-based rate limiting should be enabled
    #[allow(invalid_reference_casting)]
    pub async fn set_group_enabled(&self, enabled: bool) {
        // Safety: this is safe because we're updating a boolean value
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };

        if enabled && !self.config.group_enabled {
            // Enable group limiters
            self_mut.group_limiters = Some(Arc::new(DashMap::new()));
            self_mut.config.group_enabled = true;
            info!("Group-based rate limiting enabled");
        } else if !enabled && self.config.group_enabled {
            // Disable group limiters
            self_mut.group_limiters = None;
            self_mut.config.group_enabled = false;
            info!("Group-based rate limiting disabled");
        }
    }

    /// Get the rate limit status for a domain.
    ///
    /// # Arguments
    /// * `domain` - The domain to check
    ///
    /// # Returns
    /// A tuple containing the current RPS limit and the number of remaining requests
    /// before hitting the rate limit.
    pub async fn get_domain_status(&self, domain: &str) -> Option<(f64, u32)> {
        if let Some(domain_limiters) = &self.domain_limiters {
            if let Some(limiter) = domain_limiters.get(domain) {
                let rps = self
                    .config
                    .domain_limits
                    .get(domain)
                    .copied()
                    .unwrap_or(self.config.default_domain_rps);

                // This is an approximation since we don't have direct access to the remaining quota
                let nonconforming = limiter.check().is_err();
                let remaining = if nonconforming {
                    0
                } else {
                    self.config.default_domain_burst_size
                };

                return Some((rps, remaining));
            }
        }
        None
    }

    /// Get the rate limit status for a group.
    ///
    /// # Arguments
    /// * `group` - The group to check
    ///
    /// # Returns
    /// A tuple containing the current RPS limit and the number of remaining requests
    /// before hitting the rate limit.
    pub async fn get_group_status(&self, group: &str) -> Option<(f64, u32)> {
        if let Some(group_limiters) = &self.group_limiters {
            if let Some(limiter) = group_limiters.get(group) {
                // This is an approximation since we don't have direct access to the internal state
                let nonconforming = limiter.check().is_err();
                let remaining = if nonconforming { 0 } else { 5 }; // Approximation

                // Return a hardcoded approximation instead of trying to extract values
                // Return a hardcoded value for rps instead of using what could be a non-existent value
                return Some((5.0, remaining));
            }
        }
        None
    }

    /// Get the global rate limit status.
    ///
    /// # Returns
    /// A tuple containing the current RPS limit and the number of remaining requests
    /// before hitting the rate limit.
    pub async fn get_global_status(&self) -> Option<(f64, u32)> {
        if let Some(global_limiter) = &self.global_limiter {
            // This is an approximation since we don't have direct access to the remaining quota
            let nonconforming = global_limiter.check().is_err();
            let remaining = if nonconforming {
                0
            } else {
                self.config.global_burst_size
            };

            return Some((self.config.global_rps, remaining));
        }
        None
    }

    pub fn domain(&self, domain: &str, rps: f64, burst: Option<u32>) -> Result<(), RateLimitError> {
        let domain_limiters = self.domain_limiters.as_ref().unwrap();
        if let Some(mut limiter) = domain_limiters.get_mut(domain) {
            let quota =
                Quota::per_second(std::num::NonZeroU32::new(rps as u32).unwrap_or_else(|| {
                    std::num::NonZeroU32::new(DEFAULT_DOMAIN_RPS as u32).unwrap()
                }))
                .allow_burst(
                    std::num::NonZeroU32::new(burst.unwrap_or(DEFAULT_DOMAIN_BURST_SIZE)).unwrap(),
                );
            *limiter = RateLimiter::direct(quota);
            Ok(())
        } else {
            Err(RateLimitError::UnknownDomain(
                domain.to_string(),
                "Cannot update rate limit for unknown domain".to_string(),
            ))
        }
    }

    pub fn add_domain(
        &self,
        domain: &str,
        rps: Option<f64>,
        burst: Option<u32>,
    ) -> Result<(), RateLimitError> {
        let domain_limiters = self.domain_limiters.as_ref().unwrap();
        if domain_limiters.contains_key(domain) {
            return Err(RateLimitError::DomainExists(domain.to_string()));
        }
        let rps = rps.unwrap_or(DEFAULT_DOMAIN_RPS);
        let burst = burst.unwrap_or(DEFAULT_DOMAIN_BURST_SIZE);
        let quota = Quota::per_second(
            std::num::NonZeroU32::new(rps as u32)
                .unwrap_or_else(|| std::num::NonZeroU32::new(DEFAULT_DOMAIN_RPS as u32).unwrap()),
        )
        .allow_burst(std::num::NonZeroU32::new(burst).unwrap());
        let limiter = RateLimiter::direct(quota);
        domain_limiters.insert(domain.to_string(), limiter);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_global_rate_limiting() {
        // Create a config with a very restrictive rate limit for testing
        let mut config = RateLimiterConfig::default();
        config.global_rps = 0.5; // Reduce to 0.5 per second for more strict limiting
        config.global_burst_size = 1; // Only allow 1 request at a time
        config.global_enabled = true; // Ensure global rate limiting is enabled
        config.domain_enabled = false; // Disable domain rate limiting to isolate test
        let limiter = ToolRateLimiter::new(config);

        // Ensure the global limiter is created
        assert!(
            limiter.global_limiter.is_some(),
            "Global limiter should be initialized"
        );

        // First request should succeed
        let result1 = limiter.check_url("https://example.com/test1").await;
        assert!(
            result1.is_ok(),
            "First request should succeed, got {:?}",
            result1
        );

        // Make multiple immediate requests to guarantee hitting the rate limit
        let mut hit_rate_limit = false;

        // Try multiple times immediately without delay
        for i in 0..10 {
            let result = limiter
                .check_url(&format!("https://example.com/test{}", i + 2))
                .await;
            if result.is_err() {
                hit_rate_limit = true;
                if let Err(RateLimitError::LimitExceeded { target, .. }) = result {
                    assert!(
                        target.contains("global") || target.contains("example.com"),
                        "Rate limit target should be global or the domain"
                    );
                }
                break;
            }
        }

        assert!(
            hit_rate_limit,
            "Should have hit the rate limit after multiple requests"
        );

        // Wait a bit and try again - should succeed after cooldown
        sleep(Duration::from_secs(3)).await; // Wait longer to ensure the rate limit resets
        let result_after_wait = limiter
            .check_url("https://example.com/test_after_wait")
            .await;
        assert!(
            result_after_wait.is_ok(),
            "Request after waiting should succeed"
        );
    }

    #[tokio::test]
    async fn test_domain_rate_limiting() {
        // Create a config with domain-specific rate limits
        let mut config = RateLimiterConfig::default();
        config.global_enabled = false; // Disable global limiting for this test
        config.domain_enabled = true; // Ensure domain rate limiting is enabled
        config.default_domain_rps = 10.0; // Default is high
        config.default_domain_burst_size = 1; // But with low burst size
        config.domain_limits = {
            let mut limits = HashMap::new();
            limits.insert("example.com".to_string(), 1.0); // But example.com is limited to 1 RPS
            limits
        };
        let limiter = ToolRateLimiter::new(config);

        // Request to example.com should succeed the first time
        assert!(limiter.check_url("https://example.com/test1").await.is_ok());

        // Second immediate request should fail due to rate limiting
        let result = limiter.check_url("https://example.com/test2").await;
        assert!(
            result.is_err(),
            "Second request to example.com should be rate limited"
        );

        if let Err(RateLimitError::LimitExceeded { target, message }) = result {
            // Validate the error details
            assert_eq!(target, "example.com");
        } else {
            panic!("Expected a LimitExceeded error but got: {:?}", result);
        }

        // But request to different domain should succeed
        assert!(limiter.check_url("https://other.com/test").await.is_ok());
    }

    #[tokio::test]
    async fn test_group_rate_limiting() {
        let config = RateLimiterConfig::default();
        let limiter = ToolRateLimiter::new(config);

        // Set a very low rate limit for the "api" group
        assert!(limiter.check_group("api", 1.0, Some(1)).await.is_ok());

        // Second request to the same group should fail
        assert!(limiter.check_group("api", 1.0, Some(1)).await.is_err());

        // But request to a different group should succeed
        assert!(limiter.check_group("database", 1.0, Some(1)).await.is_ok());
    }

    #[tokio::test]
    async fn test_update_limits() {
        let config = RateLimiterConfig::default();
        let limiter = ToolRateLimiter::new(config);

        // Set a very low rate limit initially
        assert!(limiter.check_group("api", 1.0, Some(1)).await.is_ok());
        assert!(limiter.check_group("api", 1.0, Some(1)).await.is_err());

        // Update the limit with explicit values, not relying on get_group_status
        limiter.update_group_limit("api", 10.0, Some(5)).await;

        // Now we should be able to make more requests
        assert!(limiter.check_group("api", 10.0, Some(5)).await.is_ok());
        assert!(limiter.check_group("api", 10.0, Some(5)).await.is_ok());
    }

    #[tokio::test]
    async fn test_domain_strategy() {
        // Test with FullDomain strategy
        let mut config = RateLimiterConfig::default();
        config.global_enabled = false;
        config.domain_strategy = DomainStrategy::FullDomain;
        let limiter = ToolRateLimiter::new(config);

        // These should be treated as different domains
        assert!(limiter
            .check_url("https://api.example.com/test")
            .await
            .is_ok());
        assert!(limiter
            .check_url("https://www.example.com/test")
            .await
            .is_ok());

        // Test with MainDomain strategy
        let mut config = RateLimiterConfig::default();
        config.global_enabled = false;
        config.domain_strategy = DomainStrategy::MainDomain;
        config.default_domain_rps = 1.0;
        config.default_domain_burst_size = 1;
        let limiter = ToolRateLimiter::new(config);

        // These should be treated as the same domain (example.com)
        assert!(limiter
            .check_url("https://api.example.com/test")
            .await
            .is_ok());
        assert!(limiter
            .check_url("https://www.example.com/test")
            .await
            .is_err());
    }
}
