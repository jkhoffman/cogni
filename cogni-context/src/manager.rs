use crate::counter::TokenCounter;
use crate::error::ContextError;
use crate::strategies::PruningStrategy;
use cogni_core::Message;
use std::sync::Arc;

pub struct ContextManager {
    counter: Arc<dyn TokenCounter>,
    max_tokens: usize,
    reserve_output_tokens: usize,
    pruning_strategy: Arc<dyn PruningStrategy>,
}

impl ContextManager {
    pub fn new(counter: Arc<dyn TokenCounter>) -> Self {
        let max_tokens = counter.model_context_window();
        Self {
            counter: Arc::clone(&counter),
            max_tokens,
            reserve_output_tokens: 1000, // Default reservation
            pruning_strategy: Arc::new(crate::strategies::SlidingWindowStrategy::default()),
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_reserve_output_tokens(mut self, tokens: usize) -> Self {
        self.reserve_output_tokens = tokens;
        self
    }

    pub fn with_strategy(mut self, strategy: Arc<dyn PruningStrategy>) -> Self {
        self.pruning_strategy = strategy;
        self
    }

    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserve_output_tokens)
    }

    pub async fn fit_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, ContextError> {
        let total_tokens = self.counter.count_messages(&messages);
        let available = self.available_tokens();

        if total_tokens <= available {
            return Ok(messages);
        }

        tracing::debug!(
            "Messages exceed context window: {} > {}. Pruning...",
            total_tokens,
            available
        );

        self.pruning_strategy
            .prune(messages, available, &*self.counter)
            .await
    }

    pub fn count_messages(&self, messages: &[Message]) -> usize {
        self.counter.count_messages(messages)
    }

    pub fn would_fit(&self, messages: &[Message]) -> bool {
        self.count_messages(messages) <= self.available_tokens()
    }

    pub fn tokens_remaining(&self, messages: &[Message]) -> Option<usize> {
        let used = self.count_messages(messages);
        let available = self.available_tokens();

        if used <= available {
            Some(available - used)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::SlidingWindowStrategy;
    use cogni_core::Message;

    struct MockCounter {
        tokens_per_message: usize,
    }

    impl TokenCounter for MockCounter {
        fn count_text(&self, text: &str) -> usize {
            text.len() / 4
        }

        fn count_message(&self, _message: &Message) -> usize {
            self.tokens_per_message
        }

        fn count_messages(&self, messages: &[Message]) -> usize {
            messages.len() * self.tokens_per_message
        }

        fn model_context_window(&self) -> usize {
            1000
        }
    }

    #[tokio::test]
    async fn test_context_manager_no_pruning() {
        let counter = Arc::new(MockCounter {
            tokens_per_message: 10,
        });
        let manager = ContextManager::new(counter).with_reserve_output_tokens(100); // Reserve less tokens for the test

        let messages = vec![
            Message::system("System prompt"),
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let result = manager.fit_messages(messages.clone()).await.unwrap();
        assert_eq!(result.len(), messages.len());
    }

    #[tokio::test]
    async fn test_context_manager_with_pruning() {
        let counter = Arc::new(MockCounter {
            tokens_per_message: 100,
        });
        let strategy = Arc::new(SlidingWindowStrategy::new(true, 5));
        let manager = ContextManager::new(counter)
            .with_reserve_output_tokens(200)
            .with_strategy(strategy);

        let messages = vec![
            Message::system("System prompt"),
            Message::user("Message 1"),
            Message::assistant("Response 1"),
            Message::user("Message 2"),
            Message::assistant("Response 2"),
            Message::user("Message 3"),
            Message::assistant("Response 3"),
            Message::user("Message 4"),
            Message::assistant("Response 4"),
            Message::user("Message 5"),
        ];

        let result = manager.fit_messages(messages).await.unwrap();
        assert!(result.len() < 10);
        assert!(manager.would_fit(&result));
    }
}
