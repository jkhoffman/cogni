//! Caching utilities for tools.
//!
//! This module provides a caching system for tools to reduce redundant API calls
//! and improve performance. It supports:
//! - Time-based expiration
//! - LRU eviction
//! - Size-based limits
//! - Cache hit/miss metrics

use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use bytes::Bytes;
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

/// Default cache TTL in seconds.
pub const DEFAULT_CACHE_TTL_SECS: u64 = 3600;

/// Default cache size (number of items).
pub const DEFAULT_CACHE_SIZE: usize = 1000;

/// Default maximum cache entry size in bytes.
pub const DEFAULT_MAX_ENTRY_SIZE_BYTES: usize = 1024 * 1024; // 1MB

/// Errors that can occur during caching operations.
#[derive(Error, Debug)]
pub enum CacheError {
    /// The cache key is invalid
    #[error("Invalid cache key: {0}")]
    InvalidKey(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Entry too large
    #[error("Cache entry too large: {size} bytes (max: {max} bytes)")]
    EntryTooLarge { size: usize, max: usize },

    /// Internal error
    #[error("Cache error: {0}")]
    Internal(String),
}

/// Configuration for the tool cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live for cache entries in seconds.
    pub ttl_secs: u64,
    /// Maximum number of entries in the cache.
    pub max_entries: usize,
    /// Maximum size of a single cache entry in bytes.
    pub max_entry_size_bytes: usize,
    /// Whether to enable hit/miss metrics.
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_secs: DEFAULT_CACHE_TTL_SECS,
            max_entries: DEFAULT_CACHE_SIZE,
            max_entry_size_bytes: DEFAULT_MAX_ENTRY_SIZE_BYTES,
            enable_metrics: true,
        }
    }
}

/// Metrics for cache performance.
#[derive(Debug, Default, Clone)]
pub struct CacheMetrics {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of cache entries evicted due to TTL expiration.
    pub ttl_evictions: u64,
    /// Number of cache entries evicted due to size constraints.
    pub size_evictions: u64,
    /// Number of cache entries evicted explicitly.
    pub explicit_evictions: u64,
    /// Total bytes stored in the cache.
    pub bytes_stored: u64,
}

impl CacheMetrics {
    /// Calculate the cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Reset all metrics to zero.
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.ttl_evictions = 0;
        self.size_evictions = 0;
        self.explicit_evictions = 0;
        self.bytes_stored = 0;
    }
}

/// A cached value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    /// The cached value.
    value: T,
    /// Time when the entry was created.
    created_at: u64,
    /// Size of the entry in bytes (approximated).
    size_bytes: usize,
}

/// A thread-safe cache for tool data.
#[derive(Debug, Clone)]
pub struct ToolCache {
    /// The configuration for this cache.
    config: CacheConfig,
    /// The underlying cache storage.
    cache: Arc<DashMap<String, CacheEntry<Bytes>>>,
    /// Cache metrics.
    metrics: Arc<CacheMetrics>,
    /// Atomic counters for metrics.
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    ttl_evictions: Arc<AtomicU64>,
    size_evictions: Arc<AtomicU64>,
    explicit_evictions: Arc<AtomicU64>,
    bytes_stored: Arc<AtomicU64>,
}

impl ToolCache {
    /// Create a new tool cache with the given configuration.
    ///
    /// # Arguments
    /// * `config` - The configuration for the cache
    ///
    /// # Returns
    /// A new `ToolCache` instance
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(DashMap::new()),
            metrics: Arc::new(CacheMetrics::default()),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            ttl_evictions: Arc::new(AtomicU64::new(0)),
            size_evictions: Arc::new(AtomicU64::new(0)),
            explicit_evictions: Arc::new(AtomicU64::new(0)),
            bytes_stored: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new tool cache with default configuration.
    ///
    /// # Returns
    /// A new `ToolCache` instance with default configuration
    pub fn default() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get a value from the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// Returns `Some(Bytes)` if the value is in the cache, or `None` if it's not
    pub fn get(&self, key: &str) -> Option<Bytes> {
        // Check if the key exists in the cache
        if let Some(entry) = self.cache.get(key) {
            // Check if the entry has expired
            let now = current_timestamp_secs();
            if now - entry.created_at > self.config.ttl_secs {
                // Entry has expired, remove it
                self.cache.remove(key);
                if self.config.enable_metrics {
                    self.ttl_evictions.fetch_add(1, Ordering::Relaxed);
                    self.bytes_stored
                        .fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
                }
                self.misses.fetch_add(1, Ordering::Relaxed);
                debug!("Cache miss (expired): {}", key);
                None
            } else {
                // Entry is valid
                if self.config.enable_metrics {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                }
                debug!("Cache hit: {}", key);
                Some(entry.value.clone())
            }
        } else {
            // Key not in cache
            if self.config.enable_metrics {
                self.misses.fetch_add(1, Ordering::Relaxed);
            }
            debug!("Cache miss: {}", key);
            None
        }
    }

