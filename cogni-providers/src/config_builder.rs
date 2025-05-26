//! Configuration builders for provider configs

use crate::anthropic::AnthropicConfig;
use crate::constants::*;
use crate::ollama::OllamaConfig;

/// Builder for Ollama configuration
#[derive(Default)]
pub struct OllamaConfigBuilder {
    base_url: Option<String>,
    default_model: Option<String>,
}

impl OllamaConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> OllamaConfig {
        OllamaConfig {
            base_url: self
                .base_url
                .unwrap_or_else(|| OLLAMA_DEFAULT_BASE_URL.to_string()),
            default_model: self
                .default_model
                .unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string()),
        }
    }
}

/// Builder for Anthropic configuration
pub struct AnthropicConfigBuilder {
    api_key: String,
    base_url: Option<String>,
    default_model: Option<String>,
}

impl AnthropicConfigBuilder {
    /// Create a new builder with required API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            default_model: None,
        }
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> AnthropicConfig {
        AnthropicConfig {
            api_key: self.api_key,
            base_url: self
                .base_url
                .unwrap_or_else(|| ANTHROPIC_DEFAULT_BASE_URL.to_string()),
            default_model: self
                .default_model
                .unwrap_or_else(|| ANTHROPIC_DEFAULT_MODEL.to_string()),
        }
    }
}
