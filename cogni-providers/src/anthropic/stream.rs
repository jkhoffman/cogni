//! Anthropic streaming implementation

use std::pin::Pin;
use std::task::{Context, Poll};
use cogni_core::{StreamEvent, Error, ContentDelta, ToolCallDelta, MetadataDelta};
use futures_core::Stream;
use reqwest_eventsource::{Event, EventSource};
use crate::anthropic::converter::{AnthropicStreamEvent, ContentBlock, ContentDelta as AnthropicContentDelta};

pub struct AnthropicStream {
    inner: EventSource,
    current_tool_index: usize,
    current_tool_id: Option<String>,
    current_tool_name: Option<String>,
    current_tool_input: String,
    message_id: Option<String>,
    model: Option<String>,
}

impl AnthropicStream {
    pub fn new(event_source: EventSource) -> Self {
        Self {
            inner: event_source,
            current_tool_index: 0,
            current_tool_id: None,
            current_tool_name: None,
            current_tool_input: String::new(),
            message_id: None,
            model: None,
        }
    }
    
    fn parse_event(&mut self, data: &str) -> Result<Option<StreamEvent>, Error> {
        let event: AnthropicStreamEvent = serde_json::from_str(data)
            .map_err(|e| Error::Serialization { 
                message: e.to_string(),
                source: None,
            })?;
            
        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                self.message_id = Some(message.id.clone());
                self.model = Some(message.model.clone());
                
                // Send metadata event
                Ok(Some(StreamEvent::Metadata(MetadataDelta {
                    model: Some(message.model),
                    id: Some(message.id),
                    custom: Default::default(),
                })))
            }
            AnthropicStreamEvent::ContentBlockStart { index, content_block } => {
                match content_block {
                    ContentBlock::ToolUse { id, name, .. } => {
                        self.current_tool_index = index;
                        self.current_tool_id = Some(id.clone());
                        self.current_tool_name = Some(name.clone());
                        self.current_tool_input.clear();
                        
                        // Send initial tool call delta
                        Ok(Some(StreamEvent::ToolCall(ToolCallDelta {
                            index,
                            id: Some(id),
                            name: Some(name),
                            arguments: Some(String::new()),
                        })))
                    }
                    _ => Ok(None),
                }
            }
            AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
                match delta {
                    AnthropicContentDelta::TextDelta { text } => {
                        Ok(Some(StreamEvent::Content(ContentDelta {
                            text,
                        })))
                    }
                    AnthropicContentDelta::InputJsonDelta { partial_json } => {
                        self.current_tool_input.push_str(&partial_json);
                        
                        Ok(Some(StreamEvent::ToolCall(ToolCallDelta {
                            index,
                            id: None,
                            name: None,
                            arguments: Some(partial_json),
                        })))
                    }
                }
            }
            AnthropicStreamEvent::ContentBlockStop { index } => {
                // Verify we're stopping the correct tool
                if index == self.current_tool_index && self.current_tool_id.is_some() {
                    self.current_tool_id = None;
                    self.current_tool_name = None;
                    self.current_tool_input.clear();
                }
                Ok(None)
            }
            AnthropicStreamEvent::MessageDelta { delta } => {
                // Extract usage information if available
                if let Some(usage) = delta.usage {
                    let mut custom = std::collections::HashMap::new();
                    custom.insert("input_tokens".to_string(), usage.input_tokens.to_string());
                    custom.insert("output_tokens".to_string(), usage.output_tokens.to_string());
                    
                    Ok(Some(StreamEvent::Metadata(MetadataDelta {
                        model: None,
                        id: None,
                        custom,
                    })))
                } else {
                    Ok(None)
                }
            }
            AnthropicStreamEvent::MessageStop => {
                Ok(Some(StreamEvent::Done))
            }
            AnthropicStreamEvent::Ping => Ok(None),
        }
    }
}

impl Stream for AnthropicStream {
    type Item = Result<StreamEvent, Error>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(Event::Open))) => continue,
                Poll::Ready(Some(Ok(Event::Message(msg)))) => {
                    if msg.event == "error" {
                        return Poll::Ready(Some(Err(Error::Provider {
                            provider: "anthropic".to_string(),
                            message: msg.data,
                            retry_after: None,
                            source: None,
                        })));
                    }
                    
                    match self.parse_event(&msg.data) {
                        Ok(Some(event)) => return Poll::Ready(Some(Ok(event))),
                        Ok(None) => continue,
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(Error::Network {
                        message: e.to_string(),
                        source: None,
                    })))
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}