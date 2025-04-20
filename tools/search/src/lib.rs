//! Search tool for the Cogni framework.
//!
//! This crate provides a search tool implementation using various search APIs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use async_trait::async_trait;
use cogni_core::{
    error::{ToolConfigError, ToolError},
    traits::tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
};
use log::warn;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Input for the search tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInput {
    /// The search query
    pub query: String,
    /// Maximum number of results to return
    pub max_results: Option<usize>,
}

/// A search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The title of the result
    pub title: String,
    /// The URL of the result
    pub url: String,
    /// A snippet or description of the result
    pub snippet: String,
}

/// Output from the search tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOutput {
    /// The search results
    pub results: Vec<SearchResult>,
}

/// Configuration for the search tool.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// The API key for the search service
    pub api_key: String,
    /// Base URL for the API
    pub base_url: String,
    /// Rate limit in requests per second
    pub rate_limit: f64,
    /// Cache duration in seconds
    pub cache_duration: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://serpapi.com/search".to_string(),
            rate_limit: 10.0,
            cache_duration: 3600,
        }
    }
}

impl ToolConfig for SearchConfig {
    fn validate(&self) -> Result<(), ToolConfigError> {
        if self.api_key.is_empty() {
            return Err(ToolConfigError::MissingField {
                field_name: "api_key".into(),
            });
        }
        if self.base_url.is_empty() {
            return Err(ToolConfigError::MissingField {
                field_name: "base_url".into(),
            });
        }
        if !self.base_url.starts_with("http") {
            return Err(ToolConfigError::InvalidValue {
                field_name: "base_url".into(),
                message: "base_url must be a valid HTTP(S) URL".into(),
            });
        }
        if self.rate_limit <= 0.0 {
            return Err(ToolConfigError::InvalidValue {
                field_name: "rate_limit".into(),
                message: "rate_limit must be greater than 0".into(),
            });
        }
        if self.cache_duration == 0 {
            return Err(ToolConfigError::InvalidValue {
                field_name: "cache_duration".into(),
                message: "cache_duration must be greater than 0".into(),
            });
        }
        Ok(())
    }
}

/// The search tool.
pub struct SearchTool {
    _config: SearchConfig,
    client: Option<reqwest::Client>,
}

impl SearchTool {
    /// Create a new search tool with the given configuration.
    pub fn new(config: SearchConfig) -> Self {
        Self {
            _config: config,
            client: None,
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    type Input = String;
    type Output = String;
    type Config = SearchConfig;

    fn try_new(config: Self::Config) -> Result<Self, ToolConfigError> {
        config.validate()?;
        Ok(Self::new(config))
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        self.client = Some(reqwest::Client::new());
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        self.client = None;
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ThreadSafe, ToolCapability::NetworkAccess]
    }

    #[instrument(skip_all)]
    async fn invoke(&self, _input: Self::Input) -> Result<Self::Output, ToolError> {
        todo!("Implement search tool")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "search".to_string(),
            description: "Search the web for information".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["query"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "results": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "The title of the result"
                                },
                                "url": {
                                    "type": "string",
                                    "description": "The URL of the result"
                                },
                                "snippet": {
                                    "type": "string",
                                    "description": "A snippet or description of the result"
                                }
                            },
                            "required": ["title", "url", "snippet"]
                        }
                    }
                },
                "required": ["results"]
            }),
            examples: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SearchConfig {
        SearchConfig {
            api_key: "test_key".to_string(),
            base_url: "https://api.search.test".to_string(),
            rate_limit: 10.0,
            cache_duration: 3600,
        }
    }

    #[tokio::test]
    async fn test_tool_creation() {
        let config = create_test_config();
        let mut tool = SearchTool::new(config);
        assert!(tool.initialize().await.is_ok());
        assert!(tool.shutdown().await.is_ok());
    }

    #[test]
    fn test_config_validation() {
        let valid_config = create_test_config();
        assert!(valid_config.validate().is_ok());

        let invalid_config = SearchConfig {
            api_key: "".to_string(),
            ..create_test_config()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = SearchConfig {
            base_url: "".to_string(),
            ..create_test_config()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = SearchConfig {
            base_url: "invalid_url".to_string(),
            ..create_test_config()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = SearchConfig {
            rate_limit: 0.0,
            ..create_test_config()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = SearchConfig {
            cache_duration: 0,
            ..create_test_config()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_capabilities() {
        let tool = SearchTool::new(create_test_config());
        let capabilities = tool.capabilities();
        assert!(capabilities.contains(&ToolCapability::ThreadSafe));
        assert!(capabilities.contains(&ToolCapability::NetworkAccess));
    }
}
