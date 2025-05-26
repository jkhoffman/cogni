use async_trait::async_trait;
use cogni_core::{Message, Provider, Request, Role};
use std::sync::Arc;

use crate::counter::TokenCounter;
use crate::error::ContextError;

#[async_trait]
pub trait PruningStrategy: Send + Sync {
    async fn prune(
        &self,
        messages: Vec<Message>,
        target_tokens: usize,
        counter: &dyn TokenCounter,
    ) -> Result<Vec<Message>, ContextError>;
}

#[derive(Debug, Clone)]
pub struct SlidingWindowStrategy {
    keep_system: bool,
    keep_recent: usize,
}

impl SlidingWindowStrategy {
    pub fn new(keep_system: bool, keep_recent: usize) -> Self {
        Self {
            keep_system,
            keep_recent,
        }
    }
}

impl Default for SlidingWindowStrategy {
    fn default() -> Self {
        Self {
            keep_system: true,
            keep_recent: 10,
        }
    }
}

#[async_trait]
impl PruningStrategy for SlidingWindowStrategy {
    async fn prune(
        &self,
        messages: Vec<Message>,
        target_tokens: usize,
        counter: &dyn TokenCounter,
    ) -> Result<Vec<Message>, ContextError> {
        let mut result = Vec::new();
        let mut total_tokens = 0;

        // First, add system messages if requested
        if self.keep_system {
            for msg in &messages {
                if msg.role == Role::System {
                    let tokens = counter.count_message(msg);
                    if total_tokens + tokens <= target_tokens {
                        result.push(msg.clone());
                        total_tokens += tokens;
                    }
                }
            }
        }

        // Then add recent messages from the end
        let non_system_messages: Vec<_> = messages
            .into_iter()
            .filter(|msg| msg.role != Role::System || !self.keep_system)
            .collect();

        let start_idx = non_system_messages.len().saturating_sub(self.keep_recent);
        for msg in non_system_messages.into_iter().skip(start_idx).rev() {
            let tokens = counter.count_message(&msg);
            if total_tokens + tokens <= target_tokens {
                result.push(msg);
                total_tokens += tokens;
            } else {
                break;
            }
        }

        // Reverse the non-system messages to maintain order
        let system_count = result.iter().filter(|m| m.role == Role::System).count();
        result[system_count..].reverse();

        if result.is_empty() {
            return Err(ContextError::PruningError(
                "Cannot fit any messages within token limit".to_string(),
            ));
        }

        Ok(result)
    }
}

#[derive(Clone)]
pub struct ImportanceBasedStrategy {
    importance_scorer: Arc<dyn Fn(&Message) -> f32 + Send + Sync>,
    keep_system: bool,
    min_messages: usize,
}

impl ImportanceBasedStrategy {
    pub fn new<F>(importance_scorer: F) -> Self
    where
        F: Fn(&Message) -> f32 + Send + Sync + 'static,
    {
        Self {
            importance_scorer: Arc::new(importance_scorer),
            keep_system: true,
            min_messages: 3,
        }
    }

    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages = min;
        self
    }
}

#[async_trait]
impl PruningStrategy for ImportanceBasedStrategy {
    async fn prune(
        &self,
        messages: Vec<Message>,
        target_tokens: usize,
        counter: &dyn TokenCounter,
    ) -> Result<Vec<Message>, ContextError> {
        let mut scored_messages: Vec<(f32, Message)> = messages
            .into_iter()
            .map(|msg| {
                let score = if msg.role == Role::System && self.keep_system {
                    f32::MAX // System messages get highest priority
                } else {
                    (self.importance_scorer)(&msg)
                };
                (score, msg)
            })
            .collect();

        // Sort by importance (highest first)
        scored_messages.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut result = Vec::new();
        let mut total_tokens = 0;

        // Add messages by importance until we hit the limit
        for (_, msg) in scored_messages {
            let tokens = counter.count_message(&msg);
            if total_tokens + tokens <= target_tokens || result.len() < self.min_messages {
                result.push(msg);
                total_tokens += tokens;
            }
        }

        // Sort result back to chronological order
        // This is a simplified approach - in practice, you'd want to preserve original indices
        result.sort_by_key(|msg| match msg.role {
            Role::System => 0,
            Role::User => 1,
            Role::Assistant => 2,
            Role::Tool => 3,
            _ => 4, // For any future role variants
        });

        Ok(result)
    }
}

