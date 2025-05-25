//! Ollama provider implementation

use async_trait::async_trait;
use cogni_core::{Provider, Request, Response, Error};
use reqwest::Client;

use crate::ollama::{
    config::OllamaConfig,
    converter::{to_ollama_request, OllamaResponse},
    parser::parse_response,
    stream::OllamaStream,
};

/// Ollama provider implementation
#[derive(Debug, Clone)]
pub struct Ollama {
    config: OllamaConfig,
    client: Client,
}

impl Ollama {
    /// Create a new Ollama provider with the given configuration
    pub fn new(config: OllamaConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    /// Create a new Ollama provider with default local configuration
    pub fn local() -> Self {
        Self::new(OllamaConfig::default())
    }
    
    /// Create a new Ollama provider with a custom base URL
    pub fn with_base_url(base_url: String) -> Self {
        let config = OllamaConfig {
            base_url,
            ..Default::default()
        };
        Self::new(config)
    }
}

#[async_trait]
impl Provider for Ollama {
    type Stream = OllamaStream;
    
    async fn request(&self, request: Request) -> Result<Response, Error> {
            let mut ollama_request = to_ollama_request(&request);
            ollama_request.stream = Some(false);
            
            let response = self.client
                .post(format!("{}/api/chat", self.config.base_url))
                .json(&ollama_request)
                .send()
                .await
                .map_err(|e| Error::Network {
                    message: e.to_string(),
                    source: None,
                })?;
                
            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(Error::Provider {
                    provider: "ollama".to_string(),
                    message: error_text,
                    retry_after: None,
                    source: None,
                });
            }
            
            let ollama_response: OllamaResponse = response.json()
                .await
                .map_err(|e| Error::Serialization {
                    message: e.to_string(),
                    source: None,
                })?;
                
            parse_response(ollama_response)
    }
    
    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
            let mut ollama_request = to_ollama_request(&request);
            ollama_request.stream = Some(true);
            
            let response = self.client
                .post(format!("{}/api/chat", self.config.base_url))
                .json(&ollama_request)
                .send()
                .await
                .map_err(|e| Error::Network {
                    message: e.to_string(),
                    source: None,
                })?;
                
            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(Error::Provider {
                    provider: "ollama".to_string(),
                    message: error_text,
                    retry_after: None,
                    source: None,
                });
            }
            
            Ok(OllamaStream::new(response))
    }
}