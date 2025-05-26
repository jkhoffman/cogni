//! File-based state storage implementation

use crate::{ConversationState, StateError, StateResult, StateStore};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

/// File-based state store implementation
/// 
/// This store persists conversation states as JSON files in a directory.
/// Each conversation is stored as a separate file named by its UUID.
#[derive(Debug, Clone)]
pub struct FileStore {
    base_path: PathBuf,
}

impl FileStore {
    /// Create a new file store at the given path
    /// 
    /// The directory will be created if it doesn't exist.
    pub fn new(base_path: impl AsRef<Path>) -> StateResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&base_path)
            .map_err(|e| StateError::Configuration(format!("Failed to create directory: {}", e)))?;
            
        debug!("Initialized file store at: {:?}", base_path);
        Ok(Self { base_path })
    }

    /// Get the file path for a conversation
    fn get_file_path(&self, id: &Uuid) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }

    /// List all conversation files
    async fn list_files(&self) -> StateResult<Vec<PathBuf>> {
        let mut entries = fs::read_dir(&self.base_path).await?;
        let mut files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                files.push(path);
            }
        }
        
        Ok(files)
    }
}

#[async_trait]
impl StateStore for FileStore {
    async fn save(&self, state: &ConversationState) -> StateResult<()> {
        let path = self.get_file_path(&state.id);
        trace!("Saving conversation {} to file: {:?}", state.id, path);
        
        // Serialize to pretty JSON
        let json = serde_json::to_string_pretty(state)?;
        
        // Write atomically by writing to temp file first
        let temp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;
        drop(file);
        
        // Rename temp file to final name (atomic on most filesystems)
        fs::rename(&temp_path, &path).await?;
        
        debug!("Saved conversation {} to file: {:?}", state.id, path);
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> StateResult<ConversationState> {
        let path = self.get_file_path(id);
        trace!("Loading conversation {} from file: {:?}", id, path);
        
        match fs::read_to_string(&path).await {
            Ok(json) => {
                let state: ConversationState = serde_json::from_str(&json)?;
                if state.id != *id {
                    error!("ID mismatch in file {:?}: expected {}, got {}", path, id, state.id);
                    return Err(StateError::InvalidState(format!(
                        "ID mismatch: file contains different conversation"
                    )));
                }
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(StateError::NotFound(*id))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn delete(&self, id: &Uuid) -> StateResult<()> {
        let path = self.get_file_path(id);
        trace!("Deleting conversation {} from file: {:?}", id, path);
        
        match fs::remove_file(&path).await {
            Ok(()) => {
                debug!("Deleted conversation {} from file: {:?}", id, path);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(StateError::NotFound(*id))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn list(&self) -> StateResult<Vec<ConversationState>> {
        trace!("Listing all conversations from file store");
        let files = self.list_files().await?;
        let mut states = Vec::new();
        let mut errors = 0;
        
        for path in files {
            match fs::read_to_string(&path).await {
                Ok(json) => match serde_json::from_str::<ConversationState>(&json) {
                    Ok(state) => states.push(state),
                    Err(e) => {
                        warn!("Failed to parse conversation file {:?}: {}", path, e);
                        errors += 1;
                    }
                },
                Err(e) => {
                    warn!("Failed to read conversation file {:?}: {}", path, e);
                    errors += 1;
                }
            }
        }
        
        if errors > 0 {
            warn!("Encountered {} errors while listing conversations", errors);
        }
        
        // Sort by updated_at descending (most recent first)
        states.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        debug!("Listed {} conversations from file store", states.len());
        Ok(states)
    }

    async fn exists(&self, id: &Uuid) -> StateResult<bool> {
        let path = self.get_file_path(id);
        Ok(path.exists())
    }

    async fn list_ids(&self) -> StateResult<Vec<Uuid>> {
        trace!("Listing conversation IDs from file store");
        let files = self.list_files().await?;
        let mut ids = Vec::new();
        
        for path in files {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = Uuid::parse_str(stem) {
                    ids.push(id);
                }
            }
        }
        
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConversationState;
    use cogni_core::{Content, Message, Metadata, Role};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_store_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).unwrap();
        
        let mut state = ConversationState::new();
        state.set_title("Test");
        state.add_message(Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            metadata: Metadata::default(),
        });

        // Save
        store.save(&state).await.unwrap();
        
        // File should exist
        let file_path = store.get_file_path(&state.id);
        assert!(file_path.exists());
        
        // Load
        let loaded = store.load(&state.id).await.unwrap();
        assert_eq!(loaded.metadata.title, Some("Test".to_string()));
        assert_eq!(loaded.messages.len(), 1);
        
        // Update
        state.add_tag("updated");
        store.save(&state).await.unwrap();
        let updated = store.load(&state.id).await.unwrap();
        assert!(updated.metadata.tags.contains(&"updated".to_string()));
        
        // List
        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, state.id);
        
        // Delete
        store.delete(&state.id).await.unwrap();
        assert!(!file_path.exists());
        assert!(matches!(
            store.load(&state.id).await,
            Err(StateError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_file_store_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).unwrap();
        let mut handles = vec![];

        // Spawn multiple tasks that save states concurrently
        for i in 0..5 {
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
        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 5);
        
        for id in ids {
            assert!(store.exists(&id).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_file_store_invalid_files() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).unwrap();
        
        // Create a valid state
        let state = ConversationState::new();
        store.save(&state).await.unwrap();
        
        // Create an invalid JSON file
        let invalid_path = temp_dir.path().join("invalid.json");
        fs::write(&invalid_path, "{ invalid json").await.unwrap();
        
        // Create a non-JSON file
        let non_json_path = temp_dir.path().join("not-json.txt");
        fs::write(&non_json_path, "not json").await.unwrap();
        
        // List should only return the valid state
        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, state.id);
    }
}