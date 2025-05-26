//! Response types for LLM interactions

use crate::types::tool::ToolCall;
use std::collections::HashMap;
use std::fmt;

/// Metadata about a response
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseMetadata {
    /// Model used for generation
    pub model: Option<String>,
    /// Unique ID for this response
    pub id: Option<String>,
    /// Usage statistics
    pub usage: Option<Usage>,
    /// Finish reason
    pub finish_reason: Option<FinishReason>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// Token usage statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Why the model stopped generating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// Natural end of message
    Stop,
    /// Hit max_tokens limit
    Length,
    /// Hit a stop sequence
    StopSequence,
    /// Model decided to call a tool
    ToolCalls,
    /// Content was filtered
    ContentFilter,
}

/// A complete response from an LLM
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The generated content
    pub content: String,
    /// Tool calls requested by the model
    pub tool_calls: Vec<ToolCall>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl Response {
    /// Create a simple text response
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: Vec::new(),
            metadata: ResponseMetadata::default(),
        }
    }

    /// Check if the response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)?;
        if !self.tool_calls.is_empty() {
            write!(f, " [+{} tool calls]", self.tool_calls.len())?;
        }
        Ok(())
    }
}

impl fmt::Display for FinishReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
            FinishReason::StopSequence => write!(f, "stop_sequence"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
        }
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Usage(prompt: {}, completion: {}, total: {})",
            self.prompt_tokens, self.completion_tokens, self.total_tokens
        )
    }
}
