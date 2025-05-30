//! Core traits and types for the Cogni LLM library
//!
//! This crate provides the fundamental abstractions used throughout the Cogni ecosystem.
//! It has zero external dependencies, relying only on the Rust standard library.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;
pub mod provider;
pub mod types;

// Re-export commonly used items
pub use error::{Error, Result};
pub use provider::Provider;
pub use types::{
    message::{Audio, Content, Image, Message, Metadata, Role},
    request::{Model, Parameters, ParametersBuilder, Request, RequestBuilder},
    response::{FinishReason, Response, ResponseMetadata, Usage},
    stream::{ContentDelta, MetadataDelta, StreamAccumulator, StreamEvent, ToolCallDelta},
    structured::{ResponseFormat, StructuredOutput},
    tool::{Function, Tool, ToolCall, ToolChoice, ToolResult},
};

// Re-export derive macro when feature is enabled
#[cfg(feature = "derive")]
pub use cogni_derive::StructuredOutput as DeriveStructuredOutput;
