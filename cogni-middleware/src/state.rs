//! State management middleware

use crate::{BoxFuture, Layer, Service};
use cogni_core::{Error, Request, Response};
use cogni_state::{ConversationState, StateStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, trace};
use uuid::Uuid;

/// Configuration for state middleware
#[derive(Debug, Clone)]
pub struct StateConfig {
    /// Whether to automatically save state after each request
    pub auto_save: bool,
    /// Whether to load conversation history into requests
    pub include_history: bool,
    /// Maximum number of messages to include in history
    pub max_history_messages: Option<usize>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            auto_save: true,
            include_history: true,
            max_history_messages: None,
        }
    }
}

/// State management middleware layer
#[derive(Clone)]
pub struct StateLayer {
    store: Arc<dyn StateStore>,
    config: StateConfig,
}

impl StateLayer {
    /// Create a new state layer with a store
    pub fn new(store: Arc<dyn StateStore>) -> Self {
        Self {
            store,
            config: StateConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(store: Arc<dyn StateStore>, config: StateConfig) -> Self {
        Self { store, config }
    }
}

impl<S> Layer<S> for StateLayer {
    type Service = StateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StateService {
            inner,
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            state_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// State management middleware service
pub struct StateService<S> {
    inner: S,
    store: Arc<dyn StateStore>,
    config: StateConfig,
    state_cache: Arc<Mutex<HashMap<Uuid, ConversationState>>>,
}

impl<S> StateService<S> {
    /// Get or create a conversation state for a request
    async fn get_or_create_state(&self, conversation_id: Option<Uuid>) -> Result<ConversationState, Error> {
        if let Some(id) = conversation_id {
            // Check cache first
            let cache = self.state_cache.lock().await;
            if let Some(state) = cache.get(&id) {
                return Ok(state.clone());
            }
            drop(cache);

            // Load from store
            match self.store.load(&id).await {
                Ok(state) => {
                    // Update cache
                    self.state_cache.lock().await.insert(id, state.clone());
                    Ok(state)
                }
                Err(_) => {
                    // Create new state with specified ID
                    let state = ConversationState::with_id(id);
                    self.state_cache.lock().await.insert(id, state.clone());
                    Ok(state)
                }
            }
        } else {
            // Create new state
            Ok(ConversationState::new())
        }
    }

    /// Extract conversation ID from request metadata
    fn extract_conversation_id(request: &Request) -> Option<Uuid> {
        // Look for conversation ID in the first message's metadata
        request.messages.first()
            .and_then(|msg| msg.metadata.custom.get("conversation_id"))
            .and_then(|id| Uuid::parse_str(id).ok())
    }

    /// Update state after response
    async fn update_state(
        &self,
        mut state: ConversationState,
        request: &Request,
        response: &Response,
    ) -> Result<(), Error> {
        // Add user message from request (last message)
        if let Some(user_msg) = request.messages.last() {
            if !state.messages.iter().any(|m| m == user_msg) {
                state.add_message(user_msg.clone());
            }
        }

        // Add assistant response
        if !response.content.is_empty() {
            state.add_message(cogni_core::Message::assistant(&response.content));
        }

        // Update token count
        if let Some(usage) = &response.metadata.usage {
            let current = state.metadata.token_count.unwrap_or(0);
            state.update_token_count(current + usage.total_tokens);
        }

        // Save to cache
        self.state_cache.lock().await.insert(state.id, state.clone());

        // Save to store if auto-save is enabled and we have a conversation ID
        // (don't save ephemeral conversations)
        if self.config.auto_save && request.messages.first()
            .and_then(|msg| msg.metadata.custom.get("conversation_id"))
            .is_some() {
            self.store.save(&state).await
                .map_err(|e| Error::Storage(format!("Failed to save state: {}", e)))?;
        }

        Ok(())
    }
}

impl<S> Service<Request> for StateService<S>
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn call(&mut self, mut request: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();
        let service = self.clone();

        Box::pin(async move {
            // Extract conversation ID
            let conversation_id = Self::extract_conversation_id(&request);
            trace!("Processing request with conversation_id: {:?}", conversation_id);

            // Get or create state
            let state = service.get_or_create_state(conversation_id).await?;

            // Store original messages before modifying request
            let original_messages = request.messages.clone();

            // Include conversation history if configured
            if config.include_history && !state.messages.is_empty() {
                let history_messages = if let Some(max) = config.max_history_messages {
                    state.messages.iter()
                        .rev()
                        .take(max)
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    state.messages.clone()
                };

                // Prepend history to request messages
                let mut all_messages = history_messages;
                all_messages.extend(request.messages.clone());
                request.messages = all_messages;
            }

            // Call inner service
            let response = inner.call(request.clone()).await?;

            // Create a request with only the original messages for state update
            let update_request = Request {
                messages: original_messages,
                model: request.model,
                parameters: request.parameters,
                tools: request.tools,
            };

            // Update state with response
            service.update_state(state, &update_request, &response).await?;

            debug!("State middleware processed request for conversation: {:?}", conversation_id);
            Ok(response)
        })
    }
}

// Clone implementation for StateService
impl<S: Clone> Clone for StateService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            state_cache: Arc::clone(&self.state_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::EchoService;
    use cogni_core::{Message, Role};
    use cogni_state::MemoryStore;

    #[tokio::test]
    async fn test_state_middleware_new_conversation() {
        let store = Arc::new(MemoryStore::new());
        let layer = StateLayer::new(store.clone());
        let mut service = layer.layer(EchoService);

        let request = Request {
            messages: vec![Message::user("Hello")],
            model: Default::default(),
            parameters: Default::default(),
            tools: vec![],
        };

        let response = service.call(request).await.unwrap();
        assert_eq!(response.content, "Echo: Hello");

        // Check that state was saved
        let states = store.list().await.unwrap();
        assert_eq!(states.len(), 0); // No conversation ID provided, so no save
    }

    #[tokio::test]
    async fn test_state_middleware_with_conversation_id() {
        let store = Arc::new(MemoryStore::new());
        let layer = StateLayer::new(store.clone());
        let mut service = layer.layer(EchoService);

        let conversation_id = Uuid::new_v4();
        let mut message = Message::user("Hello");
        message.metadata.custom.insert(
            "conversation_id".to_string(),
            conversation_id.to_string(),
        );

        let request = Request {
            messages: vec![message],
            model: Default::default(),
            parameters: Default::default(),
            tools: vec![],
        };

        let response = service.call(request).await.unwrap();
        assert_eq!(response.content, "Echo: Hello");

        // Check that state was saved
        let state = store.load(&conversation_id).await.unwrap();
        assert_eq!(state.messages.len(), 2); // User + Assistant
        assert_eq!(state.messages[0].role, Role::User);
        assert_eq!(state.messages[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_state_middleware_conversation_history() {
        let store = Arc::new(MemoryStore::new());
        let conversation_id = Uuid::new_v4();

        // Create initial conversation
        let mut state = ConversationState::with_id(conversation_id);
        state.add_message(Message::user("First message"));
        state.add_message(Message::assistant("First response"));
        store.save(&state).await.unwrap();

        let layer = StateLayer::new(store.clone());
        let mut service = layer.layer(EchoService);

        // Send new message with same conversation ID
        let mut message = Message::user("Second message");
        message.metadata.custom.insert(
            "conversation_id".to_string(),
            conversation_id.to_string(),
        );

        let request = Request {
            messages: vec![message],
            model: Default::default(),
            parameters: Default::default(),
            tools: vec![],
        };

        let _response = service.call(request).await.unwrap();

        // Check that history was included
        let updated_state = store.load(&conversation_id).await.unwrap();
        assert_eq!(updated_state.messages.len(), 4); // 2 original + 2 new
    }
}