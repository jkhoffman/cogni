//! Memory interface for the Cogni framework.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use time::OffsetDateTime;

use crate::error::MemoryError;

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
