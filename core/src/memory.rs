//! Memory interface for the Cogni framework.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use time::OffsetDateTime;

use crate::error::MemoryError;
use crate::traits::memory::MemoryQuery;

/// A unique identifier for a conversation session
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SessionId {
    /// Create a new session ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// The role of a participant in a conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Role {
    /// The user
    User,
    /// The assistant
    Assistant,
    /// The system
    System,
}

/// An entry in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The role of the participant
    pub role: Role,

    /// The content of the message
    pub content: String,

    /// When this entry was created
    pub timestamp: OffsetDateTime,
}

/// A trait representing a store for conversation memory
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Load the last n entries for a session
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError>;

    /// Save an entry for a session
    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError>;
}

/// A simple in-memory implementation of the MemoryStore trait.
#[derive(Debug, Default)]
pub struct InMemoryMemory {
    store: std::collections::HashMap<SessionId, Vec<MemoryEntry>>,
}

impl InMemoryMemory {
    /// Create a new in-memory memory store.
    pub fn new() -> Self {
        Self {
            store: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemory {
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        match self.store.get(session) {
            Some(entries) => {
                let start = if entries.len() > n {
                    entries.len() - n
                } else {
                    0
                };
                Ok(entries[start..].to_vec())
            }
            None => Ok(Vec::new()),
        }
    }

    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError> {
        let mut store = self.store.clone();
        store.entry(session.clone()).or_default().push(entry);
        Ok(())
    }
}

#[async_trait]
impl crate::traits::memory::MemoryStore for InMemoryMemory {
    async fn load(
        &self,
        session: &crate::traits::memory::SessionId,
        n: usize,
    ) -> Result<Vec<crate::traits::memory::MemoryEntry>, MemoryError> {
        let session_id = SessionId(session.0.clone());
        match self.store.get(&session_id) {
            Some(entries) => {
                let start = if entries.len() > n {
                    entries.len() - n
                } else {
                    0
                };
                // Convert our MemoryEntry to traits::memory::MemoryEntry
                Ok(entries[start..]
                    .iter()
                    .map(|entry| crate::traits::memory::MemoryEntry {
                        role: match entry.role {
                            Role::User => crate::traits::memory::Role::User,
                            Role::Assistant => crate::traits::memory::Role::Assistant,
                            Role::System => crate::traits::memory::Role::System,
                        },
                        content: entry.content.clone(),
                        timestamp: entry.timestamp,
                    })
                    .collect())
            }
            None => Ok(Vec::new()),
        }
    }

    async fn save(
        &self,
        session: &crate::traits::memory::SessionId,
        entry: crate::traits::memory::MemoryEntry,
    ) -> Result<(), MemoryError> {
        let session_id = SessionId(session.0.clone());
        // Convert traits::memory::MemoryEntry to our MemoryEntry
        let memory_entry = MemoryEntry {
            role: match entry.role {
                crate::traits::memory::Role::User => Role::User,
                crate::traits::memory::Role::Assistant => Role::Assistant,
                crate::traits::memory::Role::System => Role::System,
            },
            content: entry.content,
            timestamp: entry.timestamp,
        };

        let mut store = self.store.clone();
        store.entry(session_id).or_default().push(memory_entry);
        Ok(())
    }

    async fn query_history(
        &self,
        query: MemoryQuery,
    ) -> Result<Vec<crate::traits::memory::MemoryEntry>, MemoryError> {
        let session_id = SessionId(query.session.0.clone());

        if let Some(entries) = self.store.get(&session_id) {
            let mut result: Vec<_> = entries
                .iter()
                .filter(|entry| {
                    // Filter by start time
                    if let Some(start_time) = query.start_time {
                        if entry.timestamp < start_time {
                            return false;
                        }
                    }

                    // Filter by end time
                    if let Some(end_time) = query.end_time {
                        if entry.timestamp > end_time {
                            return false;
                        }
                    }

                    // Filter by role
                    if let Some(role) = query.role {
                        let entry_role = match entry.role {
                            Role::User => crate::traits::memory::Role::User,
                            Role::Assistant => crate::traits::memory::Role::Assistant,
                            Role::System => crate::traits::memory::Role::System,
                        };
                        if entry_role != role {
                            return false;
                        }
                    }

                    // Filter by content substring
                    if let Some(ref substring) = query.content_substring {
                        if !entry.content.contains(substring) {
                            return false;
                        }
                    }

                    true
                })
                .map(|entry| crate::traits::memory::MemoryEntry {
                    role: match entry.role {
                        Role::User => crate::traits::memory::Role::User,
                        Role::Assistant => crate::traits::memory::Role::Assistant,
                        Role::System => crate::traits::memory::Role::System,
                    },
                    content: entry.content.clone(),
                    timestamp: entry.timestamp,
                })
                .collect();

            // Sort by timestamp
            result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            // Apply pagination
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(result.len());

            if offset >= result.len() {
                return Ok(Vec::new());
            }

            let end = std::cmp::min(offset + limit, result.len());
            Ok(result[offset..end].to_vec())
        } else {
            Ok(Vec::new())
        }
    }
}
