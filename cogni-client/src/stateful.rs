//! Stateful client for managing conversation history

use crate::Client;
use cogni_core::{Error, Message, Provider, Request, Response};
use cogni_state::{ConversationState, StateStore};
use std::sync::Arc;
use tracing::{debug, trace};
use uuid::Uuid;

/// A client that maintains conversation state across interactions
///
/// This wrapper around the base `Client` automatically manages conversation
/// history, saving and loading state through a `StateStore`.
pub struct StatefulClient<P: Provider> {
    /// The underlying client
    client: Client<P>,
    /// The state store for persistence
    store: Arc<dyn StateStore>,
    /// Current conversation state (if loaded)
    current_state: Option<ConversationState>,
    /// Whether to auto-save after each interaction
    auto_save: bool,
}

impl<P: Provider> StatefulClient<P> {
    /// Create a new stateful client
    pub fn new(client: Client<P>, store: Arc<dyn StateStore>) -> Self {
        Self {
            client,
            store,
            current_state: None,
            auto_save: true,
        }
    }

    /// Set whether to automatically save state after each interaction
    pub fn set_auto_save(&mut self, auto_save: bool) {
        self.auto_save = auto_save;
    }

    /// Create a new conversation
    pub async fn new_conversation(&mut self) -> Result<Uuid, Error> {
        let state = ConversationState::new();
        let id = state.id;
        self.current_state = Some(state);
        if self.auto_save {
            self.save().await?;
        }
        debug!("Created new conversation: {}", id);
        Ok(id)
    }

    /// Load an existing conversation
    pub async fn load_conversation(&mut self, id: Uuid) -> Result<(), Error> {
        trace!("Loading conversation: {}", id);
        let state = self
            .store
            .load(&id)
            .await
            .map_err(|e| Error::Storage(format!("Failed to load conversation: {}", e)))?;
        self.current_state = Some(state);
        debug!("Loaded conversation: {}", id);
        Ok(())
    }

    /// Save the current conversation
    pub async fn save(&self) -> Result<(), Error> {
        if let Some(ref state) = self.current_state {
            trace!("Saving conversation: {}", state.id);
            self.store
                .save(state)
                .await
                .map_err(|e| Error::Storage(format!("Failed to save conversation: {}", e)))?;
            debug!("Saved conversation: {}", state.id);
        }
        Ok(())
    }

    /// Get the current conversation ID
    pub fn current_conversation_id(&self) -> Option<Uuid> {
        self.current_state.as_ref().map(|s| s.id)
    }

    /// Get the current conversation state
    pub fn current_state(&self) -> Option<&ConversationState> {
        self.current_state.as_ref()
    }

    /// Get a mutable reference to the current conversation state
    pub fn current_state_mut(&mut self) -> Option<&mut ConversationState> {
        self.current_state.as_mut()
    }

    /// Clear the current conversation (doesn't delete from store)
    pub fn clear_current(&mut self) {
        self.current_state = None;
    }

    /// Send a chat message and update the conversation state
    pub async fn chat(&mut self, message: &str) -> Result<Response, Error> {
        // Ensure we have a conversation
        if self.current_state.is_none() {
            self.new_conversation().await?;
        }

        let state = self.current_state.as_mut().unwrap();

        // Add user message to state
        let user_message = Message::user(message);
        state.add_message(user_message.clone());

        // Build request with full conversation history
        let request = Request {
            messages: state.messages.clone(),
            model: self.client.default_model.clone(),
            parameters: self.client.default_parameters.clone(),
            tools: vec![],
            response_format: None,
        };

        // Send request
        let response = self.client.provider.request(request).await?;

        // Add assistant response to state
        if !response.content.is_empty() {
            state.add_message(Message::assistant(&response.content));
        }

        // Update token count if available
        if let Some(usage) = &response.metadata.usage {
            let current_count = state.metadata.token_count.unwrap_or(0);
            state.update_token_count(current_count + usage.total_tokens);
        }

        // Auto-save if enabled
        if self.auto_save {
            self.save().await?;
        }

        Ok(response)
    }

    /// Stream a chat message and update the conversation state
    pub async fn stream_chat(
        &mut self,
        message: &str,
    ) -> Result<impl futures::Stream<Item = Result<cogni_core::StreamEvent, Error>>, Error> {
        // Ensure we have a conversation
        if self.current_state.is_none() {
            self.new_conversation().await?;
        }

        let state = self.current_state.as_mut().unwrap();

        // Add user message to state
        let user_message = Message::user(message);
        state.add_message(user_message.clone());

        // Build request with full conversation history
        let request = Request {
            messages: state.messages.clone(),
            model: self.client.default_model.clone(),
            parameters: self.client.default_parameters.clone(),
            tools: vec![],
            response_format: None,
        };

        // Note: We'll need to accumulate the response and save it after streaming completes
        // This is a limitation of the streaming API - we can't update state during streaming
        let stream = self.client.provider.stream(request).await?;

        Ok(stream)
    }

