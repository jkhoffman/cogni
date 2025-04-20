//! Memory interface for the Cogni framework.
//!
//! This module defines the core traits and types for storing and retrieving
//! conversation history in a consistent way across different storage backends.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use time::OffsetDateTime;

use crate::error::MemoryError;

/// A unique identifier for a conversation session.
///
/// Each conversation is identified by a unique session ID, which is used
/// to store and retrieve the conversation history.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SessionId {
    /// Create a new session ID from any type that can be converted into a String.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// The role of a participant in a conversation.
///
/// This enum represents the different roles that participants can have
/// in a conversation, such as the user asking questions, the assistant
/// providing responses, or the system providing context.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Role {
    /// The user asking questions or providing input
    User,
    /// The assistant providing responses
    Assistant,
    /// The system providing context or instructions
    System,
}

/// An entry in the conversation memory.
///
/// Each entry represents a single message in the conversation, including
/// who sent it (role), what was said (content), and when it was sent (timestamp).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The role of the participant who sent this message
    pub role: Role,

    /// The content of the message
    pub content: String,

    /// When this entry was created
    pub timestamp: OffsetDateTime,
}

/// A trait representing a store for conversation memory.
///
/// This trait defines the core interface for storing and retrieving
/// conversation history. Implementations of this trait can use different
/// storage backends (e.g., SQLite, Redis, PostgreSQL) while providing
/// a consistent interface for the rest of the framework.
///
/// # Examples
///
/// ```rust,no_run
/// use cogni_core::traits::memory::{MemoryStore, SessionId, MemoryEntry, Role};
/// use async_trait::async_trait;
/// use time::OffsetDateTime;
///
/// struct MyMemoryStore;
///
/// #[async_trait]
/// impl MemoryStore for MyMemoryStore {
///     async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, cogni_core::error::MemoryError> {
///         Ok(vec![MemoryEntry {
///             role: Role::User,
///             content: "Hello".into(),
///             timestamp: OffsetDateTime::now_utc(),
///         }])
///     }
///
///     async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), cogni_core::error::MemoryError> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Load the last n entries for a session.
    ///
    /// This method retrieves the most recent entries from the conversation
    /// history for the specified session. The number of entries returned
    /// is limited by the `n` parameter.
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError>;

    /// Save an entry for a session.
    ///
    /// This method adds a new entry to the conversation history for the
    /// specified session.
    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError>;
}
