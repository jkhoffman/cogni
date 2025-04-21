use crate::error::AgentError;
/// Memory bridge module for the agent.
/// Provides integration between agent and memory store.
use crate::traits::memory::{MemoryEntry, MemoryStore, SessionId};

pub struct MemoryBridge;

impl MemoryBridge {
    /// Load the last n memory entries for a session.
    pub async fn load(
        memory: &dyn MemoryStore,
        session_id: &SessionId,
        n: usize,
    ) -> Result<Vec<MemoryEntry>, AgentError> {
        memory.load(session_id, n).await.map_err(AgentError::Memory)
    }

    /// Save a new memory entry for a session.
    pub async fn save(
        memory: &dyn MemoryStore,
        session_id: &SessionId,
        entry: MemoryEntry,
    ) -> Result<(), AgentError> {
        memory
            .save(session_id, entry)
            .await
            .map_err(AgentError::Memory)
    }
}
