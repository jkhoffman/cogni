//! Ollama streaming implementation

use crate::ollama::converter::OllamaStreamResponse;
use bytes::Bytes;
use cogni_core::{ContentDelta, Error, MetadataDelta, StreamEvent, ToolCallDelta};
use futures_core::Stream;
use reqwest::Response as ReqwestResponse;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct OllamaStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    model: Option<String>,
    current_tool_calls: Vec<(String, String, String)>, // (id, name, arguments)
}

impl OllamaStream {
    pub fn new(response: ReqwestResponse) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            model: None,
            current_tool_calls: Vec::new(),
        }
    }

    fn parse_line(&mut self, line: &str) -> Result<Option<StreamEvent>, Error> {
        if line.is_empty() {
            return Ok(None);
        }

        let response: OllamaStreamResponse =
            serde_json::from_str(line).map_err(|e| Error::Serialization {
                message: format!("Failed to parse Ollama response: {}", e),
                source: None,
            })?;

        // First message often contains model info
        if self.model.is_none() && !response.model.is_empty() {
            self.model = Some(response.model.clone());
            return Ok(Some(StreamEvent::Metadata(MetadataDelta {
                model: Some(response.model),
                id: None,
                custom: Default::default(),
            })));
        }

        // Handle content streaming
        if !response.message.content.is_empty() {
            return Ok(Some(StreamEvent::Content(ContentDelta {
                text: response.message.content,
            })));
        }

        // Handle tool calls - Ollama sends all tool calls in one message
        if let Some(tool_calls) = response.message.tool_calls {
            // Store all tool calls
            for (idx, call) in tool_calls.into_iter().enumerate() {
                let mut call_id = String::with_capacity(10);
                call_id.push_str("call_");
                call_id.push_str(&idx.to_string());
                self.current_tool_calls.push((
                    call_id,
                    call.function.name,
                    serde_json::to_string(&call.function.arguments).unwrap_or_default(),
                ));
            }

            // Return the first tool call if any
            if let Some((id, name, args)) = self.current_tool_calls.first() {
                return Ok(Some(StreamEvent::ToolCall(ToolCallDelta {
                    index: 0,
                    id: Some(id.clone()),
                    name: Some(name.clone()),
                    arguments: Some(args.clone()),
                })));
            }
        }

        // Handle completion
        if response.is_done {
            return Ok(Some(StreamEvent::Done));
        }

        Ok(None)
    }
}

impl Stream for OllamaStream {
    type Item = Result<StreamEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    // Convert bytes to string and append to buffer
                    let text = std::str::from_utf8(&chunk).map_err(|e| Error::Serialization {
                        message: format!("Invalid UTF-8 in response: {}", e),
                        source: None,
                    })?;
                    self.buffer.push_str(text);

                    // Process complete lines
                    while let Some(newline_pos) = self.buffer.find('\n') {
                        let line = self.buffer[..newline_pos].trim();
                        let line_owned = line.to_string();
                        self.buffer.drain(..=newline_pos);
                        let event_result = self.parse_line(&line_owned);

                        match event_result {
                            Ok(Some(event)) => return Poll::Ready(Some(Ok(event))),
                            Ok(None) => continue,
                            Err(e) => return Poll::Ready(Some(Err(e))),
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(Error::Network {
                        message: e.to_string(),
                        source: None,
                    })))
                }
                Poll::Ready(None) => {
                    // Process any remaining data in buffer
                    if !self.buffer.is_empty() {
                        let line = std::mem::take(&mut self.buffer);
                        match self.parse_line(&line) {
                            Ok(Some(event)) => return Poll::Ready(Some(Ok(event))),
                            Ok(None) => {}
                            Err(e) => return Poll::Ready(Some(Err(e))),
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
