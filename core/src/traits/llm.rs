//! Language model interface for the Cogni framework.
//!
//! This module defines the core traits and types for interacting with
//! language models in a consistent way across different providers.

use async_trait::async_trait;
use futures::Stream;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use std::pin::Pin;
use std::time::Duration;

use crate::error::LlmError;

/// Options for generating text from a language model.
#[derive(Debug, Default, Clone)]
pub struct GenerateOptions {
    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: Option<f32>,

    /// Timeout for the generation request
    pub timeout: Option<Duration>,
}

/// A trait representing a language model that can generate text.
///
/// This trait defines the core interface for language models in the Cogni framework.
/// Implementations of this trait can support different types of prompts and responses,
/// allowing for flexibility in how different models handle input and output.
///
/// # Type Parameters
///
/// * `Prompt` - The type of prompt accepted by this model
/// * `Response` - The type of response returned by this model
/// * `TokenStream` - The type of token stream returned by this model
///
/// # Examples
///
/// ```rust,no_run
/// use cogni_core::traits::llm::{LanguageModel, GenerateOptions};
/// use async_trait::async_trait;
/// use std::pin::Pin;
///
/// struct MyModel;
///
/// #[async_trait]
/// impl LanguageModel for MyModel {
///     type Prompt = String;
///     type Response = String;
///     type TokenStream = futures::stream::Empty<Result<String, cogni_core::error::LlmError>>;
///
///     async fn generate(
///         &self,
///         prompt: Self::Prompt,
///         _opts: GenerateOptions,
///     ) -> Result<Self::Response, cogni_core::error::LlmError> {
///         Ok(format!("Response to: {}", prompt))
///     }
///
///     async fn stream_generate(
///         &self,
///         _prompt: Self::Prompt,
///         _opts: GenerateOptions,
///     ) -> Result<Pin<Box<Self::TokenStream>>, cogni_core::error::LlmError> {
///         Ok(Box::pin(futures::stream::empty()))
///     }
///
///     fn name(&self) -> &'static str {
///         "my_model"
///     }
/// }
/// ```
#[async_trait]
pub trait LanguageModel: Send + Sync + 'static {
    /// The type of prompt accepted by this model
    type Prompt: Serialize + Send + Sync;

    /// The type of response returned by this model
    type Response: DeserializeOwned + Send + Sync;

    /// The type of token stream returned by this model
    type TokenStream: Stream<Item = Result<String, LlmError>> + Send + 'static;

    /// Generate text from a prompt.
    ///
    /// This method takes a prompt and generation options and returns a complete response.
    /// For streaming responses, use [`stream_generate`].
    async fn generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError>;

    /// Generate a stream of tokens from a prompt.
    ///
    /// This method is similar to [`generate`], but returns a stream of tokens
    /// that can be processed as they arrive, rather than waiting for the complete response.
    async fn stream_generate(
        &self,
        prompt: Self::Prompt,
        opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError>;

    /// Get the name of this model.
    ///
    /// This should return a unique identifier for the model implementation.
    fn name(&self) -> &'static str;
}
