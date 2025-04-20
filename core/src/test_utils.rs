//! Test utilities for the Cogni framework.
//!
//! This module provides common utilities and mock implementations
//! for testing framework components.

#![cfg(test)]

use async_trait::async_trait;
use futures::Stream;
use serde::{Serialize, de::DeserializeOwned};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::{LlmError, MemoryError, ToolError},
    traits::{
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
        let response = self.generate(prompt, opts).await?;
        Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
    }

    fn name(&self) -> &'static str {
        "mock_model"
    }
}

/// A mock tool configuration for testing.
#[derive(Debug, Clone)]
pub struct MockToolConfig {
    pub value: String,
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

/// A mock tool for testing.
pub struct MockTool {
    config: MockToolConfig,
    invocations: Arc<Mutex<Vec<String>>>,
}

impl MockTool {
    /// Create a new mock tool
    pub fn new(config: MockToolConfig) -> Self {
        Self {
            config,
            invocations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the recorded invocations
    pub async fn invocations(&self) -> Vec<String> {
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
        Ok(format!("Processed: {}", input))
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

/// A mock memory store for testing.
pub struct MockMemoryStore {
    entries: Arc<Mutex<Vec<(SessionId, MemoryEntry)>>>,
}

impl MockMemoryStore {
    /// Create a new mock memory store
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl MemoryStore for MockMemoryStore {
    async fn load(&self, session: &SessionId, n: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.entries.lock().await;
        Ok(entries
            .iter()
            .filter(|(s, _)| s == session)
            .map(|(_, e)| e.clone())
            .take(n)
            .collect())
    }

    async fn save(&self, session: &SessionId, entry: MemoryEntry) -> Result<(), MemoryError> {
        self.entries.lock().await.push((session.clone(), entry));
        Ok(())
    }
}

impl Default for MockMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
