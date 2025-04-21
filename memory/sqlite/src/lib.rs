//! SQLite memory backend for the Cogni framework.
//!
//! This crate provides a SQLite-based implementation of the `MemoryStore` trait.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use async_trait::async_trait;
use cogni_core::error::MemoryError;
use cogni_core::traits::memory::{MemoryEntry, MemoryQuery, MemoryStore, Role, SessionId};
use sqlx::{Pool, Row, Sqlite, sqlite::SqlitePool};
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

        let rows = sqlx::query(
            r#"
            SELECT role, content, timestamp
            FROM memory_entries
            WHERE session_id = ?
            ORDER BY timestamp ASC
            LIMIT ?
            "#,
        )
        .bind(session_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Convert rows to MemoryEntry structs
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let role: String = row
                .try_get("role")
                .map_err(|e| MemoryError::Database(e.to_string()))?;
            let content: String = row
                .try_get("content")
                .map_err(|e| MemoryError::Database(e.to_string()))?;
            let timestamp_str: String = row
                .try_get("timestamp")
                .map_err(|e| MemoryError::Database(e.to_string()))?;

            let role = Self::parse_role(&role)?;
            let timestamp = OffsetDateTime::parse(
                &timestamp_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;

            result.push(MemoryEntry {
                role,
                content,
                timestamp,
            });
        }

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

        sqlx::query(
            r#"
            INSERT INTO memory_entries (session_id, role, content, timestamp)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(session_str)
        .bind(role_str)
        .bind(entry.content)
        .bind(timestamp_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn query_history(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        // First, build the complete SQL query string
        let mut sql = String::from(
            "SELECT role, content, timestamp FROM memory_entries WHERE session_id = ?",
        );

        let mut has_start_time = false;
        if query.start_time.is_some() {
            sql.push_str(" AND timestamp >= ?");
            has_start_time = true;
        }

        let mut has_end_time = false;
        if query.end_time.is_some() {
            sql.push_str(" AND timestamp <= ?");
            has_end_time = true;
        }

        let mut has_role = false;
        if query.role.is_some() {
            sql.push_str(" AND role = ?");
            has_role = true;
        }

        let mut has_substring = false;
        if query.content_substring.is_some() {
            sql.push_str(" AND content LIKE ?");
            has_substring = true;
        }

        sql.push_str(" ORDER BY timestamp ASC");

        let mut has_limit = false;
        if query.limit.is_some() {
            sql.push_str(" LIMIT ?");
            has_limit = true;
        }

        let mut has_offset = false;
        if query.offset.is_some() {
            sql.push_str(" OFFSET ?");
            has_offset = true;
        }

        // Now create the query and bind parameters
        let mut query_builder = sqlx::query(&sql);

        // Bind session id (always present)
        query_builder = query_builder.bind(query.session.to_string());

        // Bind optional parameters
        if has_start_time {
            let start_time_str = query
                .start_time
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;
            query_builder = query_builder.bind(start_time_str);
        }

        if has_end_time {
            let end_time_str = query
                .end_time
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;
            query_builder = query_builder.bind(end_time_str);
        }

        if has_role {
            let role_str = Self::role_to_string(query.role.unwrap()).to_string();
            query_builder = query_builder.bind(role_str);
        }

        if has_substring {
            let like_pattern = format!("%{}%", query.content_substring.as_ref().unwrap());
            query_builder = query_builder.bind(like_pattern);
        }

        if has_limit {
            query_builder = query_builder.bind(query.limit.unwrap() as i64);
        }

        if has_offset {
            query_builder = query_builder.bind(query.offset.unwrap() as i64);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let role: String = row
                .try_get("role")
                .map_err(|e| MemoryError::Database(e.to_string()))?;
            let content: String = row
                .try_get("content")
                .map_err(|e| MemoryError::Database(e.to_string()))?;
            let timestamp_str: String = row
                .try_get("timestamp")
                .map_err(|e| MemoryError::Database(e.to_string()))?;

            let role = Self::parse_role(&role)?;
            let timestamp = OffsetDateTime::parse(
                &timestamp_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| MemoryError::InvalidFormat(e.to_string()))?;

            result.push(MemoryEntry {
                role,
                content,
                timestamp,
            });
        }

        Ok(result)
    }
}