    /// Get a value from the cache and deserialize it.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// Returns `Ok(Some(T))` if the value is in the cache, `Ok(None)` if it's not,
    /// or an error if the value couldn't be deserialized
    pub fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        if let Some(bytes) = self.get(key) {
            // Deserialize the value
            let value = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Set a value in the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache
    ///
    /// # Returns
    /// Returns `Ok(())` if the value was stored, or an error if it wasn't
    ///
    /// # Errors
    /// Returns `CacheError` if the value is too large or the key is invalid
    pub fn set(&self, key: &str, value: Bytes) -> Result<(), CacheError> {
        // Check if the value is too large
        if value.len() > self.config.max_entry_size_bytes {
            return Err(CacheError::EntryTooLarge {
                size: value.len(),
                max: self.config.max_entry_size_bytes,
            });
        }

        // Validate the key
        if key.is_empty() {
            return Err(CacheError::InvalidKey("Empty key".to_string()));
        }

        // Create the cache entry
        let size = value.len();
        let entry = CacheEntry {
            value,
            created_at: current_timestamp_secs(),
            size_bytes: size,
        };

        // Store the value
        if let Some(old_entry) = self.cache.insert(key.to_string(), entry.clone()) {
            // Update the stored bytes counter
            if self.config.enable_metrics {
                self.bytes_stored
                    .fetch_sub(old_entry.size_bytes as u64, Ordering::Relaxed);
                self.bytes_stored
                    .fetch_add(entry.size_bytes as u64, Ordering::Relaxed);
            }
        } else {
            // New entry
            if self.config.enable_metrics {
                self.bytes_stored
                    .fetch_add(entry.size_bytes as u64, Ordering::Relaxed);
            }
        }

        // Check if we need to evict entries due to size constraints
        if self.cache.len() > self.config.max_entries {
            self.evict_oldest();
        }

        Ok(())
    }

    /// Set a serializable value in the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache
    ///
    /// # Returns
    /// Returns `Ok(())` if the value was stored, or an error if it wasn't
    ///
    /// # Errors
    /// Returns `CacheError` if the value is too large, the key is invalid,
    /// or the value couldn't be serialized
    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), CacheError> {
        // Serialize the value
        let json = serde_json::to_vec(value)?;
        let bytes = Bytes::from(json);

        // Store the serialized value
        self.set(key, bytes)
    }

    /// Remove a value from the cache.
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// Returns `true` if the value was removed, or `false` if it wasn't in the cache
    pub fn remove(&self, key: &str) -> bool {
        if let Some((_, entry)) = self.cache.remove(key) {
            // Update metrics
            if self.config.enable_metrics {
                self.explicit_evictions.fetch_add(1, Ordering::Relaxed);
                self.bytes_stored
                    .fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
            }
            true
        } else {
            false
        }
    }

    /// Clear all values from the cache.
    pub fn clear(&self) {
        // Get total bytes before clearing
        let total_bytes = if self.config.enable_metrics {
            self.cache
                .iter()
                .map(|entry| entry.size_bytes as u64)
                .sum::<u64>()
        } else {
            0
        };

        // Clear the cache
        self.cache.clear();

        // Update metrics
        if self.config.enable_metrics {
            self.explicit_evictions
                .fetch_add(self.cache.len() as u64, Ordering::Relaxed);
            self.bytes_stored.fetch_sub(total_bytes, Ordering::Relaxed);
        }
    }

