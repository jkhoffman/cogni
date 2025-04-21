//! Integration test harness for the Cogni framework.
//!
//! This module provides the infrastructure for running end-to-end tests
//! of the framework, including LLM interactions, tool usage, and memory
//! operations.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use cogni_core::{
    error::{Error, LlmError, ToolError},
    traits::{
        llm::{GenerateOptions, LanguageModel},
        memory::{MemoryEntry, MemoryStore, SessionId},
        tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
        Builder,
    },
};

mod agent_harness;

pub use agent_harness::agent_integration_test;
pub use agent_harness::{AgentTestConfig, AgentTestHarness};

/// Test harness configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Whether to use mock LLMs or real ones
    pub use_mock_llm: bool,
    /// Whether to use mock tools or real ones
    pub use_mock_tools: bool,
    /// Whether to use mock memory or real storage
    pub use_mock_memory: bool,
    /// Test timeout in seconds
    pub timeout_secs: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_mock_llm: true,
            use_mock_tools: true,
            use_mock_memory: true,
            timeout_secs: 30,
        }
    }
}

/// A builder for constructing test scenarios
#[derive(Debug)]
pub struct TestBuilder {
    config: TestConfig,
    llm_responses: Vec<String>,
    tool_responses: Vec<String>,
    memory_entries: Vec<MemoryEntry>,
}

impl TestBuilder {
    /// Create a new test builder with default configuration
    pub fn new() -> Self {
        Self {
            config: TestConfig::default(),
            llm_responses: Vec::new(),
            tool_responses: Vec::new(),
            memory_entries: Vec::new(),
        }
    }

    /// Set the test configuration
    pub fn with_config(mut self, config: TestConfig) -> Self {
        self.config = config;
        self
    }

    /// Add expected LLM responses
    pub fn with_llm_responses(mut self, responses: Vec<String>) -> Self {
        self.llm_responses = responses;
        self
    }

    /// Add expected tool responses
    pub fn with_tool_responses(mut self, responses: Vec<String>) -> Self {
        self.tool_responses = responses;
        self
    }

    /// Add initial memory entries
    pub fn with_memory_entries(mut self, entries: Vec<MemoryEntry>) -> Self {
        self.memory_entries = entries;
        self
    }

    /// Build the test harness
    pub fn build(self) -> TestHarness {
        TestHarness {
            config: self.config,
            llm: Arc::new(MockLanguageModel::new(self.llm_responses)),
            tool: Arc::new(MockTool::new(MockToolConfig {
                value: "test".into(),
                responses: self.tool_responses,
            })),
            memory: Arc::new(MockMemoryStore::new(self.memory_entries)),
        }
    }
}

/// The main test harness that coordinates test components
#[derive(Debug)]
pub struct TestHarness {
    config: TestConfig,
    llm: Arc<dyn LanguageModel<Prompt = String, Response = String> + Send + Sync>,
    tool: Arc<dyn Tool<Input = String, Output = String> + Send + Sync>,
    memory: Arc<dyn MemoryStore + Send + Sync>,
}

impl TestHarness {
    /// Create a new test harness with default configuration
    pub fn new() -> Self {
        TestBuilder::new().build()
    }

    /// Get a reference to the LLM
    pub fn llm(&self) -> Arc<dyn LanguageModel<Prompt = String, Response = String> + Send + Sync> {
        self.llm.clone()
    }

    /// Get a reference to the tool
    pub fn tool(&self) -> Arc<dyn Tool<Input = String, Output = String> + Send + Sync> {
        self.tool.clone()
    }

    /// Get a reference to the memory store
    pub fn memory(&self) -> Arc<dyn MemoryStore + Send + Sync> {
        self.memory.clone()
    }

    /// Run a test scenario with timeout
    pub async fn run<F, Fut>(&self, test_fn: F) -> Result<(), Error>
    where
        F: FnOnce(TestHarness) -> Fut,
        Fut: std::future::Future<Output = Result<(), Error>>,
    {
        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);
        tokio::time::timeout(timeout, test_fn(self.clone()))
            .await
            .map_err(|_| Error::Timeout)?
    }
}

impl Clone for TestHarness {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            llm: self.llm.clone(),
            tool: self.tool.clone(),
            memory: self.memory.clone(),
        }
    }
}

/// A mock tool configuration that includes expected responses
#[derive(Debug, Clone)]
struct MockToolConfig {
    value: String,
    responses: Vec<String>,
}

impl ToolConfig for MockToolConfig {
    fn validate(&self) -> Result<(), String> {
        if self.value.is_empty() {
            Err("value cannot be empty".into())
        } else {
            Ok(())
        }
    }
}

/// A mock tool that returns predefined responses
struct MockTool {
    config: MockToolConfig,
    invocations: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockTool {
    fn new(config: MockToolConfig) -> Self {
        Self {
            responses: Arc::new(Mutex::new(config.responses.clone())),
            invocations: Arc::new(Mutex::new(Vec::new())),
            config,
        }
    }

    async fn invocations(&self) -> Vec<String> {
        self.invocations.lock().await.clone()
    }
}

#[async_trait]
impl Tool for MockTool {
    type Input = String;
    type Output = String;
    type Config = MockToolConfig;

    fn try_new(config: Self::Config) -> Result<Self, ToolError> {
        Ok(Self::new(config))
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Stateless, ToolCapability::ThreadSafe]
    }

    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
        self.invocations.lock().await.push(input.clone());
        let mut responses = self.responses.lock().await;
        responses
            .pop()
            .ok_or_else(|| ToolError::InvocationFailed("No more responses".into()))
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "mock_tool".into(),
            description: "A mock tool for testing".into(),
            input_schema: serde_json::json!({
                "type": "string"
            }),
            output_schema: serde_json::json!({
                "type": "string"
            }),
            examples: vec![],
        }
    }
}

/// A mock language model that returns predefined responses
struct MockLanguageModel {
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockLanguageModel {
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
        _opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        let mut responses = self.responses.lock().await;
        responses
            .pop()
            .ok_or_else(|| LlmError::InvalidResponse("No more responses".into()))
    }

    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Self::TokenStream, LlmError> {
        use futures::stream;
        let response = self.generate(prompt, opts).await?;
        Ok(Box::pin(stream::once(async move { Ok(response) })))
    }

    fn name(&self) -> &'static str {
        "mock_model"
    }
}

/// A mock memory store that can be pre-populated with entries
struct MockMemoryStore {
    entries: Arc<Mutex<Vec<(SessionId, MemoryEntry)>>>,
}

impl MockMemoryStore {
    fn new(initial_entries: Vec<MemoryEntry>) -> Self {
        let entries = initial_entries
            .into_iter()
            .map(|entry| (SessionId::new("test"), entry))
            .collect();
        Self {
            entries: Arc::new(Mutex::new(entries)),
        }
    }
}

#[async_trait]
impl MemoryStore for MockMemoryStore {
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, Error> {
        let entries = self.entries.lock().await;
        Ok(entries
            .iter()
            .filter(|(s, _)| s == session)
            .map(|(_, e)| e.clone())
            .take(n)
            .collect())
    }

    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), Error> {
        self.entries.lock().await.push((session.clone(), entry));
        Ok(())
    }
}
