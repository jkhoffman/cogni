use crate::error::AgentError;
/// Strategy module for the agent.
/// Provides strategy implementations such as ReActStrategy.
use async_trait::async_trait;
use serde_json::Value;
use std::default::Default;

/// Actions that a strategy can return during planning.
pub enum Action {
    LlmCall(String),         // Prompt string
    ToolCall(String, Value), // Tool name and JSON input
    Done(String),            // Final output string
}

/// Agent state passed to the strategy for planning.
pub struct AgentState {
    pub user_input: String,
    pub step: usize,
}

#[async_trait]
pub trait Strategy: Send + Sync {
    async fn plan(&self, state: &AgentState) -> Result<Action, AgentError>;
}

/// Simple deterministic ReAct strategy implementation.
#[derive(Default)]
pub struct ReActStrategy {}

#[async_trait]
impl Strategy for ReActStrategy {
    async fn plan(&self, state: &AgentState) -> Result<Action, AgentError> {
        // Simple deterministic plan:
        // step 0: LlmCall with user prompt
        // step 1: if user prompt contains "search", ToolCall "search" with empty JSON
        // step 2: Done with final LLM response
        match state.step {
            0 => Ok(Action::LlmCall(state.user_input.clone())),
            1 => {
                if state.user_input.to_lowercase().contains("search") {
                    Ok(Action::ToolCall("search".to_string(), Value::default()))
                } else {
                    Ok(Action::Done("No search needed".to_string()))
                }
            }
            _ => Ok(Action::Done("Final response".to_string())),
        }
    }
}