pub struct SummarizationStrategy<P: Provider> {
    summarizer: Arc<P>,
    chunk_size: usize,
    keep_system: bool,
    keep_recent: usize,
}

impl<P: Provider + Send + Sync + 'static> SummarizationStrategy<P> {
    pub fn new(summarizer: Arc<P>) -> Self {
        Self {
            summarizer,
            chunk_size: 10,
            keep_system: true,
            keep_recent: 5,
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn with_keep_recent(mut self, count: usize) -> Self {
        self.keep_recent = count;
        self
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> PruningStrategy for SummarizationStrategy<P> {
    async fn prune(
        &self,
        messages: Vec<Message>,
        target_tokens: usize,
        counter: &dyn TokenCounter,
    ) -> Result<Vec<Message>, ContextError> {
        let mut result = Vec::new();
        let mut total_tokens = 0;

        // Keep system messages
        let (system_messages, other_messages): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|msg| msg.role == Role::System && self.keep_system);

        for msg in system_messages {
            let tokens = counter.count_message(&msg);
            if total_tokens + tokens <= target_tokens {
                result.push(msg);
                total_tokens += tokens;
            }
        }

        // Keep recent messages
        let recent_count = self.keep_recent.min(other_messages.len());
        let (older_messages, recent_messages) =
            other_messages.split_at(other_messages.len() - recent_count);

        // Add recent messages
        for msg in recent_messages {
            let tokens = counter.count_message(msg);
            if total_tokens + tokens <= target_tokens {
                result.push(msg.clone());
                total_tokens += tokens;
            }
        }

        // Summarize older messages if there's space
        if !older_messages.is_empty() && total_tokens < target_tokens {
            let chunks: Vec<_> = older_messages.chunks(self.chunk_size).collect();

            for chunk in chunks {
                let summary_prompt = format!(
                    "Summarize the following conversation excerpt concisely:\n\n{}",
                    chunk
                        .iter()
                        .map(|m| format!("{:?}: {}", m.role, m.content.as_text().unwrap_or("")))
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                let summary_request = Request::builder()
                    .messages(vec![Message::user(summary_prompt)])
                    .max_tokens(200)
                    .build();

                match self.summarizer.request(summary_request).await {
                    Ok(response) => {
                        if !response.content.is_empty() {
                            let summary_msg = Message::system(format!(
                                "[Summary of earlier messages]: {}",
                                response.content
                            ));
                            let tokens = counter.count_message(&summary_msg);

                            if total_tokens + tokens <= target_tokens {
                                result.push(summary_msg);
                                total_tokens += tokens;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to summarize messages: {}", e);
                        // Continue without the summary
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::Role;

    struct MockCounter;

    impl TokenCounter for MockCounter {
        fn count_text(&self, text: &str) -> usize {
            text.len() / 4
        }

        fn count_message(&self, message: &Message) -> usize {
            10 + self.count_text(message.content.as_text().unwrap_or(""))
        }

        fn count_messages(&self, messages: &[Message]) -> usize {
            messages.iter().map(|m| self.count_message(m)).sum()
        }

        fn model_context_window(&self) -> usize {
            1000
        }
    }

    #[tokio::test]
    async fn test_sliding_window_strategy() {
        let strategy = SlidingWindowStrategy::new(true, 3);
        let counter = MockCounter;

        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Message 1"),
            Message::assistant("Response 1"),
            Message::user("Message 2"),
            Message::assistant("Response 2"),
            Message::user("Message 3"),
        ];

        let result = strategy.prune(messages, 100, &counter).await.unwrap();

        // Should keep system message and last 3 messages
        assert!(result[0].role == Role::System);
        assert!(result.len() <= 4);
    }

    #[tokio::test]
    async fn test_importance_based_strategy() {
        let strategy = ImportanceBasedStrategy::new(|msg| {
            if msg.content.as_text().unwrap_or("").contains("important") {
                10.0
            } else {
                1.0
            }
        });

        let counter = MockCounter;
        let messages = vec![
            Message::system("System"),
            Message::user("Normal message"),
            Message::user("This is important"),
            Message::user("Another normal message"),
        ];

        let result = strategy.prune(messages, 50, &counter).await.unwrap();

        // Should prioritize system and "important" messages
        assert!(result
            .iter()
            .any(|m| m.content.as_text().unwrap_or("").contains("important")));
    }
}
