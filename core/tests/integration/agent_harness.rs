use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

use cogni_core::{
    error::AgentError,
    traits::{
        agent::{Agent, AgentConfig, AgentInput, AgentOutput},
        llm::LanguageModel,
        memory::MemoryStore,
        tool::Tool,
    },
};

/// Configuration for agent test harness
#[derive(Debug, Clone)]
pub struct AgentTestConfig<C: AgentConfig> {
    pub agent_config: C,
    pub llm: Arc<dyn LanguageModel<Prompt = String, Response = String> + Send + Sync>,
    pub tools: std::collections::HashMap<
        String,
        Arc<
            dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>
                + Send
                + Sync,
        >,
    >,
    pub memory: Arc<dyn MemoryStore + Send + Sync>,
}

/// Test harness for agent operations
pub struct AgentTestHarness<A: Agent> {
    pub agent: A,
    pub llm: Arc<dyn LanguageModel<Prompt = String, Response = String> + Send + Sync>,
    pub tools: std::collections::HashMap<
        String,
        Arc<
            dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>
                + Send
                + Sync,
        >,
    >,
    pub memory: Arc<dyn MemoryStore + Send + Sync>,
}

impl<A> AgentTestHarness<A>
where
    A: Agent,
{
    /// Create a new agent test harness with the given config
    pub fn new(
        config: AgentTestConfig<A::Config>,
    ) -> Result<Self, cogni_core::error::ToolConfigError> {
        let agent = A::try_new(config.agent_config)?;
        Ok(Self {
            agent,
            llm: config.llm,
            tools: config.tools,
            memory: config.memory,
        })
    }

    /// Initialize the agent
    pub async fn initialize(&mut self) -> Result<(), AgentError> {
        self.agent.initialize().await
    }

    /// Shutdown the agent
    pub async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.agent.shutdown().await
    }

    /// Execute the agent with given input
    pub async fn execute(&self, input: A::Input) -> Result<A::Output, AgentError> {
        self.agent.execute(input).await
    }

    /// Process an input through the agent workflow
    pub async fn process(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        self.agent
            .process(input, self.llm.clone(), &self.tools, self.memory.clone())
            .await
    }

    /// Generate a plan for the input
    pub async fn plan(&self, input: &str) -> Result<serde_json::Value, AgentError> {
        self.agent
            .plan(input, self.llm.clone(), self.memory.clone())
            .await
    }
}

/// Macro to simplify writing agent integration tests
#[macro_export]
macro_rules! agent_integration_test {
    ($test_name:ident, $agent_type:ty, $agent_config:expr, $test_block:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), cogni_core::error::Error> {
            use std::collections::HashMap;
            use std::sync::Arc;
            use cogni_core::traits::tool::Tool;
            use cogni_core::traits::memory::MemoryStore;
            use cogni_core::traits::llm::LanguageModel;
            use crate::integration::agent_harness::{AgentTestConfig, AgentTestHarness};

            // Setup mock LLM, tools, and memory for testing
            let llm = Arc::new(crate::test_utils::MockLanguageModel::new(vec![]));
            let tools: HashMap<String, Arc<dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()> + Send + Sync>> = HashMap::new();
            let memory = Arc::new(crate::test_utils::MockMemoryStore::new(vec![]));

            let config = AgentTestConfig {
                agent_config: $agent_config,
                llm,
                tools,
                memory,
            };

            let mut harness = AgentTestHarness::<$agent_type>::new(config)?;
            harness.initialize().await?;

            let result = async move $test_block.await;

            harness.shutdown().await?;

            result
        }
    };
}
