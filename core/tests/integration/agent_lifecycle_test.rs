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
    traits::agent::{Agent, AgentInput, AgentOutput},
    traits::llm::{GenerateOptions, LanguageModel},
    traits::tool::{Tool, ToolSelector},
    InMemoryMemory,
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

/// Assertion helper to check that AgentOutput message contains expected substring.
fn assert_agent_output_contains(output: &AgentOutput, expected_substring: &str) {
    assert!(
        output.message.contains(expected_substring),
        "AgentOutput message '{}' does not contain expected substring '{}'",
        output.message,
        expected_substring
    );
}

/// Generate a basic AgentInput with a message and optional conversation ID.
fn generate_agent_input(message: &str, conversation_id: Option<&str>) -> AgentInput {
    AgentInput {
        message: message.to_string(),
        conversation_id: conversation_id.map(|s| s.to_string()),
        context: serde_json::json!({}),
    }
}

#[tokio::test]
async fn test_agent_lifecycle() -> Result<(), AgentError> {
    // Create mock components
    let llm = Arc::new(MockLanguageModel::new(vec![
        "I'll help you with that".to_string(),
        "Here's the information you requested".to_string(),
        "Final response".to_string(),
    ]));
    let tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>> =
        vec![];
    let memory = Arc::new(InMemoryMemory::new());
    let strategy = Arc::new(ReActStrategy::default());
    let selector = Arc::new(MockToolSelector);

    // Build agent using builder
    let mut agent = AgentBuilder::new()
        .llm(llm)
        .tools(tools)
        .memory(memory)
        .selector(selector)
        .strategy(strategy)
        .build()?;

    // Initialize agent
    agent.initialize().await?;

    // Execute agent with input
    let input = generate_agent_input("Hello, can you help me?", Some("test-session"));
    let output = agent.execute(input).await?;

    // Verify output
    assert_agent_output_contains(&output, "I'll help you");

    // Execute agent again with follow-up
    let input = generate_agent_input("Can you search for information?", Some("test-session"));
    let output = agent.execute(input).await?;

    // Verify output
    assert_agent_output_contains(&output, "information");

    // Shutdown agent
    agent.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_agent_execute_loop() -> Result<(), AgentError> {
    // Create mock components with responses for a 3-step loop
    let llm = Arc::new(MockLanguageModel::new(vec![
        "Step 1 response".to_string(),
        "Step 2 response".to_string(),
        "Final response".to_string(),
    ]));
    let tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>> =
        vec![];
    let memory = Arc::new(InMemoryMemory::new());
    let selector = Arc::new(MockToolSelector);

    // Create a strategy that will go through all 3 steps
    struct TestStrategy {
        step: std::sync::atomic::AtomicUsize,
    }

    #[async_trait]
    impl cogni_core::agent::strategy::Strategy for TestStrategy {
        async fn plan(
            &self,
            state: &cogni_core::agent::strategy::AgentState,
        ) -> Result<cogni_core::agent::strategy::Action, AgentError> {
            let current_step = self.step.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match current_step {
                0 => Ok(cogni_core::agent::strategy::Action::LlmCall(
                    state.user_input.clone(),
                )),
                1 => Ok(cogni_core::agent::strategy::Action::LlmCall(
                    state.user_input.clone(),
                )),
                _ => Ok(cogni_core::agent::strategy::Action::Done(
                    state.user_input.clone(),
                )),
            }
        }
    }

    let strategy = Arc::new(TestStrategy {
        step: std::sync::atomic::AtomicUsize::new(0),
    });

    // Build agent using builder
    let mut agent = AgentBuilder::new()
        .llm(llm)
        .tools(tools)
        .memory(memory)
        .selector(selector)
        .strategy(strategy)
        .build()?;

    // Initialize agent
    agent.initialize().await?;

    // Execute agent with input - should go through all 3 steps
    let input = generate_agent_input("Test input", Some("test-session"));
    let output = agent.execute(input).await?;

    // Verify output contains the final response
    assert_agent_output_contains(&output, "Final response");

    // Shutdown agent
    agent.shutdown().await?;

    Ok(())
}
