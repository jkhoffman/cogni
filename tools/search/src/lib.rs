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
use cogni_tools_common::{CacheConfig, RateLimiterConfig, ToolCache, ToolRateLimiter};
use hex;
use log::warn;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
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
    cache: Arc<ToolCache>,
    rate_limiter: Arc<ToolRateLimiter>,
}

impl SearchTool {
    /// Create a new search tool with the given configuration.
    pub fn new(config: SearchConfig) -> Self {
        let cache = Arc::new(ToolCache::new(CacheConfig {
            ttl_secs: config.cache_duration,
            ..CacheConfig::default()
        }));
        let rate_limiter = Arc::new(ToolRateLimiter::new(RateLimiterConfig {
            global_rps: config.rate_limit,
            ..RateLimiterConfig::default()
        }));
        Self {
            _config: config,
            client: None,
            cache,
            rate_limiter,
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    type Input = SearchInput;
    type Output = SearchOutput;
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
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
        use cogni_tools_common::http::HttpClient;
        use serde_json::Value;

        // Cache key: hash of query+max_results
        let mut hasher = Sha256::new();
        hasher.update(&input.query);
        if let Some(max) = input.max_results {
            hasher.update(max.to_le_bytes());
        }
        let cache_key = format!("search:{}", hex::encode(hasher.finalize()));

        // Check cache
        if let Ok(Some(cached)) = self.cache.get_json::<SearchOutput>(&cache_key) {
            return Ok(cached);
        }

        // Rate limiting
        self.rate_limiter
            .check_url(&self._config.base_url)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("SearchTool", "rate_limit"),
                message: format!("Rate limit error: {e}"),
                retryable: true,
            })?;

        let client = HttpClient::with_default_config().map_err(|e| ToolError::ExecutionFailed {
            context: cogni_core::error::ErrorContext::new("SearchTool", "http_client_init"),
            message: format!("Failed to create HttpClient: {e}"),
            retryable: false,
        })?;

        let mut params = vec![
            ("q", input.query.as_str()),
            ("api_key", self._config.api_key.as_str()),
        ];
        let max_results_str;
        if let Some(max) = input.max_results {
            max_results_str = max.to_string();
            params.push(("num", max_results_str.as_str()));
        }
        let query_string = serde_urlencoded::to_string(&params).unwrap();
        let url = format!("{}?{}", self._config.base_url, query_string);

        let resp: Value =
            client
                .get_json(&url, None)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    context: cogni_core::error::ErrorContext::new("SearchTool", "http_get_json"),
                    message: format!("HTTP error: {e}"),
                    retryable: true,
                })?;

        // Parse SerpAPI response (Google-style)
        let mut results = Vec::new();
        if let Some(arr) = resp.get("organic_results").and_then(|v| v.as_array()) {
            for item in arr {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let url = item
                    .get("link")
                    .or_else(|| item.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let snippet = item
                    .get("snippet")
                    .or_else(|| item.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !title.is_empty() && !url.is_empty() {
                    results.push(SearchResult {
                        title,
                        url,
                        snippet,
                    });
                }
            }
        }
        let output = SearchOutput { results };
        // Store in cache
        let _ = self.cache.set_json(&cache_key, &output);
        Ok(output)
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
