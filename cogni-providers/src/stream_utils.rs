//! Common streaming utilities for all providers

use cogni_core::{ContentDelta, Error, MetadataDelta, StreamEvent, ToolCallDelta};
use std::collections::HashMap;

/// Common state for accumulating streaming data
#[derive(Default)]
pub struct StreamState {
    /// Buffer for incomplete data
    pub buffer: String,
    /// Current model name
    pub model: Option<String>,
    /// Tool calls being accumulated
    pub current_tool_calls: Vec<(String, String, String)>, // (id, name, arguments)
    /// Metadata being accumulated
    pub metadata: HashMap<String, String>,
}

/// Helper trait for parsing streaming responses
pub trait StreamParser: Send + Sync {
    /// Parse a chunk of data into stream events
    fn parse_chunk(
        &mut self,
        data: &str,
        state: &mut StreamState,
    ) -> Result<Vec<StreamEvent>, Error>;
}

/// Generic stream wrapper that handles common streaming logic
#[allow(dead_code)]
pub struct ProviderStream<S, P> {
    /// Inner stream (can be EventSource, Response bytes, etc.)
    inner: S,
    /// Parser for this provider
    parser: P,
    /// Current streaming state
    state: StreamState,
    /// Buffered events to return
    events: Vec<StreamEvent>,
}

impl<S, P> ProviderStream<S, P> {
    /// Create a new provider stream
    pub fn new(inner: S, parser: P) -> Self {
        Self {
            inner,
            parser,
            state: StreamState::default(),
            events: Vec::new(),
        }
    }
}

/// Helper to create common metadata events
pub fn create_metadata_event(
    model: Option<String>,
    custom: HashMap<String, String>,
) -> StreamEvent {
    StreamEvent::Metadata(MetadataDelta {
        model,
        id: None,
        custom,
    })
}

/// Helper to create content delta events
pub fn create_content_event(text: String) -> StreamEvent {
    StreamEvent::Content(ContentDelta { text })
}

/// Helper to create tool call delta events
pub fn create_tool_call_event(
    index: usize,
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
) -> StreamEvent {
    StreamEvent::ToolCall(ToolCallDelta {
        index,
        id,
        name,
        arguments,
    })
}

/// Common error handling for stream parsing
pub fn handle_parse_error(error: serde_json::Error, context: &str) -> Error {
    Error::Serialization {
        message: format!("Failed to parse {} response: {}", context, error),
        source: None,
    }
}

/// Buffer management for line-based streaming protocols
pub struct LineBuffer {
    buffer: String,
}

impl LineBuffer {
    /// Create a new line buffer
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Add data to buffer and return complete lines
    pub fn add_data(&mut self, data: &[u8]) -> Vec<String> {
        self.buffer.push_str(&String::from_utf8_lossy(data));

        let mut lines = Vec::new();
        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim().to_string();
            if !line.is_empty() {
                lines.push(line);
            }
            self.buffer.drain(..=pos);
        }

        lines
    }

    /// Get any remaining data in the buffer
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.buffer))
        }
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Common SSE (Server-Sent Events) parsing logic
pub fn parse_sse_line(line: &str) -> Option<(&str, &str)> {
    if let Some(pos) = line.find(':') {
        let (field, value) = line.split_at(pos);
        let value = value.get(1..)?.trim_start(); // Skip the ':' and trim spaces
        Some((field, value))
    } else {
        None
    }
}
