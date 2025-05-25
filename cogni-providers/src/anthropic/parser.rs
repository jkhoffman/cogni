//! Anthropic response parsing

use cogni_core::{Response, Usage, ResponseMetadata, FinishReason, Error};
use crate::anthropic::converter::{AnthropicResponse, extract_text_content, extract_tool_calls};

pub fn parse_response(response: AnthropicResponse) -> Result<Response, Error> {
    let content = extract_text_content(&response);
    let tool_calls = extract_tool_calls(&response);
    
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