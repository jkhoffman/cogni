//! Types for structured output functionality

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Trait for types that can be used as structured output from LLMs.
///
/// Implementing this trait allows a type to be used with provider-specific
/// structured output features (like OpenAI's JSON mode or Anthropic's structured output).
pub trait StructuredOutput: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Returns the JSON Schema for this type.
    ///
    /// This schema is used by providers to ensure the model's output
    /// conforms to the expected structure.
    fn schema() -> Value;

    /// Returns example instances of this type.
    ///
    /// Examples can help improve model performance by showing
    /// the expected format and values.
    fn examples() -> Vec<Self> {
        vec![]
    }
}

/// Format specification for structured responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ResponseFormat {
    /// Request a response matching a specific JSON Schema.
    JsonSchema {
        /// The JSON Schema that the response must conform to.
        schema: Value,
        /// Whether to enforce strict schema validation (provider-specific).
        strict: bool,
    },
    /// Request any valid JSON object response.
    #[default]
    JsonObject,
}
