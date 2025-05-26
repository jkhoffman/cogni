//! Anthropic response parsing

use crate::anthropic::converter::{extract_text_content, extract_tool_calls, AnthropicResponse};
use crate::traits::ResponseParser;
use async_trait::async_trait;
use cogni_core::{Error, FinishReason, Response, ResponseMetadata, Usage};
use serde_json::Value;

pub fn parse_response(response: AnthropicResponse) -> Result<Response, Error> {
    let mut content = extract_text_content(&response);
    let mut tool_calls = extract_tool_calls(&response);

    // Check if this is a structured output response
    if let Some(structured_idx) = tool_calls
        .iter()
        .position(|tc| tc.name == "structured_output" || tc.name == "json_output")
    {
        // Extract the structured output tool call
        let structured_call = tool_calls.remove(structured_idx);

        // Use the tool's arguments as the content
        // If there's already text content, append the JSON
        if content.is_empty() {
            content = structured_call.arguments;
        } else {
            content = format!("{}\n{}", content, structured_call.arguments);
        }
    }

    let usage = response.usage.as_ref().map(parse_usage);
    let finish_reason = if !tool_calls.is_empty() {
        Some(FinishReason::ToolCalls)
    } else {
        Some(FinishReason::Stop)
    };

    let metadata = ResponseMetadata {
        model: Some(response.model),
        id: Some(response.id),
        usage,
        finish_reason,
        ..Default::default()
    };

    Ok(Response {
        content,
        tool_calls,
        metadata,
    })
}

pub fn parse_usage(usage: &crate::anthropic::converter::AnthropicUsage) -> Usage {
    Usage {
        prompt_tokens: usage.input_tokens,
        completion_tokens: usage.output_tokens,
        total_tokens: usage.input_tokens + usage.output_tokens,
    }
}

/// Parser implementation for Anthropic
#[derive(Clone, Copy)]
pub struct AnthropicParser;

#[async_trait]
impl ResponseParser for AnthropicParser {
    async fn parse_response(&self, value: Value) -> Result<Response, Error> {
        let anthropic_response: AnthropicResponse =
            serde_json::from_value(value).map_err(|e| Error::Serialization {
                message: e.to_string(),
                source: None,
            })?;
        parse_response(anthropic_response)
    }
}
