use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use cogni_core::{
    agent::builder::AgentBuilder,
    agent::strategy::ReActStrategy,
    error::{AgentError, LlmError, ToolError},
    traits::llm::{GenerateOptions, LanguageModel},
    traits::tool::{Tool, ToolSelector},
};

/// A mock language model for testing.
struct MockLanguageModel {
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockLanguageModel {
    /// Create a new mock model with predefined responses
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
        }
    }
}

#[async_trait]
impl LanguageModel for MockLanguageModel {
    type Prompt = String;
    type Response = String;
    type TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>;

    async fn generate(
        &self,
        _prompt: Self::Prompt,
        _options: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        let mut responses = self.responses.lock().await;
        if responses.is_empty() {
            Ok("default response".to_string())
        } else {
            Ok(responses.remove(0))
        }
    }

    async fn stream_generate(
        &self,
        _prompt: Self::Prompt,
        _options: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        // For simplicity, return an empty stream
        Ok(Box::pin(Box::pin(futures::stream::empty())))
    }

    fn name(&self) -> &'static str {
        "mock_language_model"
    }
}

/// A mock tool selector for testing.
struct MockToolSelector;

#[async_trait]
impl ToolSelector for MockToolSelector {
    async fn select(
        &self,
        _tools: &HashMap<String, Arc<dyn Tool<Input = Value, Output = Value, Config = ()>>>,
        _context: &str,
    ) -> Result<Option<String>, ToolError> {
        Ok(Some("mock_tool".to_string()))
    }
}

#[tokio::test]
async fn test_agent_builder_success() {
    // Create mock components
    let llm = Arc::new(MockLanguageModel::new(vec!["test response".to_string()]));
    let tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>> =
        vec![];
    let memory = Arc::new(cogni_core::InMemoryMemory::new());
    let strategy = Arc::new(ReActStrategy::default());
    let selector = Arc::new(MockToolSelector);

    // Build agent using builder
    let agent = AgentBuilder::new()
        .llm(llm)
        .tools(tools)
        .memory(memory)
        .selector(selector)
        .strategy(strategy)
        .build();

    // Verify agent was built successfully
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_agent_builder_missing_deps() {
    // Test missing LLM
    let tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>> =
        vec![];
    let memory = Arc::new(cogni_core::InMemoryMemory::new());
    let strategy = Arc::new(ReActStrategy::default());
    let selector = Arc::new(MockToolSelector);

    let agent = AgentBuilder::new()
        .tools(tools.clone())
        .memory(memory.clone())
        .selector(selector.clone())
        .strategy(strategy.clone())
        .build();

    assert!(agent.is_err());
    if let Err(AgentError::Config(msg)) = agent {
        assert!(msg.contains("LLM is required"));
    } else {
        panic!("Expected AgentError::Config");
    }

    // Test missing tools
    let llm = Arc::new(MockLanguageModel::new(vec!["test response".to_string()]));
    let agent = AgentBuilder::new()
        .llm(llm.clone())
        .memory(memory.clone())
        .selector(selector.clone())
        .strategy(strategy.clone())
        .build();

    assert!(agent.is_err());
    if let Err(AgentError::Config(msg)) = agent {
        assert!(msg.contains("Tools are required"));
    } else {
        panic!("Expected AgentError::Config");
    }

    // Test missing memory
    let agent = AgentBuilder::new()
        .llm(llm.clone())
        .tools(tools.clone())
        .selector(selector.clone())
        .strategy(strategy.clone())
        .build();

    assert!(agent.is_err());
    if let Err(AgentError::Config(msg)) = agent {
        assert!(msg.contains("Memory is required"));
    } else {
        panic!("Expected AgentError::Config");
    }

    // Test missing selector
    let agent = AgentBuilder::new()
        .llm(llm.clone())
        .tools(tools.clone())
        .memory(memory.clone())
        .strategy(strategy.clone())
        .build();

    assert!(agent.is_err());
    if let Err(AgentError::Config(msg)) = agent {
        assert!(msg.contains("Selector is required"));
    } else {
        panic!("Expected AgentError::Config");
    }

    // Test with all required components
    let agent = AgentBuilder::new()
        .llm(llm.clone())
        .tools(tools.clone())
        .memory(memory.clone())
        .selector(selector.clone())
        .build();

    // This should succeed with default strategy
    assert!(agent.is_ok());
}
