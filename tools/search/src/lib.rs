//! Search tool for the Cogni framework.
//!
//! This crate provides a search tool implementation using various search APIs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::{
    error::ToolError,
    tool::{Tool, ToolSpec},
};
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
    api_key: String,
    /// Base URL for the API
    base_url: String,
}

impl SearchConfig {
    /// Create a new configuration with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://serpapi.com/search".to_string(),
        }
    }

    /// Set a custom base URL for the API.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

/// The search tool.
pub struct SearchTool {
    config: SearchConfig,
    client: reqwest::Client,
}

impl SearchTool {
    /// Create a new search tool with the given configuration.
    pub fn new(config: SearchConfig) -> Result<Self, ToolError> {
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }
}

#[async_trait]
impl Tool for SearchTool {
    type Input = SearchInput;
    type Output = SearchOutput;

    #[instrument(skip(self, input))]
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
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

    #[tokio::test]
    async fn test_tool_creation() {
        let config = SearchConfig::new("test_key");
        let tool = SearchTool::new(config).unwrap();

        // Test will be expanded when invoke is implemented
    }
}
