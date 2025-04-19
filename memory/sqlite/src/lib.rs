//! SQLite memory backend for the Cogni framework.
//!
//! This crate provides a SQLite-based implementation of the `MemoryStore` trait.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::{
    error::MemoryError,
    memory::{MemoryEntry, MemoryStore, SessionId},
};
use sqlx::{Pool, Sqlite, sqlite::SqlitePool};
use tracing::{debug, instrument};

/// Configuration for the SQLite memory store.
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// The database URL
    database_url: String,
    /// Maximum connections in the pool
    max_connections: u32,
}

impl SqliteConfig {
    /// Create a new configuration with the given database URL.
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            max_connections: 5,
        }
    }

    /// Set the maximum number of connections in the pool.
    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }
}

/// A SQLite-based memory store.
pub struct SqliteStore {
    pool: Pool<Sqlite>,
}

impl SqliteStore {
    /// Create a new SQLite store with the given configuration.
    pub async fn new(config: SqliteConfig) -> Result<Self, MemoryError> {
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.database_url)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .foreign_keys(true)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl MemoryStore for SqliteStore {
    #[instrument(skip(self))]
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        todo!("Implement memory loading")
    }

    #[instrument(skip(self))]
    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError> {
        todo!("Implement memory saving")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[tokio::test]
    async fn test_store_creation() {
        let config = SqliteConfig::new(":memory:");
        let store = SqliteStore::new(config).await.unwrap();

        // Test will be expanded when load/save are implemented
    }
}
