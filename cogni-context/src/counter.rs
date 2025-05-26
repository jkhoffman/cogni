use async_trait::async_trait;
use cogni_core::Message;
use std::sync::Arc;
use tiktoken_rs::CoreBPE;

use crate::error::ContextError;
use crate::types::ModelLimits;

#[async_trait]
pub trait TokenCounter: Send + Sync {
    fn count_text(&self, text: &str) -> usize;

    fn count_message(&self, message: &Message) -> usize {
        let mut tokens = 0;

        // Count role tokens (typically 1-2 tokens)
        tokens += self.count_text(&format!("{:?}", message.role));

        // Count content tokens
        if let Some(text) = message.content.as_text() {
            tokens += self.count_text(text);
        }

        // Count name tokens if present
        if let Some(name) = &message.metadata.name {
            tokens += self.count_text(name);
        }

        // Add overhead for message structure (typically 3-4 tokens)
        tokens + 4
    }

    fn count_messages(&self, messages: &[Message]) -> usize {
        messages.iter().map(|msg| self.count_message(msg)).sum()
    }

    fn model_context_window(&self) -> usize;
}

pub struct TiktokenCounter {
    encoder: Arc<CoreBPE>,
    model_limits: ModelLimits,
}

impl TiktokenCounter {
    pub fn for_model(model: &str) -> Result<Self, ContextError> {
        let encoder = tiktoken_rs::get_bpe_from_model(model)
            .map_err(|e| ContextError::UnsupportedModel(format!("{}: {}", model, e)))?;

        let model_limits = ModelLimits::for_model(model)
            .ok_or_else(|| ContextError::UnsupportedModel(model.to_string()))?;

        Ok(Self {
            encoder: Arc::new(encoder),
            model_limits,
        })
    }

    pub fn with_encoder(encoder: CoreBPE, model_limits: ModelLimits) -> Self {
        Self {
            encoder: Arc::new(encoder),
            model_limits,
        }
    }
}

impl TokenCounter for TiktokenCounter {
    fn count_text(&self, text: &str) -> usize {
        self.encoder.encode_ordinary(text).len()
    }

    fn model_context_window(&self) -> usize {
        self.model_limits.context_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::Message;

    #[test]
    fn test_token_counting() {
        // This would need a mock or actual encoder for testing
        // For now, we'll create a simple mock implementation
        struct MockCounter;

        impl TokenCounter for MockCounter {
            fn count_text(&self, text: &str) -> usize {
                // Simple approximation: ~1 token per 4 chars
                text.len() / 4
            }

            fn model_context_window(&self) -> usize {
                4096
            }
        }

        let counter = MockCounter;
        let message = Message::user("Hello, world!");

        let count = counter.count_message(&message);
        assert!(count > 0);

        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
            Message::assistant("Hi there!"),
        ];

        let total_count = counter.count_messages(&messages);
        assert!(total_count > count);
    }
}
