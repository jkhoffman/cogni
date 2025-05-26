//! Integration tests for combined agentic features
//!
//! This test suite verifies that the three main agentic features work correctly together:
//! - Stateful conversation management
//! - Context window management
//! - Structured output generation

use cogni::{
    client::Client,
    context::{ContextManager, SlidingWindowStrategy},
    state::{FileStore, MemoryStore},
    Error, Message, Provider, Request, ResponseFormat, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tempfile::TempDir;

// Test structured output types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestAnalysis {
    summary: String,
    key_points: Vec<String>,
    confidence_score: f32,
}

impl StructuredOutput for TestAnalysis {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "Brief summary of the analysis"
                },
                "key_points": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Key points from the analysis"
                },
                "confidence_score": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "Confidence score between 0 and 1"
                }
            },
            "required": ["summary", "key_points", "confidence_score"]
        })
    }
}

// Mock provider for deterministic testing
#[derive(Clone)]
struct MockProvider {
    responses: Arc<std::sync::Mutex<Vec<String>>>,
    call_count: Arc<std::sync::Mutex<usize>>,
}

impl MockProvider {
    fn new() -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(vec![])),
            call_count: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    fn with_responses(self, responses: Vec<String>) -> Self {
        *self.responses.lock().unwrap() = responses;
        self
    }

    fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    type Stream = futures::stream::Empty<Result<cogni::StreamEvent, Error>>;

    async fn request(&self, request: Request) -> Result<cogni::Response, Error> {
        let mut count = self.call_count.lock().unwrap();
        let responses = self.responses.lock().unwrap();

        let response_content = if *count < responses.len() {
            responses[*count].clone()
        } else if let Some(format) = &request.response_format {
            // For structured output requests, return appropriate JSON
            match format {
                ResponseFormat::JsonSchema { .. } => serde_json::json!({
                    "summary": "Test analysis summary",
                    "key_points": ["Point 1", "Point 2"],
                    "confidence_score": 0.85
                })
                .to_string(),
                ResponseFormat::JsonObject => serde_json::json!({
                    "test": "response"
                })
                .to_string(),
            }
        } else {
            format!("Response to message {}", *count + 1)
        };

        *count += 1;

        Ok(cogni::Response {
            content: response_content,
            tool_calls: vec![],
            metadata: cogni::ResponseMetadata {
                usage: Some(cogni::Usage {
                    prompt_tokens: request.messages.len() as u32 * 10,
                    completion_tokens: 20,
                    total_tokens: request.messages.len() as u32 * 10 + 20,
                }),
                ..Default::default()
            },
        })
    }

    async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
        Ok(futures::stream::empty())
    }
}

#[tokio::test]
async fn test_stateful_conversation_persistence() {
    // Create a temporary directory for state storage
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(FileStore::new(temp_dir.path()).unwrap());

    let provider = MockProvider::new().with_responses(vec![
        "Hello! I'm ready to help.".to_string(),
        "The weather is sunny today.".to_string(),
        "Yes, I remember you asked about the weather.".to_string(),
    ]);

    let client = Client::new(provider.clone());
    let mut agent = client.with_state(store.clone());

    // Start a new conversation
    let conversation_id = agent.new_conversation().await.unwrap();

    // Add system message
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::system("You are a helpful assistant."));
        state.set_title("Test Conversation");
        state.add_tag("test");
    }

    // First interaction
    let response1 = agent.chat("Hello!").await.unwrap();
    assert_eq!(response1.content, "Hello! I'm ready to help.");

    // Second interaction
    let response2 = agent.chat("What's the weather?").await.unwrap();
    assert_eq!(response2.content, "The weather is sunny today.");

    // Save conversation
    agent.save().await.unwrap();

    // Create a new agent and load the conversation
    let client2 = Client::new(provider.clone());
    let mut agent2 = client2.with_state(store.clone());
    agent2.load_conversation(conversation_id).await.unwrap();

    // Verify conversation was loaded correctly
    let state = agent2.current_state().unwrap();
    assert_eq!(state.messages.len(), 5); // system + 2 user + 2 assistant
    assert_eq!(state.metadata.title, Some("Test Conversation".to_string()));
    assert!(state.metadata.tags.contains(&"test".to_string()));

    // Continue the conversation
    let response3 = agent2.chat("Do you remember what I asked?").await.unwrap();
    assert_eq!(
        response3.content,
        "Yes, I remember you asked about the weather."
    );

    // Verify provider received all messages in context
    assert_eq!(provider.call_count(), 3);
}

