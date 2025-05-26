//! Message types for conversations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The role of a message in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Role {
    /// System message (instructions)
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool message (function result)
    Tool,
}

/// Content types that can be included in a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Content {
    /// Plain text content
    Text(String),
    /// Image content
    Image(Image),
    /// Audio content
    Audio(Audio),
    /// Multiple content items
    Multiple(Vec<Content>),
}

/// Image content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// Base64-encoded image data
    pub data: Option<String>,
    /// URL to the image
    pub url: Option<String>,
    /// MIME type (e.g., "image/png")
    pub mime_type: String,
}

/// Audio content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    /// Base64-encoded audio data
    pub data: String,
    /// MIME type (e.g., "audio/mp3")
    pub mime_type: String,
}

/// Metadata associated with a message
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// Arbitrary key-value pairs
    pub custom: HashMap<String, String>,
    /// Tool call ID if this is a tool response
    pub tool_call_id: Option<String>,
    /// Name override for the message
    pub name: Option<String>,
}

/// A message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender
    pub role: Role,
    /// The content of the message
    pub content: Content,
    /// Additional metadata
    pub metadata: Metadata,
}

impl Message {
    /// Create a simple text message
    pub fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: Content::Text(text.into()),
            metadata: Metadata::default(),
        }
    }

    /// Create a system message
    pub fn system(text: impl Into<String>) -> Self {
        Self::text(Role::System, text)
    }

    /// Create a user message
    pub fn user(text: impl Into<String>) -> Self {
        Self::text(Role::User, text)
    }

    /// Create an assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::text(Role::Assistant, text)
    }

    /// Create a tool message
    pub fn tool(text: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        let mut msg = Self::text(Role::Tool, text);
        msg.metadata.tool_call_id = Some(tool_call_id.into());
        msg
    }
}

impl Content {
    /// Get text content if this is a Text variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get image content if this is an Image variant
    pub fn as_image(&self) -> Option<&Image> {
        match self {
            Content::Image(img) => Some(img),
            _ => None,
        }
    }

    /// Get audio content if this is an Audio variant
    pub fn as_audio(&self) -> Option<&Audio> {
        match self {
            Content::Audio(audio) => Some(audio),
            _ => None,
        }
    }
}

// Conversion implementations
impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}
