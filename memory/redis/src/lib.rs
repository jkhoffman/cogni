//! Redis memory backend for the Cogni framework.
//!
//! This crate provides a Redis-based implementation of the `MemoryStore` trait.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use cogni_core::error::MemoryError;
use cogni_core::traits::memory::{MemoryEntry, MemoryQuery, MemoryStore, SessionId};
use redis::{Client, Commands};
use tracing::instrument;

/// Configuration for the Redis memory store.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used in future implementations
pub struct RedisConfig {
    /// The Redis URL
    url: String,
    /// The prefix for Redis keys
    prefix: String,
}

impl RedisConfig {
    /// Create a new configuration with the given Redis URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            prefix: "cogni:memory:".to_string(),
        }
    }

    /// Set a custom key prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
}

/// A Redis-based memory store.
pub struct RedisMemory {
    client: Client,
    prefix: String,
}

impl RedisMemory {
    /// Create a new Redis store with the given configuration.
    pub fn new(url: &str, prefix: &str) -> Result<Self, MemoryError> {
        let client = Client::open(url).map_err(|e| MemoryError::Database(e.to_string()))?;
        Ok(Self {
            client,
            prefix: prefix.to_string(),
        })
    }

    fn session_key(&self, session: &SessionId) -> String {
        format!("{}:{}", self.prefix, session)
    }
}

#[async_trait::async_trait]
impl MemoryStore for RedisMemory {
    #[instrument(skip(self))]
    async fn load(&self, session: &SessionId, _n: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        let key = self.session_key(session);
        let entries: redis::RedisResult<Vec<String>> = conn.lrange(&key, 0, -1);
        let entries = entries.map_err(|e| MemoryError::Database(e.to_string()))?;

        entries
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json).map_err(|e| MemoryError::InvalidFormat(e.to_string()))
            })
            .collect()
    }

    #[instrument(skip(self))]
    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        let key = self.session_key(session);
        let json =
            serde_json::to_string(&entry).map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;

        let _: redis::RedisResult<()> = conn.rpush(&key, json);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn query_history(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        let key = self.session_key(&query.session);
        let entries: redis::RedisResult<Vec<String>> = conn.lrange(&key, 0, -1);
        let entries = entries.map_err(|e| MemoryError::Database(e.to_string()))?;

        let mut result = Vec::new();

        for json in entries {
            let entry: MemoryEntry = match serde_json::from_str(&json) {
                Ok(e) => e,
                Err(e) => return Err(MemoryError::InvalidFormat(e.to_string())),
            };

            // Filter by start_time
            if let Some(start_time) = query.start_time {
                if entry.timestamp < start_time {
                    continue;
                }
            }

            // Filter by end_time
            if let Some(end_time) = query.end_time {
                if entry.timestamp > end_time {
                    continue;
                }
            }

            // Filter by role
            if let Some(role) = query.role {
                if entry.role != role {
                    continue;
                }
            }

            // Filter by content_substring
            if let Some(ref substring) = query.content_substring {
                if !entry.content.contains(substring) {
                    continue;
                }
            }

            result.push(entry);
        }

        // Sort by timestamp ascending (should already be sorted, but ensure)
        result.sort_by_key(|e| e.timestamp);

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(result.len());

        let sliced = if offset >= result.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(offset + limit, result.len());
            result[offset..end].to_vec()
        };

        Ok(sliced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_creation() {
        let config = RedisConfig {
            url: "redis://localhost".to_string(),
            prefix: "test".to_string(),
        };
        let _store = RedisMemory::new(&config.url, &config.prefix).unwrap();
    }
}
