use crate::agent::strategy::{ReActStrategy, Strategy};
use crate::agent::{BasicAgent, LlmTokenStream};
use crate::error::AgentError;
use crate::traits::llm::LanguageModel;
use crate::traits::memory::MemoryStore;
use crate::traits::tool::{Tool, ToolSelector};
use serde_json::Value;
use std::sync::Arc;

/// Type alias for a tool with JSON input/output
pub type JsonTool = Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>;

/// Builder module for the agent.
/// Provides builder functions to create BasicAgent instances.
pub struct AgentBuilder {
    llm: Option<
        Arc<
            dyn LanguageModel<Prompt = String, Response = String, TokenStream = LlmTokenStream>
                + Send
                + Sync,
        >,
    >,
    tools: Option<Vec<JsonTool>>,
    memory: Option<Arc<dyn MemoryStore + Send + Sync>>,
    selector: Option<Arc<dyn ToolSelector>>,
    strategy: Option<Arc<dyn Strategy + Send + Sync>>,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            llm: None,
            tools: None,
            memory: None,
            selector: None,
            strategy: None,
        }
    }

    pub fn llm(
        mut self,
        llm: Arc<
            dyn LanguageModel<Prompt = String, Response = String, TokenStream = LlmTokenStream>
                + Send
                + Sync,
        >,
    ) -> Self {
        self.llm = Some(llm);
        self
    }

    pub fn tools(mut self, tools: Vec<JsonTool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn memory(mut self, memory: Arc<dyn MemoryStore + Send + Sync>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn selector(mut self, selector: Arc<dyn ToolSelector>) -> Self {
        self.selector = Some(selector);
        self
    }

    pub fn strategy(mut self, strategy: Arc<dyn Strategy + Send + Sync>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    #[allow(clippy::result_large_err)]
    pub fn build(self) -> Result<BasicAgent, AgentError> {
        let llm = self
            .llm
            .ok_or_else(|| AgentError::Config("LLM is required"))?;
        let tools = self
            .tools
            .ok_or_else(|| AgentError::Config("Tools are required"))?;
        let memory = self
            .memory
            .ok_or_else(|| AgentError::Config("Memory is required"))?;
        let _selector = self
            .selector
            .ok_or_else(|| AgentError::Config("Selector is required"))?;
        let strategy = self.strategy.unwrap_or_else(|| {
            Arc::new(ReActStrategy::default()) as Arc<dyn Strategy + Send + Sync>
        });

        // BasicAgent does not currently store selector, so ignoring selector here.
        // Could be extended to store selector if needed.

        Ok(BasicAgent::new(llm, tools, memory, strategy))
    }
}
