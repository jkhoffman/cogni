//! Anthropic provider configuration

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
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-3-5-sonnet-latest".to_string(),
        }
    }
}