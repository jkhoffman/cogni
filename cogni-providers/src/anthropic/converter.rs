//! Conversion between Cogni types and Anthropic API types

use cogni_core::{Content, Request, ResponseFormat, Role, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Anthropic API request types
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Anthropic API response types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Streaming response types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicStreamMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AnthropicStreamMessage {
    pub id: String,
    pub model: String,
    pub role: String,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub usage: Option<AnthropicUsage>,
}

// Conversion functions
pub fn to_anthropic_request(request: &Request) -> AnthropicRequest {
    let mut messages = Vec::new();
    let mut system_message = None;

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                // Anthropic uses a separate system parameter
                if let Content::Text(text) = &msg.content {
                    system_message = Some(text.clone());
                }
            }
            Role::User => {
                messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: convert_content(&msg.content),
                });
            }
            Role::Assistant => {
                messages.push(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: convert_content(&msg.content),
                });
            }
            Role::Tool => {
                // Tool responses are handled via content blocks
                if let Some(tool_call_id) = &msg.metadata.tool_call_id {
                    if let Content::Text(text) = &msg.content {
                        messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: tool_call_id.clone(),
                                content: text.clone(),
                            }]),
                        });
                    }
                }
            }
            _ => {
                // Unknown role - skip this message
                continue;
            }
        }
    }

    let tools = if request.tools.is_empty() {
        None
    } else {
        Some(
            request
                .tools
                .iter()
                .map(|tool| AnthropicTool {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.function.parameters.clone(),
                })
                .collect(),
        )
    };

    let response_format = request.response_format.as_ref().map(|format| match format {
        ResponseFormat::JsonSchema { schema, strict } => {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "strict": strict,
                    "schema": schema
                }
            })
        }
        ResponseFormat::JsonObject => {
            serde_json::json!({ "type": "json_object" })
        }
    });

    AnthropicRequest {
        model: request.model.to_string(),
        messages,
        max_tokens: request.parameters.max_tokens.unwrap_or(4096),
        temperature: request.parameters.temperature,
        stream: None, // Will be set by the provider
        tools,
        system: system_message,
        response_format,
    }
}

fn convert_content(content: &Content) -> AnthropicContent {
    match content {
        Content::Text(text) => AnthropicContent::Text(text.clone()),
        Content::Image(_) => {
            // TODO: Implement image support for Anthropic
            AnthropicContent::Text("[Image content not yet supported]".to_string())
        }
        Content::Audio(_) => {
            // TODO: Implement audio support for Anthropic
            AnthropicContent::Text("[Audio content not yet supported]".to_string())
        }
        Content::Multiple(contents) => {
            // Convert multiple contents to text for now
            let text = contents
                .iter()
                .filter_map(|c| match c {
                    Content::Text(t) => Some(t.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            AnthropicContent::Text(text)
        }
    }
}

pub fn extract_text_content(response: &AnthropicResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn extract_tool_calls(response: &AnthropicResponse) -> Vec<ToolCall> {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(ToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: input.to_string(),
            }),
            _ => None,
        })
        .collect()
}
