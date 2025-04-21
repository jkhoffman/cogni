//! Test utilities for the Cogni framework.
//!
//! This module provides common utilities and mock implementations
//! for testing framework components.

#![cfg(test)]

use async_trait::async_trait;
use futures::Stream;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::{AgentError, LlmError, MemoryError, ToolError},
    traits::{
        agent::{Agent, AgentConfig, AgentInput, AgentOutput},
        llm::{GenerateOptions, LanguageModel},
        memory::{MemoryEntry, MemoryStore, SessionId},
        tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
    },
};

/// A mock language model for testing.
pub struct MockLanguageModel {
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockLanguageModel {
    /// Create a new mock model with predefined responses
    pub fn new(responses: Vec<String>) -> Self {
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
        _options: Option<GenerateOptions>,
    ) -> Result<Self::Response, LlmError> {
        let mut responses = self.responses.lock().await;
        if responses.is_empty() {
            Ok("default response".to_string())
        } else {
            Ok(responses.remove(0))
        }
    }

    fn stream(
        &self,
        _prompt: Self::Prompt,
        _options: Option<GenerateOptions>,
    ) -> Self::TokenStream {
        // For simplicity, return an empty stream
        Box::pin(futures::stream::empty())
    }
}

/// Mock Agent implementation for testing.
pub struct MockAgent {
    pub config: MockAgentConfig,
    pub state: Arc<Mutex<String>>,
}

/// Configuration for MockAgent.
#[derive(Debug, Clone)]
pub struct MockAgentConfig {
    pub initial_state: String,
}

impl AgentConfig for MockAgentConfig {
    fn validate(&self) -> Result<(), crate::traits::tool::ToolConfigError> {
        // Simple validation: initial_state must not be empty
        if self.initial_state.is_empty() {
            Err(crate::traits::tool::ToolConfigError::new(
                "initial_state cannot be empty",
            ))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Agent for MockAgent {
    type Config = MockAgentConfig;
    type Input = AgentInput;
    type Output = AgentOutput;

    fn try_new(config: Self::Config) -> Result<Self, crate::traits::tool::ToolConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(String::new())),
        })
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        let mut state = self.state.lock().await;
        *state = self.config.initial_state.clone();
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        let mut state = self.state.lock().await;
        *state = String::new();
        Ok(())
    }

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, AgentError> {
        let mut state = self.state.lock().await;
        // Append input message to state for testing
        state.push_str(&input.message);

        Ok(AgentOutput {
            message: format!("Echo: {}", input.message),
            tool_uses: vec![],
            data: serde_json::json!({ "state": *state }),
        })
    }

    async fn process(
        &self,
        input: AgentInput,
        _llm: Arc<
            dyn LanguageModel<
                Prompt = String,
                Response = String,
                TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>,
            >,
        >,
        _tools: &HashMap<
            String,
            Arc<dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>>,
        >,
        _memory: Arc<dyn MemoryStore>,
    ) -> Result<AgentOutput, AgentError> {
        self.execute(input).await
    }
}

/// Generate a basic AgentInput with a message and optional conversation ID.
pub fn generate_agent_input(message: &str, conversation_id: Option<&str>) -> AgentInput {
    AgentInput {
        message: message.to_string(),
        conversation_id: conversation_id.map(|s| s.to_string()),
        context: serde_json::json!({}),
    }
}

/// Assertion helper to check that AgentOutput message contains expected substring.
pub fn assert_agent_output_contains(output: &AgentOutput, expected_substring: &str) {
    assert!(
        output.message.contains(expected_substring),
        "AgentOutput message '{}' does not contain expected substring '{}'",
        output.message,
        expected_substring
    );
}

/// Assertion helper to check that AgentOutput data contains a specific key.
pub fn assert_agent_output_data_has_key(output: &AgentOutput, key: &str) {
    assert!(
        output.data.get(key).is_some(),
        "AgentOutput data does not contain key '{}'",
        key
    );
}