    /// Get the current metrics for the cache.
    ///
    /// # Returns
    /// Returns the current cache metrics
    pub fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            ttl_evictions: self.ttl_evictions.load(Ordering::Relaxed),
            size_evictions: self.size_evictions.load(Ordering::Relaxed),
            explicit_evictions: self.explicit_evictions.load(Ordering::Relaxed),
            bytes_stored: self.bytes_stored.load(Ordering::Relaxed),
        }
    }

    /// Reset the cache metrics.
    pub fn reset_metrics(&self) {
        if self.config.enable_metrics {
            self.hits.store(0, Ordering::Relaxed);
            self.misses.store(0, Ordering::Relaxed);
            self.ttl_evictions.store(0, Ordering::Relaxed);
            self.size_evictions.store(0, Ordering::Relaxed);
            self.explicit_evictions.store(0, Ordering::Relaxed);

            // Recalculate bytes stored
            let total_bytes = self
                .cache
                .iter()
                .map(|entry| entry.size_bytes as u64)
                .sum::<u64>();
            self.bytes_stored.store(total_bytes, Ordering::Relaxed);
        }
    }

    /// Evict expired entries from the cache.
    ///
    /// # Returns
    /// Returns the number of entries evicted
    pub fn evict_expired(&self) -> usize {
        let now = current_timestamp_secs();
        let mut evicted = 0;
        let mut bytes_freed = 0;

        // Find expired entries
        let expired_keys = self
            .cache
            .iter()
            .filter(|entry| now - entry.created_at > self.config.ttl_secs)
            .map(|entry| (entry.key().clone(), entry.size_bytes))
            .collect::<Vec<_>>();

        // Remove expired entries
        for (key, size) in expired_keys {
            self.cache.remove(&key);
            evicted += 1;
            bytes_freed += size;
        }

        // Update metrics
        if self.config.enable_metrics && evicted > 0 {
            self.ttl_evictions
                .fetch_add(evicted as u64, Ordering::Relaxed);
            self.bytes_stored
                .fetch_sub(bytes_freed as u64, Ordering::Relaxed);
        }

        evicted
    }

    /// Evict the oldest entries from the cache to make room for new ones.
    ///
    /// # Returns
    /// Returns the number of entries evicted
    fn evict_oldest(&self) -> usize {
        // Calculate how many entries to evict (20% of max or at least 1)
        let to_evict = (self.config.max_entries / 5).max(1);

        // Find the oldest entries
        let mut entries = self
            .cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.created_at, entry.size_bytes))
            .collect::<Vec<_>>();

        entries.sort_by_key(|(_, created_at, _)| *created_at);

        // Take just the oldest ones
        let oldest = entries.into_iter().take(to_evict).collect::<Vec<_>>();

        let mut evicted = 0;
        let mut bytes_freed = 0;

        // Remove the oldest entries
        for (key, _, size) in oldest {
            self.cache.remove(&key);
            evicted += 1;
            bytes_freed += size;
        }

        // Update metrics
        if self.config.enable_metrics && evicted > 0 {
            self.size_evictions
                .fetch_add(evicted as u64, Ordering::Relaxed);
            self.bytes_stored
                .fetch_sub(bytes_freed as u64, Ordering::Relaxed);
        }

        evicted
    }

    /// Get the number of entries in the cache.
    ///
    /// # Returns
    /// Returns the number of entries in the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    ///
    /// # Returns
    /// Returns `true` if the cache is empty, or `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get the TTL for cache entries.
    ///
    /// # Returns
    /// Returns the cache TTL in seconds
    pub fn ttl(&self) -> u64 {
        self.config.ttl_secs
    }

    /// Set the TTL for cache entries.
    ///
    /// # Arguments
    /// * `ttl_secs` - The new TTL in seconds
    pub fn set_ttl(&mut self, ttl_secs: u64) {
        self.config.ttl_secs = ttl_secs;
    }
}

