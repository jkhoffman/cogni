//! Ollama provider configuration

use crate::constants::{OLLAMA_DEFAULT_BASE_URL, OLLAMA_DEFAULT_MODEL};

/// Configuration for the Ollama provider
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Base URL for the Ollama API
    pub base_url: String,
    /// Default model to use if not specified in requests
    pub default_model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: OLLAMA_DEFAULT_BASE_URL.to_string(),
            default_model: OLLAMA_DEFAULT_MODEL.to_string(),
        }
    }
}

impl OllamaConfig {
    /// Create a new configuration builder
    pub fn builder() -> crate::config_builder::OllamaConfigBuilder {
        crate::config_builder::OllamaConfigBuilder::new()
    }
}