    /// List all conversations in the store
    pub async fn list_conversations(&self) -> Result<Vec<ConversationState>, Error> {
        self.store
            .list()
            .await
            .map_err(|e| Error::Storage(format!("Failed to list conversations: {}", e)))
    }

    /// Delete a conversation from the store
    pub async fn delete_conversation(&mut self, id: &Uuid) -> Result<(), Error> {
        self.store
            .delete(id)
            .await
            .map_err(|e| Error::Storage(format!("Failed to delete conversation: {}", e)))?;

        // Clear current if it's the one being deleted
        if self.current_conversation_id() == Some(*id) {
            self.clear_current();
        }

        Ok(())
    }

    /// Find conversations by tags
    pub async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<ConversationState>, Error> {
        self.store
            .find_by_tags(tags)
            .await
            .map_err(|e| Error::Storage(format!("Failed to find conversations: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{ResponseMetadata, Usage};
    use cogni_state::MemoryStore;
    use futures::stream;
    use std::pin::Pin;

    // Mock provider for testing
    struct MockProvider {
        responses: std::sync::Mutex<Vec<Response>>,
        current_index: std::sync::Mutex<usize>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                responses: std::sync::Mutex::new(vec![]),
                current_index: std::sync::Mutex::new(0),
            }
        }

        fn with_response(self, response: Response) -> Self {
            self.responses.lock().unwrap().push(response);
            self
        }
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        type Stream =
            Pin<Box<dyn futures::Stream<Item = Result<cogni_core::StreamEvent, Error>> + Send>>;

        async fn request(&self, _request: Request) -> Result<Response, Error> {
            let mut index = self.current_index.lock().unwrap();
            let responses = self.responses.lock().unwrap();

            if *index < responses.len() {
                let response = responses[*index].clone();
                *index += 1;
                Ok(response)
            } else {
                Ok(Response {
                    content: "Default response".to_string(),
                    tool_calls: vec![],
                    metadata: ResponseMetadata::default(),
                })
            }
        }

        async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
            Ok(Box::pin(stream::empty()))
        }
    }

    #[tokio::test]
    async fn test_stateful_client_basic() {
        let provider = MockProvider::new().with_response(Response {
            content: "Hello! How can I help you?".to_string(),
            tool_calls: vec![],
            metadata: ResponseMetadata {
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 8,
                    total_tokens: 18,
                }),
                ..Default::default()
            },
        });

        let client = Client::new(provider);
        let store = Arc::new(MemoryStore::new());
        let mut stateful = StatefulClient::new(client, store.clone());

        // Create new conversation
        let id = stateful.new_conversation().await.unwrap();
        assert_eq!(stateful.current_conversation_id(), Some(id));

        // Send a message
        let response = stateful.chat("Hello!").await.unwrap();
        assert_eq!(response.content, "Hello! How can I help you?");

        // Check state was updated
        let state = stateful.current_state().unwrap();
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].content.as_text(), Some("Hello!"));
        assert_eq!(
            state.messages[1].content.as_text(),
            Some("Hello! How can I help you?")
        );
        assert_eq!(state.metadata.token_count, Some(18));

        // Verify it was saved
        let loaded = store.load(&id).await.unwrap();
        assert_eq!(loaded.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_conversation_management() {
        let provider = MockProvider::new();
        let client = Client::new(provider);
        let store = Arc::new(MemoryStore::new());
        let mut stateful = StatefulClient::new(client, store.clone());

        // Create multiple conversations
        let id1 = stateful.new_conversation().await.unwrap();
        stateful
            .current_state_mut()
            .unwrap()
            .set_title("First conversation");
        stateful.current_state_mut().unwrap().add_tag("test");
        stateful.save().await.unwrap();

        let id2 = stateful.new_conversation().await.unwrap();
        stateful
            .current_state_mut()
            .unwrap()
            .set_title("Second conversation");
        stateful.current_state_mut().unwrap().add_tag("example");
        stateful.save().await.unwrap();

        // List conversations
        let conversations = stateful.list_conversations().await.unwrap();
        assert_eq!(conversations.len(), 2);

        // Find by tags
        let test_convos = stateful.find_by_tags(&["test".to_string()]).await.unwrap();
        assert_eq!(test_convos.len(), 1);
        assert_eq!(test_convos[0].id, id1);

        // Load first conversation
        stateful.load_conversation(id1).await.unwrap();
        assert_eq!(stateful.current_conversation_id(), Some(id1));

        // Delete second conversation
        stateful.delete_conversation(&id2).await.unwrap();
        let remaining = stateful.list_conversations().await.unwrap();
        assert_eq!(remaining.len(), 1);
    }
}