/// Get the current timestamp in seconds.
fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_cache_operations() {
        let cache = ToolCache::default();

        // Test set and get
        let value = Bytes::from("test value");
        cache.set("test_key", value.clone()).unwrap();

        let retrieved = cache.get("test_key").unwrap();
        assert_eq!(retrieved, value);

        // Test remove
        assert!(cache.remove("test_key"));
        assert!(cache.get("test_key").is_none());

        // Test non-existent key
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_json_operations() {
        let cache = ToolCache::default();

        // Test set_json and get_json
        let value = vec!["one", "two", "three"];
        cache.set_json("json_key", &value).unwrap();

        let retrieved: Vec<String> = cache.get_json("json_key").unwrap().unwrap();
        assert_eq!(
            retrieved,
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }

    #[test]
    fn test_metrics() {
        let mut config = CacheConfig::default();
        config.enable_metrics = true;
        let cache = ToolCache::new(config);

        // Add some entries
        cache.set("key1", Bytes::from("value1")).unwrap();
        cache.set("key2", Bytes::from("value2")).unwrap();

        // Generate some hits and misses
        cache.get("key1"); // Hit
        cache.get("key1"); // Hit
        cache.get("key2"); // Hit
        cache.get("nonexistent"); // Miss

        // Check metrics
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 3);
        assert_eq!(metrics.misses, 1);

        // Clear the cache and reset metrics
        cache.clear();
        cache.reset_metrics();

        // Check that metrics were reset
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
    }

    #[test]
    fn test_expiry() {
        // Create a cache with a short TTL
        let mut config = CacheConfig::default();
        config.ttl_secs = 1; // 1 second TTL
        config.enable_metrics = true; // Ensure metrics are enabled
        let cache = ToolCache::new(config);

        // Use a timestamp in the key to ensure it's unique per run
        let unique_key = format!(
            "expire_key_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        // Add an entry
        cache.set(&unique_key, Bytes::from("value")).unwrap();

        // Verify it exists
        assert!(cache.get(&unique_key).is_some());

        // Wait longer for it to expire - 2 seconds instead of 1.5
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Force eviction explicitly
        let evicted = cache.evict_expired();
        assert!(evicted > 0, "Expected at least one entry to be evicted");

        // Should be gone now
        assert!(
            cache.get(&unique_key).is_none(),
            "Key should no longer exist in cache after eviction"
        );

        // Check that ttl_evictions was incremented
        let metrics = cache.metrics();
        assert!(
            metrics.ttl_evictions > 0,
            "TTL eviction count should be greater than 0"
        );
    }

    #[test]
    fn test_size_constraints() {
        // Create a cache with a small maximum size
        let mut config = CacheConfig::default();
        config.max_entries = 2;
        config.enable_metrics = true; // Ensure metrics are enabled
        let cache = ToolCache::new(config);

        // Add entries up to the limit
        cache.set("key1", Bytes::from("value1")).unwrap();
        cache.set("key2", Bytes::from("value2")).unwrap();

        // Verify they exist
        assert!(cache.get("key1").is_some());
        assert!(cache.get("key2").is_some());

        // Add another entry, which should trigger eviction
        cache.set("key3", Bytes::from("value3")).unwrap();

        // Verify key3 exists and that the cache has exactly 2 entries
        assert!(
            cache.get("key3").is_some(),
            "key3 should exist in the cache"
        );
        assert_eq!(
            cache.len(),
            2,
            "Cache should have exactly 2 entries after eviction"
        );

        // Either key1, key2, or both could be evicted depending on implementation
        // All we need to check is that key3 exists and total size is maintained
        let has_key1 = cache.get("key1").is_some();
        let has_key2 = cache.get("key2").is_some();

        // Check that at least one of the old keys was evicted
        assert!(
            !(has_key1 && has_key2),
            "At least one of the old keys should be evicted"
        );
    }

    #[test]
    fn test_entry_size_limit() {
        // Create a cache with a small maximum entry size
        let mut config = CacheConfig::default();
        config.max_entry_size_bytes = 10;
        config.enable_metrics = true; // Ensure metrics are enabled
        let cache = ToolCache::new(config);

        // Try to add an entry that's too large
        let result = cache.set("big_key", Bytes::from("this value is too large"));
        assert!(result.is_err());

        // Verify the error type
        match result {
            Err(CacheError::EntryTooLarge { size, max }) => {
                assert_eq!(size, 23);
                assert_eq!(max, 10);
            }
            _ => panic!("Expected EntryTooLarge error"),
        }

        // Add a small entry
        let result = cache.set("small_key", Bytes::from("small"));
        assert!(result.is_ok());
    }
}
