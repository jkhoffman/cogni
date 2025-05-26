//! Builder pattern for provider construction
//!
//! This module provides builder types for constructing providers with custom
//! configuration. The builders follow a fluent interface pattern where all
//! configuration methods return `self`, and `build()` is the terminal method
//! that constructs the final provider.
//!
//! # Examples
//!
//! ```no_run
//! use cogni_providers::builder::{OpenAIBuilder, ProviderBuilder};
//! use std::sync::Arc;
//!
//! // Basic usage
//! let provider = OpenAIBuilder::new("api-key")
//!     .build()
//!     .expect("Failed to build provider");
//!
//! // With all options
//! # let custom_client = Arc::new(cogni_providers::http::ReqwestClient::new().unwrap());
//! let provider = OpenAIBuilder::new("api-key")
//!     .base_url("https://custom.openai.azure.com")
//!     .organization("org-123")
//!     .default_model("gpt-4")
//!     .with_client(custom_client)
//!     .build()
//!     .expect("Failed to build provider");
//! ```

use crate::http::HttpClient;
use cogni_core::Error;
use std::sync::Arc;

/// Common builder trait for all providers
///
/// This trait defines the common interface for all provider builders,
/// ensuring consistent construction patterns across different providers.
pub trait ProviderBuilder: Sized {
    /// The provider type being built
    type Provider;

    /// Set a custom HTTP client
    ///
    /// This allows injecting a custom HTTP client implementation,
    /// useful for testing or special networking requirements.
    fn with_client(self, client: Arc<dyn HttpClient>) -> Self;

    /// Build the provider
    ///
    /// Consumes the builder and returns the configured provider,
    /// or an error if the configuration is invalid.
    fn build(self) -> Result<Self::Provider, Error>;
}

/// Builder for constructing OpenAI providers
///
/// This builder provides a fluent interface for configuring OpenAI providers
/// with support for custom endpoints (Azure OpenAI), organizations, and HTTP clients.
///
/// # Example
///
/// ```no_run
/// use cogni_providers::builder::OpenAIBuilder;
///
/// let provider = OpenAIBuilder::new("sk-...")
///     .organization("org-...")
///     .default_model("gpt-4-turbo-preview")
///     .build()
///     .expect("Failed to build OpenAI provider");
/// ```
pub struct OpenAIBuilder {
    api_key: String,
    base_url: Option<String>,
    organization: Option<String>,
    default_model: Option<String>,
    client: Option<Arc<dyn HttpClient>>,
}

impl OpenAIBuilder {
    /// Create a new OpenAI builder with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            organization: None,
            default_model: None,
            client: None,
        }
    }

    /// Set the base URL (for custom deployments)
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the organization ID
    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Set a custom HTTP client
    pub fn with_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the OpenAI provider
    pub fn build(self) -> Result<crate::OpenAI, Error> {
        use crate::http::ReqwestClient;
        use crate::openai::OpenAIConfig;

        let mut config = OpenAIConfig::new(self.api_key);

        if let Some(base_url) = self.base_url {
            config = config.with_base_url(base_url);
        }
        if let Some(org) = self.organization {
            config = config.with_organization(org);
        }
        // Note: default_model is stored in the builder but not used in OpenAIConfig
        // It could be used if OpenAI provider supported per-instance default models

        let client = self.client.unwrap_or_else(|| {
            Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"))
        });

        Ok(crate::OpenAI::new(config, client))
    }
}

impl ProviderBuilder for OpenAIBuilder {
    type Provider = crate::OpenAI;

    fn with_client(self, client: Arc<dyn HttpClient>) -> Self {
        Self {
            client: Some(client),
            ..self
        }
    }

    fn build(self) -> Result<Self::Provider, Error> {
        OpenAIBuilder::build(self)
    }
}

/// Builder for constructing Anthropic providers
///
/// This builder provides a fluent interface for configuring Anthropic providers
/// with support for custom endpoints, API versions, and HTTP clients.
///
/// # Example
///
/// ```no_run
/// use cogni_providers::builder::AnthropicBuilder;
///
/// let provider = AnthropicBuilder::new("sk-ant-...")
///     .default_model("claude-3-opus-20240229")
///     .build()
///     .expect("Failed to build Anthropic provider");
/// ```
pub struct AnthropicBuilder {
    api_key: String,
    base_url: Option<String>,
    version: Option<String>,
    default_model: Option<String>,
    client: Option<Arc<dyn HttpClient>>,
}

impl AnthropicBuilder {
    /// Create a new Anthropic builder with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            version: None,
            default_model: None,
            client: None,
        }
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the API version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Set a custom HTTP client
    pub fn with_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the Anthropic provider
    pub fn build(self) -> Result<crate::Anthropic, Error> {
        use crate::anthropic::AnthropicConfig;
        use crate::http::ReqwestClient;

        let config = AnthropicConfig {
            api_key: self.api_key,
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            default_model: self
                .default_model
                .unwrap_or_else(|| "claude-3-sonnet-20240229".to_string()),
        };

        let client = self.client.unwrap_or_else(|| {
            Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"))
        });

        Ok(crate::Anthropic::new(config, client))
    }
}

impl ProviderBuilder for AnthropicBuilder {
    type Provider = crate::Anthropic;

    fn with_client(self, client: Arc<dyn HttpClient>) -> Self {
        Self {
            client: Some(client),
            ..self
        }
    }

    fn build(self) -> Result<Self::Provider, Error> {
        AnthropicBuilder::build(self)
    }
}

/// Builder for constructing Ollama providers
///
/// This builder provides a fluent interface for configuring Ollama providers
/// with support for custom endpoints and HTTP clients. Since Ollama doesn't
/// require API keys, this builder can be used without authentication.
///
/// # Example
///
/// ```no_run
/// use cogni_providers::builder::{OllamaBuilder, ProviderBuilder};
///
/// // For local Ollama instance
/// let provider = OllamaBuilder::new()
///     .build()
///     .expect("Failed to build Ollama provider");
///
/// // For remote Ollama instance
/// let provider = OllamaBuilder::new()
///     .base_url("http://remote-server:11434")
///     .default_model("llama2")
///     .build()
///     .expect("Failed to build Ollama provider");
/// ```
pub struct OllamaBuilder {
    base_url: Option<String>,
    default_model: Option<String>,
    client: Option<Arc<dyn HttpClient>>,
}

impl OllamaBuilder {
    /// Create a new Ollama builder
    pub fn new() -> Self {
        Self {
            base_url: None,
            default_model: None,
            client: None,
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
}

impl ProviderBuilder for OllamaBuilder {
    type Provider = crate::Ollama;

    fn with_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.client = Some(client);
        self
    }

    fn build(self) -> Result<Self::Provider, Error> {
        use crate::http::ReqwestClient;
        use crate::ollama::OllamaConfig;

        let config = OllamaConfig {
            base_url: self
                .base_url
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            default_model: self.default_model.unwrap_or_else(|| "llama2".to_string()),
        };

        let client = self.client.unwrap_or_else(|| {
            Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"))
        });

        Ok(crate::Ollama::new(config, client))
    }
}

impl Default for OllamaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
