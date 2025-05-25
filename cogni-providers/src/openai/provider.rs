//! OpenAI provider implementation

use crate::http::{create_headers, HttpClient, ReqwestClient};
use crate::openai::{config::OpenAIConfig, converter::OpenAIConverter, parser::OpenAIParser, stream::OpenAIStream};
use crate::traits::{RequestConverter, ResponseParser};
use async_trait::async_trait;
use cogni_core::{Error, Provider, Request, Response};
use std::sync::Arc;
use reqwest_eventsource::EventSource;

/// OpenAI provider
pub struct OpenAI {
    client: Arc<dyn HttpClient>,
    config: OpenAIConfig,
    converter: OpenAIConverter,
    parser: OpenAIParser,
}

impl OpenAI {
    /// Create a new OpenAI provider with the given configuration
    pub fn new(config: OpenAIConfig) -> Result<Self, Error> {
        let client = Arc::new(ReqwestClient::new()?);
        Ok(Self {
            client,
            config,
            converter: OpenAIConverter,
            parser: OpenAIParser,
        })
    }
    
    /// Create a new OpenAI provider with just an API key
    pub fn with_api_key(api_key: String) -> Self {
        let config = OpenAIConfig::new(api_key);
        let client = Arc::new(ReqwestClient::new().expect("Failed to create HTTP client"));
        Self {
            client,
            config,
            converter: OpenAIConverter,
            parser: OpenAIParser,
        }
    }
    
    /// Create with a custom HTTP client
    pub fn with_client(config: OpenAIConfig, client: Arc<dyn HttpClient>) -> Self {
        Self {
            client,
            config,
            converter: OpenAIConverter,
            parser: OpenAIParser,
        }
    }
}

#[async_trait]
impl Provider for OpenAI {
    type Stream = OpenAIStream;
    
    async fn request(&self, request: Request) -> Result<Response, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(false);
        
        let headers = create_headers(&self.config.api_key, None)?;
        let response = self.client.post(&self.config.chat_url(), headers, body).await?;
        
        self.parser.parse_response(response).await
    }
    
    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let mut body = self.converter.convert_request(request).await?;
        body["stream"] = serde_json::json!(true);
        
        let headers = create_headers(&self.config.api_key, None)?;
        
        // Create a reqwest request
        let client = reqwest::Client::new();
        let mut req = client.post(self.config.chat_url());
        
        // Add headers
        for (key, value) in headers.iter() {
            req = req.header(key, value);
        }
        
        // Create EventSource
        let event_source = EventSource::new(req.json(&body))
            .map_err(|e| Error::Network {
                message: format!("Failed to create event source: {}", e),
                source: None,
            })?;
        
        Ok(OpenAIStream::new(event_source))
    }
}