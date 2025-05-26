//! Response parsing for OpenAI

use crate::error;
use crate::traits::{ResponseParser, StreamEventParser};
use async_trait::async_trait;
use cogni_core::{
    ContentDelta, Error, FinishReason, MetadataDelta, Response, ResponseMetadata, StreamEvent,
    ToolCall, ToolCallDelta, Usage,
};
use serde::Deserialize;
use serde_json::Value;

/// Parses OpenAI responses
#[derive(Clone, Copy)]
pub struct OpenAIParser;

#[async_trait]
impl ResponseParser for OpenAIParser {
    async fn parse_response(&self, value: Value) -> Result<Response, Error> {
        let response: OpenAIResponse =
            serde_json::from_value(value).map_err(error::serialization_error)?;

        if let Some(choice) = response.choices.first() {
            let content = choice.message.content.clone().unwrap_or_default();

            let tool_calls = choice
                .message
                .tool_calls
                .as_ref()
                .map(|calls| {
                    calls
                        .iter()
                        .map(|tc| ToolCall {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let metadata = ResponseMetadata {
                model: Some(response.model),
                id: Some(response.id),
                usage: response.usage.map(|u| Usage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
                finish_reason: choice.finish_reason.as_deref().map(parse_finish_reason),
                ..Default::default()
            };

            Ok(Response {
                content,
                tool_calls,
                metadata,
            })
        } else {
            Err(Error::Provider {
                provider: "openai".to_string(),
                message: "No choices in response".to_string(),
                retry_after: None,
                source: None,
            })
        }
    }
}

impl StreamEventParser for OpenAIParser {
    fn parse_event(&self, data: &str) -> Result<Option<StreamEvent>, Error> {
        if let Some(json_str) = data.strip_prefix("data: ") {
            if json_str == "[DONE]" {
                return Ok(Some(StreamEvent::Done));
            }

            let chunk: StreamChunk =
                serde_json::from_str(json_str).map_err(error::serialization_error)?;

            if let Some(choice) = chunk.choices.first() {
                // Content delta
                if let Some(content) = &choice.delta.content {
                    return Ok(Some(StreamEvent::Content(ContentDelta {
                        text: content.clone(),
                    })));
                }

                // Tool call deltas - OpenAI streams one at a time
                if let Some(tool_calls) = &choice.delta.tool_calls {
                    if let Some(tc) = tool_calls.first() {
                        return Ok(Some(StreamEvent::ToolCall(ToolCallDelta {
                            index: tc.index,
                            id: tc.id.clone(),
                            name: tc.function.as_ref().and_then(|f| f.name.clone()),
                            arguments: tc.function.as_ref().and_then(|f| f.arguments.clone()),
                        })));
                    }
                }
            }

            // Metadata
            if !chunk.id.is_empty() || !chunk.model.is_empty() {
                return Ok(Some(StreamEvent::Metadata(MetadataDelta {
                    model: Some(chunk.model),
                    id: Some(chunk.id),
                    ..Default::default()
                })));
            }
        }

        Ok(None)
    }
}

fn parse_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" | "function_call" => FinishReason::ToolCalls,
        "content_filter" => FinishReason::ContentFilter,
        _ => FinishReason::Stop,
    }
}

// Response structures
#[derive(Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<Choice>,
    usage: Option<UsageInfo>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageResponse,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct MessageResponse {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallResponse>>,
}

#[derive(Deserialize)]
struct ToolCallResponse {
    id: String,
    function: FunctionCall,
}

#[derive(Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct UsageInfo {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// Streaming structures
#[derive(Deserialize)]
struct StreamChunk {
    id: String,
    model: String,
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: DeltaContent,
}

#[derive(Deserialize)]
struct DeltaContent {
    content: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Deserialize)]
struct StreamToolCall {
    index: usize,
    id: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Deserialize)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}
