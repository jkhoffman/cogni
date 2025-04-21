//! Agent implementation for the Cogni framework.
//!
//! This module provides the core abstractions for implementing agents that can:
//! 1. Use language models to process information
//! 2. Select and invoke appropriate tools
//! 3. Store and retrieve information from memory
//! 4. Implement planning capabilities
//!
//! # Agent Lifecycle
//!
//! Agents follow a defined lifecycle:
//! 1. Creation - Agent is instantiated with its configuration
//! 2. Initialization - Agent performs setup (connecting to services, loading resources)
//! 3. Operation - Agent handles inputs and executes steps
//! 4. Shutdown - Agent performs cleanup
//!
//! # Usage Example
//!
//! ```rust,no_run
//! // Example will be provided as the implementation progresses
//! ```

pub mod builder;
pub mod memory_bridge;
pub mod strategy;
pub mod tool_pipeline;

use crate::error::{AgentError, LlmError};
use crate::traits::agent::{Agent, AgentInput, AgentOutput, ToolUse};
use crate::traits::llm::{GenerateOptions, LanguageModel};
use crate::traits::memory::{MemoryStore, Role};
use crate::traits::tool::Tool;
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use time::OffsetDateTime;

/// Type alias for the LLM token stream
pub type LlmTokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>;

/// Basic agent implementation holding LM, tools, memory, and strategy.
pub struct BasicAgent {
    llm: Arc<
        dyn LanguageModel<Prompt = String, Response = String, TokenStream = LlmTokenStream>
            + Send
            + Sync,
    >,
    tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>>,
    memory: Arc<dyn MemoryStore + Send + Sync>,
    strategy: Arc<dyn strategy::Strategy + Send + Sync>,
}

impl BasicAgent {
    /// Create a new BasicAgent.
    pub fn new(
        llm: Arc<
            dyn LanguageModel<Prompt = String, Response = String, TokenStream = LlmTokenStream>
                + Send
                + Sync,
        >,
        tools: Vec<Arc<dyn Tool<Input = Value, Output = Value, Config = ()> + Send + Sync>>,
        memory: Arc<dyn MemoryStore + Send + Sync>,
        strategy: Arc<dyn strategy::Strategy + Send + Sync>,
    ) -> Self {
        Self {
            llm,
            tools,
            memory,
            strategy,
        }
    }
}

#[async_trait]
impl Agent for BasicAgent {
    type Config = ();
    type Input = AgentInput;
    type Output = AgentOutput;

    fn try_new(_config: Self::Config) -> Result<Self, crate::error::ToolConfigError> {
        Err(crate::error::ToolConfigError::ValidationFailed(
            "BasicAgent does not support try_new".to_string(),
        ))
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        // Initialization logic here
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        // Shutdown logic here
        Ok(())
    }

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, AgentError> {
        use crate::agent::memory_bridge::MemoryBridge;
        use crate::agent::strategy::{Action, AgentState};
        use serde_json::json;

        // Create a session ID from the conversation ID or generate a new one
        let session_id = crate::traits::memory::SessionId::new(
            input
                .conversation_id
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        );

        let mut user_input = input.message.clone();
        let mut step = 0;
        let max_iterations = 3;
        let mut tool_uses = Vec::new();

        while step < max_iterations {
            // Load context from memory
            let _context = MemoryBridge::load(&*self.memory, &session_id, 10).await?;

            // Create agent state
            let state = AgentState {
                user_input: user_input.clone(),
                step,
            };

            // Plan next action
            let action = self.strategy.plan(&state).await?;

            match action {
                Action::LlmCall(prompt) => {
                    // Call LLM
                    let response = self
                        .llm
                        .generate(prompt, GenerateOptions::default())
                        .await
                        .map_err(AgentError::Llm)?;

                    // Save to memory
                    let entry = crate::traits::memory::MemoryEntry {
                        role: Role::Assistant,
                        content: response.clone(),
                        timestamp: OffsetDateTime::now_utc(),
                    };
                    MemoryBridge::save(&*self.memory, &session_id, entry).await?;

                    user_input = response;
                }
                Action::ToolCall(tool_name, input_json) => {
                    // For now, we'll just use a simple approach without the full ToolPipeline
                    // since we don't have a proper ToolRegistry or ToolSelector yet
                    let result = if let Some(tool) = self.tools.first() {
                        tool.invoke(input_json.clone())
                            .await
                            .map_err(AgentError::Tool)?
                    } else {
                        return Err(AgentError::Runtime("No tools available".to_string()));
                    };

                    // Convert result to string
                    let result_str = serde_json::to_string(&result)
                        .map_err(|e| AgentError::Runtime(format!("Serialization error: {}", e)))?;

                    // Save to memory
                    let entry = crate::traits::memory::MemoryEntry {
                        role: Role::System, // Using System role since there's no Tool role
                        content: result_str.clone(),
                        timestamp: OffsetDateTime::now_utc(),
                    };
                    MemoryBridge::save(&*self.memory, &session_id, entry).await?;

                    // Record tool use
                    let tool_use = ToolUse {
                        tool_name: tool_name.clone(),
                        input: input_json,
                        output: result,
                        timestamp: chrono::Utc::now(),
                    };
                    tool_uses.push(tool_use);

                    user_input = result_str;
                }
                Action::Done(final_output) => {
                    return Ok(AgentOutput {
                        message: final_output,
                        tool_uses,
                        data: json!({}),
                    });
                }
            }

            step += 1;
        }

        Ok(AgentOutput {
            message: user_input,
            tool_uses,
            data: json!({}),
        })
    }

    async fn process(
        &self,
        _input: AgentInput,
        _llm: Arc<
            dyn LanguageModel<
                Prompt = String,
                Response = String,
                TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>,
            >,
        >,
        _tools: &std::collections::HashMap<
            String,
            Arc<dyn Tool<Input = Value, Output = Value, Config = ()>>,
        >,
        _memory: Arc<dyn MemoryStore>,
    ) -> Result<AgentOutput, AgentError> {
        // Implement process logic here
        Ok(AgentOutput {
            message: "BasicAgent processed input".to_string(),
            tool_uses: vec![],
            data: serde_json::json!({}),
        })
    }

    async fn plan(
        &self,
        _input: &str,
        _llm: Arc<
            dyn LanguageModel<
                Prompt = String,
                Response = String,
                TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>,
            >,
        >,
        _memory: Arc<dyn MemoryStore>,
    ) -> Result<serde_json::Value, AgentError> {
        // Implement plan logic here
        Ok(serde_json::json!({ "plan": "BasicAgent plan" }))
    }
}
