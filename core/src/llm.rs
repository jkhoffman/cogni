//! Language model interface for the Cogni framework.

use async_trait::async_trait;
use futures::Stream;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use std::pin::Pin;

use crate::error::LlmError;

/// Options for generating text from a language model.
#[derive(Debug, Default, Clone)]
pub struct GenerateOptions {
    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: Option<f32>,

    /// Timeout for the generation request
    pub timeout: Option<std::time::Duration>,
}

/// A trait representing a language model that can generate text.
#[async_trait]
pub trait LanguageModel: Send + Sync + 'static {
    /// The type of prompt accepted by this model
    type Prompt: Serialize + Send + Sync;

    /// The type of response returned by this model
    type Response: DeserializeOwned + Send + Sync;

    /// The type of token stream returned by this model
    type TokenStream: Stream<Item = Result<String, LlmError>> + Send + 'static;

    /// Generate text from a prompt
    async fn generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError>;

    /// Generate a stream of tokens from a prompt
    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError>;

    /// Get the name of this model
    fn name(&self) -> &'static str;
}
