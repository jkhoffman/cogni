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
            let entry = self.entries.shift_remove(key).unwrap();
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
        self.entries.shift_remove(&key);

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
        self.entries.shift_remove(key);
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
#[derive(Clone)]
pub struct CacheService<S> {
    inner: S,
    cache: Arc<RwLock<ResponseCache>>,
}

impl<S> Service<Request> for CacheService<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
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

        // Clone the inner service for the async block
        let mut inner = self.inner.clone();
        
        Box::pin(async move {
            // Try to get from cache
            if let Some(response) = cache_check.write().await.get(&key_check) {
                return Ok(response);
            }

            // Not in cache, execute request
            let response = inner.call(request).await?;

            // Cache the response
            cache.write().await.put(key, response.clone());

            Ok(response)
        })
    }
}

/// Re-export for convenience
pub use CacheLayer as CacheMiddleware;

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Audio, Content, Image, Message, Model, Request, Response, ResponseMetadata, Role};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    /// Mock service that tracks call count
    #[derive(Clone)]
    struct MockService {
        call_count: Arc<AtomicUsize>,
        response: Response,
    }

    impl Service<Request> for MockService {
        type Response = Response;
        type Error = Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

        fn call(&mut self, _request: Request) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let response = self.response.clone();
            Box::pin(async move { Ok(response) })
        }
    }

    fn create_test_request(content: &str) -> Request {
        Request::builder()
            .model(Model("test-model".into()))
            .message(Message::user(content))
            .build()
    }

    fn create_test_response(content: &str) -> Response {
        Response {
            content: content.to_string(),
            tool_calls: Vec::new(),
            metadata: ResponseMetadata::default(),
        }
    }

    #[test]
    fn test_cache_key_generation() {
        let request1 = create_test_request("Hello");
        let request2 = create_test_request("Hello");
        let request3 = create_test_request("World");

        let key1 = CacheKey::from_request(&request1);
        let key2 = CacheKey::from_request(&request2);
        let key3 = CacheKey::from_request(&request3);

        // Same requests should produce same keys
        assert_eq!(key1, key2);
        // Different requests should produce different keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_with_parameters() {
        let mut request1 = create_test_request("Hello");
        request1.parameters.temperature = Some(0.7);
        
        let mut request2 = create_test_request("Hello");
        request2.parameters.temperature = Some(0.8);
        
        let mut request3 = create_test_request("Hello");
        request3.parameters.max_tokens = Some(100);

        let key1 = CacheKey::from_request(&request1);
        let key2 = CacheKey::from_request(&request2);
        let key3 = CacheKey::from_request(&request3);

        // Different temperatures should produce different keys
        assert_ne!(key1, key2);
        // Different parameters should produce different keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_with_different_content_types() {
        let text_request = create_test_request("Hello");
        
        let image_request = Request::builder()
            .model(Model("test-model".into()))
            .message(Message {
                role: Role::User,
                content: Content::Image(Image {
                    url: Some("https://example.com/image.jpg".into()),
                    data: None,
                    mime_type: "image/jpeg".into(),
                }),
                metadata: Default::default(),
            })
            .build();
        
        let audio_request = Request::builder()
            .model(Model("test-model".into()))
            .message(Message {
                role: Role::User,
                content: Content::Audio(Audio {
                    data: "base64-audio-data".into(),
                    mime_type: "audio/mpeg".into(),
                }),
                metadata: Default::default(),
            })
            .build();

        let key1 = CacheKey::from_request(&text_request);
        let key2 = CacheKey::from_request(&image_request);
        let key3 = CacheKey::from_request(&audio_request);

        // Different content types should produce different keys
        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key2, key3);
    }

    #[tokio::test]
    async fn test_response_cache_basic() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));
        let key = CacheKey("test-key".into());
        let response = create_test_response("cached response");

        // Cache miss
        assert!(cache.get(&key).is_none());

        // Put in cache
        cache.put(key.clone(), response.clone());

        // Cache hit
        let cached = cache.get(&key).unwrap();
        assert_eq!(cached.content, "cached response");
    }

    #[tokio::test]
    async fn test_response_cache_lru_eviction() {
        let mut cache = ResponseCache::new(3, Duration::from_secs(60));

        // Fill cache to capacity
        for i in 0..3 {
            let key = CacheKey(format!("key-{}", i));
            let response = create_test_response(&format!("response-{}", i));
            cache.put(key, response);
        }

        // At this point, order is: key-0, key-1, key-2 (key-0 is oldest)

        // Access key-0 to make it most recently used
        let key0 = CacheKey("key-0".into());
        assert!(cache.get(&key0).is_some());
        
        // After get, order should be: key-1, key-2, key-0 (key-1 is oldest)

        // Add a new item, should evict key-1 (oldest)
        let key3 = CacheKey("key-3".into());
        cache.put(key3.clone(), create_test_response("response-3"));

        // key-1 should be evicted
        assert!(cache.get(&CacheKey("key-1".into())).is_none());
        // Others should still be present
        assert!(cache.get(&key0).is_some());
        assert!(cache.get(&CacheKey("key-2".into())).is_some());
        assert!(cache.get(&key3).is_some());
    }

    #[tokio::test]
    async fn test_response_cache_ttl_expiration() {
        let mut cache = ResponseCache::new(10, Duration::from_millis(50));
        let key = CacheKey("test-key".into());
        let response = create_test_response("cached response");

        cache.put(key.clone(), response);
        
        // Should be in cache immediately
        assert!(cache.get(&key).is_some());

        // Wait for TTL to expire
        sleep(Duration::from_millis(100)).await;

        // Should be expired and removed
        assert!(cache.get(&key).is_none());
    }

    #[tokio::test]
    async fn test_response_cache_clear_expired() {
        let mut cache = ResponseCache::new(10, Duration::from_millis(50));

        // Add entries with different creation times
        for i in 0..3 {
            let key = CacheKey(format!("key-{}", i));
            let response = create_test_response(&format!("response-{}", i));
            cache.put(key, response);
            if i < 2 {
                sleep(Duration::from_millis(60)).await;
            }
        }

        // key-0 and key-1 should be expired, key-2 should still be valid
        cache.clear_expired();

        assert!(cache.get(&CacheKey("key-0".into())).is_none());
        assert!(cache.get(&CacheKey("key-1".into())).is_none());
        assert!(cache.get(&CacheKey("key-2".into())).is_some());
    }

    #[tokio::test]
    async fn test_cache_service_caches_responses() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockService {
            call_count: call_count.clone(),
            response: create_test_response("test response"),
        };

        let cache_layer = CacheLayer::new(10, Duration::from_secs(60));
        let mut cached_service = cache_layer.layer(mock_service);

        let request = create_test_request("test");

        // First call should hit the underlying service
        let response1 = cached_service.call(request.clone()).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert_eq!(response1.content, "test response");

        // Second call should be cached
        let response2 = cached_service.call(request.clone()).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // Still 1, not called again
        assert_eq!(response2.content, "test response");
    }

    #[tokio::test]
    async fn test_cache_service_different_requests() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockService {
            call_count: call_count.clone(),
            response: create_test_response("test response"),
        };

        let cache_layer = CacheLayer::new(10, Duration::from_secs(60));
        let mut cached_service = cache_layer.layer(mock_service);

        let request1 = create_test_request("test1");
        let request2 = create_test_request("test2");

        // First request
        cached_service.call(request1.clone()).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Different request should hit the service
        cached_service.call(request2.clone()).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // Same requests should be cached
        cached_service.call(request1).await.unwrap();
        cached_service.call(request2).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2); // Still 2
    }

    #[tokio::test]
    async fn test_cache_service_error_propagation() {
        #[derive(Clone)]
        struct ErrorService;

        impl Service<Request> for ErrorService {
            type Response = Response;
            type Error = Error;
            type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

            fn call(&mut self, _request: Request) -> Self::Future {
                Box::pin(async move { 
                    Err(Error::Provider {
                        provider: "test".into(),
                        message: "test error".into(),
                        retry_after: None,
                        source: None,
                    })
                })
            }
        }

        let cache_layer = CacheLayer::new(10, Duration::from_secs(60));
        let mut cached_service = cache_layer.layer(ErrorService);

        let request = create_test_request("test");
        let result = cached_service.call(request).await;

        assert!(result.is_err());
        if let Err(Error::Provider { message, .. }) = result {
            assert_eq!(message, "test error");
        } else {
            panic!("Expected Provider error");
        }
    }

    #[tokio::test]
    async fn test_cache_layer_clone() {
        let layer1 = CacheLayer::new(10, Duration::from_secs(60));
        let layer2 = layer1.clone();

        // Both layers should share the same cache
        let shared_count = Arc::new(AtomicUsize::new(0));
        let mock_service1 = MockService {
            call_count: shared_count.clone(),
            response: create_test_response("test"),
        };
        let mock_service2 = MockService {
            call_count: shared_count.clone(),
            response: create_test_response("test"),
        };

        let mut service1 = layer1.layer(mock_service1);
        let mut service2 = layer2.layer(mock_service2);

        let request = create_test_request("test");

        // Cache through service1
        service1.call(request.clone()).await.unwrap();
        assert_eq!(shared_count.load(Ordering::SeqCst), 1);

        // Should hit cache through service2 
        let response = service2.call(request).await.unwrap();
        // Verify the response is correct 
        assert_eq!(response.content, "test");
        // Count should still be 1 since cache was hit
        assert_eq!(shared_count.load(Ordering::SeqCst), 1);
    }
}