#[tokio::test]
async fn test_context_manager_with_conversation() {
    let store = Arc::new(MemoryStore::new());
    let provider = MockProvider::new();

    // Create a token counter that returns predictable counts
    struct MockTokenCounter;
    impl cogni::context::TokenCounter for MockTokenCounter {
        fn count_text(&self, text: &str) -> usize {
            text.len() / 4 // Approximate 4 chars per token
        }

        fn count_message(&self, message: &Message) -> usize {
            self.count_text(message.content.as_text().unwrap_or_default()) + 3 // Role overhead
        }

        fn count_messages(&self, messages: &[Message]) -> usize {
            messages.iter().map(|m| self.count_message(m)).sum()
        }

        fn model_context_window(&self) -> usize {
            100 // Small window for testing
        }
    }

    let context_manager = ContextManager::new(Arc::new(MockTokenCounter))
        .with_max_tokens(100)
        .with_reserve_output_tokens(20)
        .with_strategy(Arc::new(SlidingWindowStrategy::new(true, 3))); // Keep system + last 3

    let client = Client::new(provider);
    let mut agent = client.with_state(store);

    // Start conversation
    agent.new_conversation().await.unwrap();

    // Add messages that will exceed context
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::system("System prompt"));
        for i in 1..=10 {
            state.add_message(Message::user(format!("User message {}", i)));
            state.add_message(Message::assistant(format!("Assistant response {}", i)));
        }
    }

    // Apply context management
    let all_messages = agent
        .current_state()
        .map(|s| s.messages.clone())
        .unwrap_or_default();
    assert_eq!(all_messages.len(), 21); // 1 system + 10 user + 10 assistant

    let pruned_messages = context_manager.fit_messages(all_messages).await.unwrap();

    // The sliding window strategy keeps system + last N messages
    // With keep_recent=3, it should keep the system message + last 3 messages
    assert!(pruned_messages.len() <= 4); // At most system + 3 recent
    assert_eq!(pruned_messages[0].role, cogni::Role::System);

    // The last message should be the most recent assistant response
    let last_msg = pruned_messages.last().unwrap();
    assert_eq!(last_msg.role, cogni::Role::Assistant);
    assert!(last_msg
        .content
        .as_text()
        .unwrap()
        .contains("Assistant response 10"));
}

#[tokio::test]
async fn test_structured_output_with_state() {
    let store = Arc::new(MemoryStore::new());
    let provider = MockProvider::new();

    let client = Client::new(provider.clone());
    let mut agent = client.with_state(store);

    // Start conversation
    agent.new_conversation().await.unwrap();

    // Add context
    agent
        .chat("I need help analyzing some data.")
        .await
        .unwrap();
    agent
        .chat("The data shows increasing trends.")
        .await
        .unwrap();

    // Request structured analysis
    let request = Request::builder()
        .messages(
            agent
                .current_state()
                .map(|s| s.messages.clone())
                .unwrap_or_default(),
        )
        .response_format(ResponseFormat::JsonSchema {
            schema: TestAnalysis::schema(),
            strict: true,
        })
        .build();

    let response = provider.request(request).await.unwrap();
    let analysis = response.parse_structured::<TestAnalysis>().unwrap();

    assert_eq!(analysis.summary, "Test analysis summary");
    assert_eq!(analysis.key_points.len(), 2);
    assert!((analysis.confidence_score - 0.85).abs() < 0.001);
}

#[tokio::test]
async fn test_all_features_combined() {
    // This test combines all three features in a realistic scenario
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(FileStore::new(temp_dir.path()).unwrap());

    let provider = MockProvider::new().with_responses(vec![
        "I'll help you analyze the sales data.".to_string(),
        "I see the Q1 revenue was $1M.".to_string(),
        "Q2 showed 20% growth.".to_string(),
        serde_json::json!({
            "summary": "Strong growth trend in H1",
            "key_points": [
                "Q1 revenue: $1M",
                "Q2 growth: 20%",
                "Positive momentum"
            ],
            "confidence_score": 0.9
        })
        .to_string(),
    ]);

    // Set up context management
    let context_manager = ContextManager::new(Arc::new(MockTokenCounter))
        .with_max_tokens(200)
        .with_reserve_output_tokens(50)
        .with_strategy(Arc::new(SlidingWindowStrategy::new(true, 5)));

    let client = Client::new(provider.clone());
    let mut agent = client.with_state(store.clone());

    // Start conversation with metadata
    let conv_id = agent.new_conversation().await.unwrap();
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::system(
            "You are a business analyst specializing in sales data.",
        ));
        state.set_title("H1 Sales Analysis");
        state.add_tag("sales");
        state.add_tag("2024-H1");
    }

    // Build conversation context
    agent
        .chat("I need help analyzing sales data.")
        .await
        .unwrap();
    agent.chat("Q1 revenue was $1M.").await.unwrap();
    agent.chat("Q2 showed 20% growth.").await.unwrap();

    // Save state
    agent.save().await.unwrap();

    // Load in new agent
    let client2 = Client::new(provider.clone());
    let mut agent2 = client2.with_state(store.clone());
    agent2.load_conversation(conv_id).await.unwrap();

    // Request structured analysis with context management
    let messages = agent2
        .current_state()
        .map(|s| s.messages.clone())
        .unwrap_or_default();
    let pruned_messages = context_manager.fit_messages(messages).await.unwrap();

    let request = Request::builder()
        .messages(pruned_messages)
        .response_format(ResponseFormat::JsonSchema {
            schema: TestAnalysis::schema(),
            strict: true,
        })
        .build();

    let response = provider.request(request).await.unwrap();
    let analysis = response.parse_structured::<TestAnalysis>().unwrap();

    // Verify results
    assert_eq!(analysis.summary, "Strong growth trend in H1");
    assert_eq!(analysis.key_points.len(), 3);
    assert!(analysis.confidence_score > 0.8);

    // Verify conversation persistence
    let saved_convs = agent2.list_conversations().await.unwrap();
    assert_eq!(saved_convs.len(), 1);
    assert_eq!(
        saved_convs[0].metadata.title,
        Some("H1 Sales Analysis".to_string())
    );
    assert!(saved_convs[0].metadata.tags.contains(&"sales".to_string()));
}

