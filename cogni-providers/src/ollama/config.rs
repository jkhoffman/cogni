//! Ollama provider configuration

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
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama3.2".to_string(),
        }
    }
}
