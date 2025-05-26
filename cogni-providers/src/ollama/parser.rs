//! Ollama response parsing

use crate::ollama::converter::{extract_text_content, extract_tool_calls, OllamaResponse};
use crate::traits::ResponseParser;
use async_trait::async_trait;
use cogni_core::{Error, FinishReason, Response, ResponseMetadata, Usage};
use serde_json::Value;

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
        (response.prompt_eval_count, response.eval_count)
    {
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

/// Parser implementation for Ollama
#[derive(Clone, Copy)]
pub struct OllamaParser;

#[async_trait]
impl ResponseParser for OllamaParser {
    async fn parse_response(&self, value: Value) -> Result<Response, Error> {
        let ollama_response: OllamaResponse =
            serde_json::from_value(value).map_err(|e| Error::Serialization {
                message: e.to_string(),
                source: None,
            })?;
        parse_response(ollama_response)
    }
}
