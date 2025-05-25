//! Ollama response parsing

use cogni_core::{Response, Usage, ResponseMetadata, FinishReason, Error};
use crate::ollama::converter::{OllamaResponse, extract_text_content, extract_tool_calls};

pub fn parse_response(response: OllamaResponse) -> Result<Response, Error> {
    let content = extract_text_content(&response.message);
    let tool_calls = extract_tool_calls(&response.message);
    
    let mut custom = std::collections::HashMap::new();
    
    // Add timing information to custom metadata
    if let Some(total_duration) = response.total_duration {
        custom.insert("total_duration_ns".to_string(), total_duration.to_string());
    }
    if let Some(load_duration) = response.load_duration {
        custom.insert("load_duration_ns".to_string(), load_duration.to_string());
    }
    if let Some(eval_duration) = response.eval_duration {
        custom.insert("eval_duration_ns".to_string(), eval_duration.to_string());
    }
    
    let mut metadata = ResponseMetadata {
        model: Some(response.model),
        custom,
        ..Default::default()
    };
    
    // Calculate usage from Ollama metrics
    if let (Some(prompt_tokens), Some(completion_tokens)) = 
        (response.prompt_eval_count, response.eval_count) {
        metadata.usage = Some(Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        });
    }
    
    // Determine finish reason
    let finish_reason = match response.done_reason.as_deref() {
        Some("stop") => Some(FinishReason::Stop),
        Some("length") => Some(FinishReason::Length),
        _ => {
            if !tool_calls.is_empty() {
                Some(FinishReason::ToolCalls)
            } else {
                Some(FinishReason::Stop)
            }
        }
    };
    metadata.finish_reason = finish_reason;
    
    Ok(Response {
        content,
        tool_calls,
        metadata,
    })
}