#[tokio::test]
async fn test_context_pruning_strategies() {
    let store = Arc::new(MemoryStore::new());
    let provider = MockProvider::new();

    // Test importance-based pruning
    let importance_strategy =
        cogni::context::ImportanceBasedStrategy::new(Box::new(|msg: &Message| {
            if msg.role == cogni::Role::System {
                1.0 // System messages are most important
            } else if msg
                .content
                .as_text()
                .unwrap_or_default()
                .contains("important")
            {
                0.8
            } else {
                0.3
            }
        }));

    let context_manager = ContextManager::new(Arc::new(MockTokenCounter))
        .with_max_tokens(80)
        .with_reserve_output_tokens(20)
        .with_strategy(Arc::new(importance_strategy));

    let client = Client::new(provider);
    let mut agent = client.with_state(store);

    agent.new_conversation().await.unwrap();

    // Add messages with varying importance
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::system("System instructions"));
        state.add_message(Message::user("This is important information"));
        state.add_message(Message::assistant("Acknowledged important info"));
        state.add_message(Message::user("Random chat 1"));
        state.add_message(Message::assistant("Response 1"));
        state.add_message(Message::user("Random chat 2"));
        state.add_message(Message::assistant("Response 2"));
        state.add_message(Message::user("Another important detail"));
        state.add_message(Message::assistant("Noted the important detail"));
    }

    let all_messages = agent
        .current_state()
        .map(|s| s.messages.clone())
        .unwrap_or_default();
    let pruned = context_manager.fit_messages(all_messages).await.unwrap();

    // Should keep system message and messages containing "important"
    assert!(pruned.iter().any(|m| m.role == cogni::Role::System));
    assert!(
        pruned
            .iter()
            .filter(|m| m
                .content
                .as_text()
                .unwrap_or_default()
                .contains("important"))
            .count()
            >= 2
    );
}

#[tokio::test]
async fn test_real_provider_integration() {
    // Skip if no API key
    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        eprintln!("Skipping real provider test - OPENAI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(FileStore::new(temp_dir.path()).unwrap());

    let provider = cogni::providers::openai::OpenAI::with_api_key(api_key);
    let context_manager = ContextManager::new(Arc::new(
        cogni::context::TiktokenCounter::for_model("gpt-3.5-turbo").unwrap(),
    ))
    .with_max_tokens(4000)
    .with_reserve_output_tokens(500);

    let client = Client::new(provider);
    let mut agent = client.with_state(store);

    // Create a conversation
    agent.new_conversation().await.unwrap();
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::system(
            "You are a helpful assistant. Keep responses brief.",
        ));
        state.set_title("Integration Test");
    }

    // Test basic chat
    let response = agent.chat("Say 'test successful' and nothing else.").await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(response.content.to_lowercase().contains("test successful"));

    // Test with context management
    let messages = agent
        .current_state()
        .map(|s| s.messages.clone())
        .unwrap_or_default();
    let pruned = context_manager.fit_messages(messages).await.unwrap();
    assert!(!pruned.is_empty());

    // Save and verify
    agent.save().await.unwrap();
    let saved_convs = agent.list_conversations().await.unwrap();
    assert_eq!(saved_convs.len(), 1);
    assert_eq!(
        saved_convs[0].metadata.title,
        Some("Integration Test".to_string())
    );
}

// Mock token counter for testing
struct MockTokenCounter;

impl cogni::context::TokenCounter for MockTokenCounter {
    fn count_text(&self, text: &str) -> usize {
        text.len() / 4
    }

    fn count_message(&self, message: &Message) -> usize {
        self.count_text(message.content.as_text().unwrap_or_default()) + 3
    }

    fn count_messages(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.count_message(m)).sum()
    }

    fn model_context_window(&self) -> usize {
        100
    }
}
