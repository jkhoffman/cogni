//! Caching middleware for response caching

use crate::{BoxFuture, Layer, Service};
use cogni_core::{Error, Request, Response};
use indexmap::IndexMap;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Cache middleware layer
#[derive(Debug, Clone)]
pub struct CacheLayer {
    /// Cache instance
    cache: Arc<RwLock<ResponseCache>>,
}

/// Cache key for requests
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    /// Create a cache key from a request
    pub fn from_request(request: &Request) -> Self {
        let mut hasher = Sha256::new();

        // Hash the model
        hasher.update(request.model.0.as_bytes());
        hasher.update(b"|");

        // Hash messages
        for msg in &request.messages {
            // Use a byte representation for role instead of format!
            match msg.role {
                cogni_core::Role::System => hasher.update(b"0"),
                cogni_core::Role::User => hasher.update(b"1"),
                cogni_core::Role::Assistant => hasher.update(b"2"),
                cogni_core::Role::Tool => hasher.update(b"3"),
                _ => hasher.update(b"9"), // Unknown roles
            }
            hasher.update(b"|");

            match &msg.content {
                cogni_core::Content::Text(text) => {
                    hasher.update(b"text:");
                    hasher.update(text.as_bytes());
                }
                cogni_core::Content::Image(image) => {
                    hasher.update(b"image:");
                    if let Some(url) = &image.url {
                        hasher.update(url.as_bytes());
                    }
                }
                cogni_core::Content::Audio(audio) => {
                    hasher.update(b"audio:");
                    hasher.update(audio.data.as_bytes());
                }
                cogni_core::Content::Multiple(parts) => {
                    hasher.update(b"multiple:");
                    hasher.update((parts.len() as u32).to_le_bytes());
                }
            }
            hasher.update(b"|");
        }

        // Hash temperature if set
        if let Some(temp) = request.parameters.temperature {
            hasher.update(b"temp:");
            hasher.update(temp.to_le_bytes());
            hasher.update(b"|");
        }

        // Hash max_tokens if set
        if let Some(max) = request.parameters.max_tokens {
            hasher.update(b"max:");
            hasher.update(max.to_le_bytes());
            hasher.update(b"|");
        }

        // Hash tools
        for tool in &request.tools {
            hasher.update(b"tool:");
            hasher.update(tool.name.as_bytes());
            hasher.update(b"|");
        }

        let result = hasher.finalize();
        CacheKey(format!("{:x}", result))
    }
}

/// Response cache with TTL and LRU eviction
#[derive(Debug)]
pub struct ResponseCache {
    /// Cached responses with LRU ordering
    entries: IndexMap<CacheKey, CacheEntry>,
    /// Maximum cache size
    max_size: usize,
    /// Default TTL for entries
    default_ttl: Duration,
}

/// A cached response entry
#[derive(Debug, Clone)]
struct CacheEntry {
    response: Response,
    created_at: Instant,
    ttl: Duration,
}

impl ResponseCache {
    /// Create a new cache
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            entries: IndexMap::new(),
            max_size,
            default_ttl,
        }
    }

    /// Get a response from cache
    pub fn get(&mut self, key: &CacheKey) -> Option<Response> {
        if let Some(entry) = self.entries.get(key) {
            // Check if expired
            if entry.created_at.elapsed() > entry.ttl {
                self.remove(key);
                return None;
            }

            // Move to end for LRU (IndexMap maintains insertion order)
            let entry = self.entries.swap_remove(key).unwrap();
            self.entries.insert(key.clone(), entry.clone());

            trace!(cache_key = %key.0, "Cache hit");
            Some(entry.response.clone())
        } else {
            trace!(cache_key = %key.0, "Cache miss");
            None
        }
    }

    /// Put a response in cache
    pub fn put(&mut self, key: CacheKey, response: Response) {
        // Remove if already exists to update position
        self.entries.swap_remove(&key);

        // Evict oldest if at capacity (first entry in IndexMap)
        while self.entries.len() >= self.max_size {
            if let Some((oldest, _)) = self.entries.shift_remove_index(0) {
                debug!(cache_key = %oldest.0, "Evicted oldest cache entry");
            }
        }

        // Insert new entry
        self.entries.insert(
            key.clone(),
            CacheEntry {
                response,
                created_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
        // No need to update access_order, IndexMap handles it

        debug!(
            cache_key = %key.0,
            cache_size = self.entries.len(),
            "Cached response"
        );
    }

    /// Remove an entry
    fn remove(&mut self, key: &CacheKey) {
        self.entries.swap_remove(key);
    }

    /// Clear expired entries
    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        // Use retain to avoid intermediate allocation
        self.entries.retain(|key, entry| {
            let expired = now.duration_since(entry.created_at) > entry.ttl;
            if expired {
                debug!(cache_key = %key.0, "Removed expired cache entry");
            }
            !expired
        });
    }
}

impl CacheLayer {
    /// Create a new cache layer
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(ResponseCache::new(max_size, ttl))),
        }
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            cache: self.cache.clone(),
        }
    }
}

/// Cache service
pub struct CacheService<S> {
    inner: S,
    cache: Arc<RwLock<ResponseCache>>,
}

impl<S> Service<Request> for CacheService<S>
where
    S: Service<Request, Response = Response, Error = Error>,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, request: Request) -> Self::Future {
        let cache = self.cache.clone();
        let key = CacheKey::from_request(&request);

        // Check cache first
        let cache_check = cache.clone();
        let key_check = key.clone();

        let fut = self.inner.call(request);

        Box::pin(async move {
            // Try to get from cache
            if let Some(response) = cache_check.write().await.get(&key_check) {
                return Ok(response);
            }

            // Not in cache, execute request
            let response = fut.await?;

            // Cache the response
            cache.write().await.put(key, response.clone());

            Ok(response)
        })
    }
}

/// Re-export for convenience
pub use CacheLayer as CacheMiddleware;
