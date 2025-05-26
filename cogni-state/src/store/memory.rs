//! In-memory state storage implementation

use crate::{ConversationState, StateError, StateResult, StateStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};
use uuid::Uuid;

/// In-memory state store implementation
///
/// This store keeps all conversation states in memory and is suitable for
/// development, testing, or short-lived applications. Data is lost when
/// the application stops.
#[derive(Debug, Clone)]
pub struct MemoryStore {
    states: Arc<RwLock<HashMap<Uuid, ConversationState>>>,
}

impl MemoryStore {
    /// Create a new empty memory store
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a memory store with initial states
    pub fn with_states(states: Vec<ConversationState>) -> Self {
        let map = states.into_iter().map(|s| (s.id, s)).collect();
        Self {
            states: Arc::new(RwLock::new(map)),
        }
    }

    /// Get the number of stored conversations
    pub async fn len(&self) -> usize {
        self.states.read().await.len()
    }

    /// Check if the store is empty
    pub async fn is_empty(&self) -> bool {
        self.states.read().await.is_empty()
    }

    /// Clear all stored conversations
    pub async fn clear(&self) {
        self.states.write().await.clear();
        debug!("Cleared all conversations from memory store");
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStore for MemoryStore {
    async fn save(&self, state: &ConversationState) -> StateResult<()> {
        trace!("Saving conversation {} to memory store", state.id);
        let mut states = self.states.write().await;
        states.insert(state.id, state.clone());
        debug!("Saved conversation {} to memory store", state.id);
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> StateResult<ConversationState> {
        trace!("Loading conversation {} from memory store", id);
        let states = self.states.read().await;
        states.get(id).cloned().ok_or(StateError::NotFound(*id))
    }

    async fn delete(&self, id: &Uuid) -> StateResult<()> {
        trace!("Deleting conversation {} from memory store", id);
        let mut states = self.states.write().await;
        if states.remove(id).is_some() {
            debug!("Deleted conversation {} from memory store", id);
            Ok(())
        } else {
            Err(StateError::NotFound(*id))
        }
    }

    async fn list(&self) -> StateResult<Vec<ConversationState>> {
        trace!("Listing all conversations from memory store");
        let states = self.states.read().await;
        let mut conversations: Vec<_> = states.values().cloned().collect();
        // Sort by updated_at descending (most recent first)
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug!(
            "Listed {} conversations from memory store",
            conversations.len()
        );
        Ok(conversations)
    }

    async fn exists(&self, id: &Uuid) -> StateResult<bool> {
        trace!("Checking if conversation {} exists in memory store", id);
        Ok(self.states.read().await.contains_key(id))
    }

    async fn list_ids(&self) -> StateResult<Vec<Uuid>> {
        trace!("Listing conversation IDs from memory store");
        let states = self.states.read().await;
        Ok(states.keys().copied().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConversationState;
    use cogni_core::{Content, Message, Metadata, Role};

    #[tokio::test]
    async fn test_memory_store_operations() {
        let store = MemoryStore::new();
        assert!(store.is_empty().await);

        let mut state = ConversationState::new();
        state.set_title("Test");
        state.add_message(Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            metadata: Metadata::default(),
        });

        // Save
        store.save(&state).await.unwrap();
        assert_eq!(store.len().await, 1);

        // Load
        let loaded = store.load(&state.id).await.unwrap();
        assert_eq!(loaded.metadata.title, Some("Test".to_string()));

        // Exists
        assert!(store.exists(&state.id).await.unwrap());

        // Update
        state.add_tag("updated");
        store.save(&state).await.unwrap();
        let updated = store.load(&state.id).await.unwrap();
        assert!(updated.metadata.tags.contains(&"updated".to_string()));

        // Delete
        store.delete(&state.id).await.unwrap();
        assert!(!store.exists(&state.id).await.unwrap());
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn test_memory_store_concurrent_access() {
        let store = MemoryStore::new();
        let mut handles = vec![];

        // Spawn multiple tasks that save states concurrently
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let mut state = ConversationState::new();
                state.set_title(format!("Conversation {}", i));
                store_clone.save(&state).await.unwrap();
                state.id
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let ids: Vec<Uuid> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Verify all states were saved
        assert_eq!(store.len().await, 10);
        for id in ids {
            assert!(store.exists(&id).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_memory_store_list_ordering() {
        let store = MemoryStore::new();

        // Create states with small delays to ensure different timestamps
        let mut state1 = ConversationState::new();
        state1.set_title("First");
        store.save(&state1).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut state2 = ConversationState::new();
        state2.set_title("Second");
        store.save(&state2).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update the first state to make it most recent
        state1.add_tag("updated");
        store.save(&state1).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, state1.id); // Most recently updated
        assert_eq!(list[1].id, state2.id);
    }
}
