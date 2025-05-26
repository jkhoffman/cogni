//! Anthropic provider configuration

use crate::constants::{ANTHROPIC_DEFAULT_BASE_URL, ANTHROPIC_DEFAULT_MODEL};

/// Configuration for the Anthropic provider
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for the Anthropic API
    pub base_url: String,
    /// Default model to use if not specified in requests
    pub default_model: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            base_url: ANTHROPIC_DEFAULT_BASE_URL.to_string(),
            default_model: ANTHROPIC_DEFAULT_MODEL.to_string(),
        }
    }
}

impl AnthropicConfig {
    /// Create a new configuration with the given API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Create a new configuration builder
    pub fn builder(api_key: impl Into<String>) -> crate::config_builder::AnthropicConfigBuilder {
        crate::config_builder::AnthropicConfigBuilder::new(api_key)
    }

    /// Set the base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}
