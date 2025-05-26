//! State storage implementations

use crate::{ConversationState, StateError, StateResult};
use async_trait::async_trait;
use uuid::Uuid;

mod memory;
mod file;

pub use file::FileStore;
pub use memory::MemoryStore;

/// Trait for storing and retrieving conversation states
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save a conversation state
    async fn save(&self, state: &ConversationState) -> StateResult<()>;

    /// Load a conversation state by ID
    async fn load(&self, id: &Uuid) -> StateResult<ConversationState>;

    /// Delete a conversation state
    async fn delete(&self, id: &Uuid) -> StateResult<()>;

    /// List all conversation states
    async fn list(&self) -> StateResult<Vec<ConversationState>>;

    /// Find conversations by tags
    async fn find_by_tags(&self, tags: &[String]) -> StateResult<Vec<ConversationState>> {
        let all_states = self.list().await?;
        Ok(all_states
            .into_iter()
            .filter(|state| tags.iter().any(|tag| state.metadata.tags.contains(tag)))
            .collect())
    }

    /// Check if a conversation exists
    async fn exists(&self, id: &Uuid) -> StateResult<bool> {
        match self.load(id).await {
            Ok(_) => Ok(true),
            Err(StateError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Update a conversation if it exists, create it otherwise
    async fn upsert(&self, state: &ConversationState) -> StateResult<()> {
        self.save(state).await
    }

    /// List conversation IDs only (more efficient than full list)
    async fn list_ids(&self) -> StateResult<Vec<Uuid>> {
        let states = self.list().await?;
        Ok(states.into_iter().map(|s| s.id).collect())
    }

    /// Get metadata for multiple conversations
    async fn get_metadata(&self, ids: &[Uuid]) -> StateResult<Vec<(Uuid, crate::StateMetadata)>> {
        let mut results = Vec::new();
        for id in ids {
            match self.load(id).await {
                Ok(state) => results.push((state.id, state.metadata)),
                Err(StateError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConversationState;
    use cogni_core::{Content, Message, Metadata, Role};

    async fn create_test_state() -> ConversationState {
        let mut state = ConversationState::new();
        state.set_title("Test Conversation");
        state.add_tag("test");
        state.add_message(Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            metadata: Metadata::default(),
        });
        state
    }

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStore::new();
        let state = create_test_state().await;
        let id = state.id;

        // Save
        store.save(&state).await.unwrap();

        // Load
        let loaded = store.load(&id).await.unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.metadata.title, Some("Test Conversation".to_string()));

        // List
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        store.delete(&id).await.unwrap();
        assert!(matches!(
            store.load(&id).await,
            Err(StateError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_file_store_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(temp_dir.path()).unwrap();
        let state = create_test_state().await;
        let id = state.id;

        // Save
        store.save(&state).await.unwrap();

        // Load
        let loaded = store.load(&id).await.unwrap();
        assert_eq!(loaded.id, id);

        // File should exist
        let file_path = temp_dir.path().join(format!("{}.json", id));
        assert!(file_path.exists());

        // Delete
        store.delete(&id).await.unwrap();
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_find_by_tags() {
        let store = MemoryStore::new();

        let mut state1 = create_test_state().await;
        state1.add_tag("important");

        let mut state2 = create_test_state().await;
        state2.add_tag("archived");

        let state3 = create_test_state().await; // Only has "test" tag

        store.save(&state1).await.unwrap();
        store.save(&state2).await.unwrap();
        store.save(&state3).await.unwrap();

        let important = store.find_by_tags(&["important".to_string()]).await.unwrap();
        assert_eq!(important.len(), 1);
        assert_eq!(important[0].id, state1.id);

        let test_states = store.find_by_tags(&["test".to_string()]).await.unwrap();
        assert_eq!(test_states.len(), 3);
    }
}