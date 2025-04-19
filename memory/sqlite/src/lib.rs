//! SQLite memory backend for the Cogni framework.
//!
//! This crate provides a SQLite-based implementation of the `MemoryStore` trait.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::{
    error::MemoryError,
    memory::{MemoryEntry, MemoryStore, Role, SessionId},
};
use sqlx::{Pool, Sqlite, sqlite::SqlitePool};
use time::OffsetDateTime;
use tracing::instrument;

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

    /// Convert a role string from the database to a Role enum
    fn parse_role(role: &str) -> Result<Role, MemoryError> {
        match role {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            _ => Err(MemoryError::InvalidFormat(format!(
                "Invalid role: {}",
                role
            ))),
        }
    }

    /// Convert a Role enum to a string for database storage
    fn role_to_string(role: Role) -> &'static str {
        match role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

#[async_trait]
impl MemoryStore for SqliteStore {
    #[instrument(skip(self))]
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        // Query the latest n entries for the session, ordered by timestamp
        let session_str = session.to_string();
        let limit = n as i64;
        let entries = sqlx::query!(
            r#"
            SELECT role, content, timestamp
            FROM memory_entries
            WHERE session_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            session_str,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Convert rows to MemoryEntry structs
        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            let role = Self::parse_role(&entry.role)?;
            let timestamp = OffsetDateTime::parse(
                &entry.timestamp,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;

            result.push(MemoryEntry {
                role,
                content: entry.content,
                timestamp,
            });
        }

        // Reverse to get chronological order
        result.reverse();
        Ok(result)
    }

    #[instrument(skip(self))]
    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError> {
        // Start a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Insert the entry
        let session_str = session.to_string();
        let role_str = Self::role_to_string(entry.role);
        let timestamp_str = entry
            .timestamp
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;

        sqlx::query!(
            r#"
            INSERT INTO memory_entries (session_id, role, content, timestamp)
            VALUES (?, ?, ?, ?)
            "#,
            session_str,
            role_str,
            entry.content,
            timestamp_str
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[tokio::test]
    async fn test_store_creation() {
        let config = SqliteConfig::new(":memory:");
        let store = SqliteStore::new(config).await.unwrap();

        // Test saving and loading entries
        let session = SessionId::new("test-session");
        let entry = MemoryEntry {
            role: Role::User,
            content: "Hello".to_string(),
            timestamp: datetime!(2024-04-01 12:00:00.0 UTC),
        };

        // Save the entry
        store.save(&session, entry.clone()).await.unwrap();

        // Load entries
        let loaded = store.load(&session, 10).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].role, Role::User);
        assert_eq!(loaded[0].content, "Hello");
        assert_eq!(loaded[0].timestamp, datetime!(2024-04-01 12:00:00.0 UTC));
    }

    #[tokio::test]
    async fn test_multiple_entries() {
        let config = SqliteConfig::new(":memory:");
        let store = SqliteStore::new(config).await.unwrap();
        let session = SessionId::new("test-session");

        // Save multiple entries
        let entries = vec![
            MemoryEntry {
                role: Role::User,
                content: "Hello".to_string(),
                timestamp: datetime!(2024-04-01 12:00:00.0 UTC),
            },
            MemoryEntry {
                role: Role::Assistant,
                content: "Hi there!".to_string(),
                timestamp: datetime!(2024-04-01 12:00:01.0 UTC),
            },
            MemoryEntry {
                role: Role::User,
                content: "How are you?".to_string(),
                timestamp: datetime!(2024-04-01 12:00:02.0 UTC),
            },
        ];

        for entry in entries.clone() {
            store.save(&session, entry).await.unwrap();
        }

        // Load all entries
        let loaded = store.load(&session, 10).await.unwrap();
        assert_eq!(loaded.len(), 3);

        // Verify chronological order
        for (i, entry) in loaded.iter().enumerate() {
            assert_eq!(entry.role, entries[i].role);
            assert_eq!(entry.content, entries[i].content);
            assert_eq!(entry.timestamp, entries[i].timestamp);
        }

        // Test limit
        let limited = store.load(&session, 2).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].content, entries[0].content);
        assert_eq!(limited[1].content, entries[1].content);
    }

    #[tokio::test]
    async fn test_invalid_role() {
        let config = SqliteConfig::new(":memory:");
        let store = SqliteStore::new(config).await.unwrap();

        // Try to parse an invalid role
        let result = SqliteStore::parse_role("invalid");
        assert!(matches!(result, Err(MemoryError::InvalidFormat(_))));
    }
}
