//! Redis memory backend for the Cogni framework.
//!
//! This crate provides a Redis-based implementation of the `MemoryStore` trait.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cogni_core::error::MemoryError;
use cogni_core::traits::memory::{MemoryEntry, MemoryStore, SessionId};
use redis::{Client, Commands, RedisResult};
use serde::{Deserialize, Serialize};
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
        let entries: RedisResult<Vec<String>> = conn.lrange(&key, 0, -1);
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

        let _: RedisResult<()> = conn.rpush(&key, json);
        Ok(())
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
