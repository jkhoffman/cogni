//! Request conversion for OpenAI

use crate::traits::RequestConverter;
use async_trait::async_trait;
use cogni_core::{Content, Error, Message, Request, Role};
use serde_json::{json, Value};

/// Converts generic requests to OpenAI format
pub struct OpenAIConverter;

#[async_trait]
impl RequestConverter for OpenAIConverter {
    async fn convert_request(&self, request: Request) -> Result<Value, Error> {
        let mut body = json!({
            "model": request.model.to_string(),
            "messages": self.convert_messages(&request.messages)?,
            "stream": false,
        });
        
        // Add parameters
        if let Some(max_tokens) = request.parameters.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = request.parameters.temperature {
            body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = request.parameters.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(n) = request.parameters.n {
            body["n"] = json!(n);
        }
        if let Some(stop) = &request.parameters.stop {
            body["stop"] = json!(stop);
        }
        if let Some(presence_penalty) = request.parameters.presence_penalty {
            body["presence_penalty"] = json!(presence_penalty);
        }
        if let Some(frequency_penalty) = request.parameters.frequency_penalty {
            body["frequency_penalty"] = json!(frequency_penalty);
        }
        if let Some(seed) = request.parameters.seed {
            body["seed"] = json!(seed);
        }
        
        // Add tools if present
        if !request.tools.is_empty() {
            body["tools"] = json!(self.convert_tools(&request.tools));
        }
        
        Ok(body)
    }
}

impl OpenAIConverter {
    fn convert_messages(&self, messages: &[Message]) -> Result<Vec<Value>, Error> {
        messages
            .iter()
            .map(|msg| self.convert_message(msg))
            .collect()
    }
    
    fn convert_message(&self, message: &Message) -> Result<Value, Error> {
        let role = match message.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            _ => "user", // Default unknown roles to user
        };
        
        let mut msg = json!({
            "role": role,
        });
        
        // Handle content
        match &message.content {
            Content::Text(text) => {
                msg["content"] = json!(text);
            }
            Content::Multiple(contents) => {
                let converted: Result<Vec<Value>, Error> = contents
                    .iter()
                    .map(|c| self.convert_content(c))
                    .collect();
                msg["content"] = json!(converted?);
            }
            other => {
                msg["content"] = json!([self.convert_content(other)?]);
            }
        }
        
        // Add metadata
        if let Some(name) = &message.metadata.name {
            msg["name"] = json!(name);
        }
        if let Some(tool_call_id) = &message.metadata.tool_call_id {
            msg["tool_call_id"] = json!(tool_call_id);
        }
        
        Ok(msg)
    }
    
    fn convert_content(&self, content: &Content) -> Result<Value, Error> {
        match content {
            Content::Text(text) => Ok(json!({
                "type": "text",
                "text": text,
            })),
            Content::Image(image) => {
                if let Some(url) = &image.url {
                    Ok(json!({
                        "type": "image_url",
                        "image_url": {
                            "url": url,
                        },
                    }))
                } else if let Some(data) = &image.data {
                    Ok(json!({
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", image.mime_type, data),
                        },
                    }))
                } else {
                    Err(Error::Validation("Image must have either URL or data".to_string()))
                }
            }
            Content::Audio(_) => {
                Err(Error::Validation("OpenAI does not support audio content in chat".to_string()))
            }
            Content::Multiple(_) => {
                Err(Error::Validation("Nested multiple content not supported".to_string()))
            }
        }
    }
    
    fn convert_tools(&self, tools: &[cogni_core::Tool]) -> Vec<Value> {
        tools
            .iter()
            .map(|tool| json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.function.parameters.clone(),
                },
            }))
            .collect()
    }
}