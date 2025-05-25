//! Streaming types for incremental responses

use crate::types::tool::ToolCall;
use std::collections::HashMap;

/// A chunk of content in a stream
#[derive(Debug, Clone, PartialEq)]
pub struct ContentDelta {
    /// The text content
    pub text: String,
}

/// A chunk of tool call information
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDelta {
    /// Index of the tool call being updated
    pub index: usize,
    /// Tool call ID (may be partial)
    pub id: Option<String>,
    /// Function name (may be partial)
    pub name: Option<String>,
    /// Arguments (may be partial JSON)
    pub arguments: Option<String>,
}

/// Metadata updates in a stream
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetadataDelta {
    /// Model information
    pub model: Option<String>,
    /// Response ID
    pub id: Option<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// Events that can occur during streaming
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// Content was generated
    Content(ContentDelta),
    /// Tool call information
    ToolCall(ToolCallDelta),
    /// Metadata update
    Metadata(MetadataDelta),
    /// Stream has ended
    Done,
}

/// Accumulates streaming events into a complete response
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    content: String,
    tool_calls: Vec<PartialToolCall>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl StreamAccumulator {
    /// Create a new accumulator
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event
    pub fn process_event(&mut self, event: StreamEvent) -> crate::error::Result<()> {
        match event {
            StreamEvent::Content(delta) => {
                self.content.push_str(&delta.text);
            }
            StreamEvent::ToolCall(delta) => {
                // Ensure we have enough tool calls
                while self.tool_calls.len() <= delta.index {
                    self.tool_calls.push(PartialToolCall::default());
                }

                let tool_call = &mut self.tool_calls[delta.index];
                if let Some(id) = delta.id {
                    tool_call.id.push_str(&id);
                }
                if let Some(name) = delta.name {
                    tool_call.name.push_str(&name);
                }
                if let Some(args) = delta.arguments {
                    tool_call.arguments.push_str(&args);
                }
            }
            StreamEvent::Metadata(delta) => {
                self.metadata.extend(delta.custom);
            }
            StreamEvent::Done => {}
        }
        Ok(())
    }

    /// Get the accumulated content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Convert accumulated tool calls to complete ones
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.tool_calls
            .iter()
            .filter(|tc| !tc.id.is_empty() && !tc.name.is_empty())
            .map(|tc| ToolCall {
                id: tc.id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            })
            .collect()
    }
}
