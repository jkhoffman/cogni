//! State persistence for Cogni agents
//!
//! This crate provides conversation state management capabilities for building
//! stateful AI agents with memory across sessions.

pub mod error;
pub mod store;
pub mod types;

pub use error::{StateError, StateResult};
pub use store::{FileStore, MemoryStore, StateStore};
pub use types::{ConversationState, StateMetadata};

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{Content, Message, Metadata, Role};

    #[tokio::test]
    async fn test_conversation_state_creation() {
        let state = ConversationState::new();
        assert!(state.messages.is_empty());
        assert!(state.metadata.title.is_none());
        assert!(state.metadata.tags.is_empty());
    }

    #[tokio::test]
    async fn test_add_message() {
        let mut state = ConversationState::new();
        let message = Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            metadata: Metadata::default(),
        };

        state.add_message(message.clone());
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, message.content);
    }
}
