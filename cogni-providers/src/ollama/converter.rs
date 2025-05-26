//! Conversion between Cogni types and Ollama API types

use crate::traits::RequestConverter;
use async_trait::async_trait;
use cogni_core::{Content, Error, Request, ResponseFormat, Role, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Ollama API request types
#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OllamaTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Serialize)]
pub struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct OllamaTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OllamaFunction,
}

#[derive(Debug, Serialize)]
pub struct OllamaFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaToolCall {
    pub function: OllamaFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaFunctionCall {
    pub name: String,
    pub arguments: Value,
}

// Ollama API response types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OllamaResponse {
    pub model: String,
    pub created_at: String,
    pub message: OllamaMessage,
    #[serde(rename = "done")]
    pub is_done: bool,
    #[serde(rename = "done_reason")]
    pub done_reason: Option<String>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

// Streaming response types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OllamaStreamResponse {
    pub model: String,
    pub created_at: String,
    #[serde(default)]
    pub message: OllamaStreamMessage,
    #[serde(rename = "done")]
    pub is_done: bool,
    #[serde(rename = "done_reason")]
    pub done_reason: Option<String>,
    // Metrics are only present in the final message
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct OllamaStreamMessage {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OllamaToolCall>>,
}

// Conversion functions
pub fn to_ollama_request(request: &Request) -> OllamaRequest {
    let messages: Vec<OllamaMessage> = request
        .messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
                _ => "user", // Default unknown roles to user
            }
            .to_string();

            let content = match &msg.content {
                Content::Text(text) => text.clone(),
                Content::Image(_) => "[Image content not yet supported]".to_string(),
                Content::Audio(_) => "[Audio content not yet supported]".to_string(),
                Content::Multiple(contents) => contents
                    .iter()
                    .filter_map(|c| match c {
                        Content::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            };

            OllamaMessage {
                role,
                content,
                tool_calls: None,
            }
        })
        .collect();

    let options = OllamaOptions {
        temperature: request.parameters.temperature,
        top_p: request.parameters.top_p,
        stop: request.parameters.stop.clone(),
        seed: request.parameters.seed,
    };

    let tools = if request.tools.is_empty() {
        None
    } else {
        Some(
            request
                .tools
                .iter()
                .map(|tool| OllamaTool {
                    tool_type: "function".to_string(),
                    function: OllamaFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.function.parameters.clone(),
                    },
                })
                .collect(),
        )
    };

    let format = request.response_format.as_ref().map(|format| match format {
        ResponseFormat::JsonSchema { schema, .. } => {
            // Ollama supports passing the schema directly as a JSON value
            schema.clone()
        }
        ResponseFormat::JsonObject => Value::String("json".to_string()),
    });

    OllamaRequest {
        model: request.model.to_string(),
        messages,
        stream: None, // Will be set by the provider
        options: Some(options),
        tools,
        format,
    }
}

pub fn extract_text_content(message: &OllamaMessage) -> String {
    message.content.clone()
}

pub fn extract_tool_calls(message: &OllamaMessage) -> Vec<ToolCall> {
    message
        .tool_calls
        .as_ref()
        .map(|calls| {
            calls
                .iter()
                .enumerate()
                .map(|(idx, call)| ToolCall {
                    id: format!("call_{}", idx),
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Converter implementation for Ollama
#[derive(Clone, Copy)]
pub struct OllamaConverter;

#[async_trait]
impl RequestConverter for OllamaConverter {
    async fn convert_request(&self, request: Request) -> Result<Value, Error> {
        let ollama_request = to_ollama_request(&request);
        serde_json::to_value(ollama_request).map_err(|e| Error::Serialization {
            message: e.to_string(),
            source: None,
        })
    }
}
