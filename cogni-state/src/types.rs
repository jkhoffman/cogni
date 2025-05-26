//! Core types for conversation state management

use chrono::{DateTime, Utc};
use cogni_core::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents the complete state of a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// Unique identifier for this conversation
    pub id: Uuid,
    /// All messages in the conversation
    pub messages: Vec<Message>,
    /// Metadata about the conversation
    pub metadata: StateMetadata,
    /// When this conversation was created
    pub created_at: DateTime<Utc>,
    /// When this conversation was last updated
    pub updated_at: DateTime<Utc>,
}

/// Metadata associated with a conversation state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateMetadata {
    /// Optional title for the conversation
    pub title: Option<String>,
    /// Tags for categorizing conversations
    pub tags: Vec<String>,
    /// Agent-specific configuration
    pub agent_config: Option<serde_json::Value>,
    /// Total token count for the conversation
    pub token_count: Option<u32>,
    /// Custom key-value pairs
    pub custom: HashMap<String, String>,
}

impl ConversationState {
    /// Create a new conversation state with a random ID
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            messages: Vec::new(),
            metadata: StateMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a conversation state with a specific ID
    pub fn with_id(id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id,
            messages: Vec::new(),
            metadata: StateMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Add multiple messages to the conversation
    pub fn add_messages(&mut self, messages: impl IntoIterator<Item = Message>) {
        self.messages.extend(messages);
        self.updated_at = Utc::now();
    }

    /// Set the title of the conversation
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.metadata.title = Some(title.into());
        self.updated_at = Utc::now();
    }

    /// Add a tag to the conversation
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag from the conversation
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let original_len = self.metadata.tags.len();
        self.metadata.tags.retain(|t| t != tag);
        if self.metadata.tags.len() != original_len {
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update the token count
    pub fn update_token_count(&mut self, count: u32) {
        self.metadata.token_count = Some(count);
        self.updated_at = Utc::now();
    }

    /// Add or update a custom metadata field
    pub fn set_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.custom.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    /// Get a custom metadata field
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.metadata.custom.get(key).map(|s| s.as_str())
    }

    /// Clear all messages while preserving metadata
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// Get the age of the conversation
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Check if the conversation has been modified since a given time
    pub fn modified_since(&self, timestamp: DateTime<Utc>) -> bool {
        self.updated_at > timestamp
    }
}

impl Default for ConversationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Content, Metadata, Role};

    #[test]
    fn test_conversation_state_creation() {
        let state = ConversationState::new();
        assert_eq!(state.messages.len(), 0);
        assert!(state.metadata.title.is_none());
        assert_eq!(state.metadata.tags.len(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut state = ConversationState::new();
        let original_updated = state.updated_at;
        
        // Sleep briefly to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        state.add_message(Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            metadata: Metadata::default(),
        });
        
        assert_eq!(state.messages.len(), 1);
        assert!(state.updated_at > original_updated);
    }

    #[test]
    fn test_tags() {
        let mut state = ConversationState::new();
        
        state.add_tag("test");
        state.add_tag("conversation");
        state.add_tag("test"); // Duplicate, shouldn't be added
        
        assert_eq!(state.metadata.tags.len(), 2);
        assert!(state.metadata.tags.contains(&"test".to_string()));
        assert!(state.metadata.tags.contains(&"conversation".to_string()));
        
        assert!(state.remove_tag("test"));
        assert!(!state.remove_tag("nonexistent"));
        assert_eq!(state.metadata.tags.len(), 1);
    }

    #[test]
    fn test_custom_metadata() {
        let mut state = ConversationState::new();
        
        state.set_custom("user_id", "12345");
        state.set_custom("session_type", "support");
        
        assert_eq!(state.get_custom("user_id"), Some("12345"));
        assert_eq!(state.get_custom("session_type"), Some("support"));
        assert_eq!(state.get_custom("nonexistent"), None);
    }
}