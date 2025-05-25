//! OpenAI provider configuration

/// Configuration for the OpenAI provider
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for the API
    pub base_url: String,
    /// Optional organization ID
    pub organization_id: Option<String>,
}

impl OpenAIConfig {
    /// Create a new configuration with an API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization_id: None,
        }
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the organization ID
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization_id = Some(org.into());
        self
    }

    /// Get the URL for chat completions
    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}
