//! HTTP client utilities for tools.
//!
//! This module provides a shared HTTP client implementation with:
//! - Retries with exponential backoff
//! - Rate limiting
//! - Request/response logging
//! - Error handling
//! - Cookie management

use std::{fmt::Debug, sync::Arc, time::Duration};

use anyhow::Result;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use bytes::Bytes;
use cogni_core::error::ToolError;
use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, ClientBuilder, Method, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, trace, warn};
use url::Url;

/// Default timeout for HTTP requests in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default number of retries for HTTP requests.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default maximum number of concurrent requests.
pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

/// Default rate limit in requests per second.
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 10;

/// An error that occurred during an HTTP request.
#[derive(Error, Debug)]
pub enum HttpError {
    /// The request timed out
    #[error("HTTP request timed out after {0} seconds")]
    Timeout(u64),

    /// The server returned an error status code
    #[error("HTTP error {status}: {message}")]
    StatusError {
        /// The HTTP status code
        status: StatusCode,
        /// The error message from the response body
        message: String,
    },

    /// The server returned a successful status code, but the response body was invalid
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Rate limiting error
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// URL parsing error
    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Other error
    #[error("HTTP error: {0}")]
    Other(String),
}

impl From<HttpError> for ToolError {
    fn from(err: HttpError) -> Self {
        match err {
            HttpError::Timeout(secs) => ToolError::Timeout(secs),
            HttpError::StatusError { status, message } => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "request"),
                message: format!("HTTP error {}: {}", status, message),
                retryable: status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS,
            },
            HttpError::InvalidResponse(msg) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "parse_response"),
                message: format!("Invalid response: {}", msg),
                retryable: false,
            },
            HttpError::RateLimitExceeded(msg) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "rate_limit"),
                message: format!("Rate limit exceeded: {}", msg),
                retryable: true,
            },
            HttpError::ConnectionError(msg) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "connect"),
                message: format!("Connection error: {}", msg),
                retryable: true,
            },
            HttpError::NetworkError(err) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "network"),
                message: format!("Network error: {}", err),
                retryable: err.is_timeout() || err.is_connect() || err.is_request(),
            },
            HttpError::UrlError(err) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "url_parse"),
                message: format!("URL parsing error: {}", err),
                retryable: false,
            },
            HttpError::JsonError(err) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "json"),
                message: format!("JSON error: {}", err),
                retryable: false,
            },
            HttpError::Other(msg) => ToolError::ExecutionFailed {
                context: cogni_core::error::ErrorContext::new("HttpClient", "other"),
                message: msg,
                retryable: false,
            },
        }
    }
}

/// Configuration for an HTTP client.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Timeout for HTTP requests in seconds.
    pub timeout_secs: u64,
    /// Maximum number of retries for HTTP requests.
    pub max_retries: u32,
    /// Rate limit in requests per second.
    pub rate_limit_rps: u32,
    /// Maximum number of concurrent requests.
    pub max_concurrent_requests: usize,
    /// Default headers to include in every request.
    pub default_headers: HeaderMap,
    /// User agent to use for requests.
    pub user_agent: String,
    /// Whether to follow redirects.
    pub follow_redirects: bool,
    /// Whether to enable cookie handling.
    pub enable_cookies: bool,
    /// Connect timeout in seconds.
    pub connect_timeout_secs: u64,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            HeaderName::from_static("accept"),
            HeaderValue::from_static("application/json"),
        );

        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_retries: DEFAULT_MAX_RETRIES,
            rate_limit_rps: DEFAULT_RATE_LIMIT_RPS,
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
            default_headers,
            user_agent: format!("cogni-http-client/0.1.0"),
            follow_redirects: true,
            enable_cookies: true,
            connect_timeout_secs: 10,
        }
    }
}

/// HTTP client for making requests with retries, rate limiting, and concurrency control.
#[derive(Debug, Clone)]
pub struct HttpClient {
    /// The underlying HTTP client.
    client: Client,
    /// The configuration for this client.
    config: HttpClientConfig,
    /// Rate limiter for throttling requests.
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    /// Semaphore for limiting concurrent requests.
    concurrency_limit: Arc<Mutex<()>>,
    /// Exponential backoff configuration for retries.
    backoff_config: ExponentialBackoff,
    /// Client-specific headers that are added to all requests.
    client_headers: Arc<DashMap<String, String>>,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration.
    ///
    /// # Arguments
    /// * `config` - The configuration for the HTTP client
    ///
    /// # Returns
    /// A new `HttpClient` instance.
    ///
    /// # Errors
    /// Returns an error if the client could not be created.
    pub fn new(config: HttpClientConfig) -> Result<Self, HttpError> {
        // Create the HTTP client builder
        let mut builder = ClientBuilder::new()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .default_headers(config.default_headers.clone())
            .user_agent(&config.user_agent);

        // Configure cookies if enabled
        if config.enable_cookies {
            builder = builder.cookie_store(true);
        }

        // Configure redirects
        if config.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::limited(10));
        } else {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        // Build the client
        let client = builder
            .build()
            .map_err(|e| HttpError::Other(format!("Failed to create HTTP client: {}", e)))?;

        // Create the rate limiter
        let quota = Quota::per_second(
            std::num::NonZeroU32::new(config.rate_limit_rps)
                .unwrap_or(std::num::NonZeroU32::new(DEFAULT_RATE_LIMIT_RPS).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        // Create the backoff configuration
        let backoff_config = ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(100))
            .with_max_elapsed_time(Some(Duration::from_secs(30)))
            .with_multiplier(2.0)
            .with_max_interval(Duration::from_secs(10))
            .build();

        Ok(Self {
            client,
            config,
            rate_limiter,
            concurrency_limit: Arc::new(Mutex::new(())),
            backoff_config,
            client_headers: Arc::new(DashMap::new()),
        })
    }

    /// Create a new HTTP client with default configuration.
    ///
    /// # Returns
    /// A new `HttpClient` instance with default configuration.
    ///
    /// # Errors
    /// Returns an error if the client could not be created.
    pub fn default() -> Result<Self, HttpError> {
        Self::new(HttpClientConfig::default())
    }

    /// Set a client-specific header for all requests.
    ///
    /// # Arguments
    /// * `name` - The header name
    /// * `value` - The header value
    pub fn set_header(&self, name: &str, value: &str) {
        self.client_headers
            .insert(name.to_string(), value.to_string());
    }

    /// Remove a client-specific header.
    ///
    /// # Arguments
    /// * `name` - The header name to remove
    pub fn remove_header(&self, name: &str) {
        self.client_headers.remove(name);
    }

    /// Perform a GET request.
    ///
    /// # Arguments
    /// * `url` - The URL to request
    /// * `headers` - Optional additional headers
    ///
    /// # Returns
    /// Returns the HTTP response on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the request failed.
    pub async fn get(&self, url: &str, headers: Option<HeaderMap>) -> Result<Response, HttpError> {
        self.request::<()>(Method::GET, url, None, headers).await
    }

    /// Perform a GET request and parse the response as JSON.
    ///
    /// # Arguments
    /// * `url` - The URL to request
    /// * `headers` - Optional additional headers
    ///
    /// # Returns
    /// Returns the parsed JSON response on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the request failed or the response was not valid JSON.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
    ) -> Result<T, HttpError> {
        let response = self.get(url, headers).await?;
        self.parse_json(response).await
    }

    /// Perform a POST request with JSON data.
    ///
    /// # Arguments
    /// * `url` - The URL to request
    /// * `data` - The data to send (will be serialized to JSON)
    /// * `headers` - Optional additional headers
    ///
    /// # Returns
    /// Returns the HTTP response on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the request failed.
    pub async fn post_json<T: Serialize>(
        &self,
        url: &str,
        data: &T,
        headers: Option<HeaderMap>,
    ) -> Result<Response, HttpError> {
        self.request(Method::POST, url, Some(data), headers).await
    }

    /// Perform a POST request with JSON data and parse the response as JSON.
    ///
    /// # Arguments
    /// * `url` - The URL to request
    /// * `data` - The data to send (will be serialized to JSON)
    /// * `headers` - Optional additional headers
    ///
    /// # Returns
    /// Returns the parsed JSON response on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the request failed or the response was not valid JSON.
    pub async fn post_json_return_json<T: Serialize, U: DeserializeOwned>(
        &self,
        url: &str,
        data: &T,
        headers: Option<HeaderMap>,
    ) -> Result<U, HttpError> {
        let response = self.post_json(url, data, headers).await?;
        self.parse_json(response).await
    }

    /// Perform an HTTP request with the given method, URL, and data.
    ///
    /// # Arguments
    /// * `method` - The HTTP method to use
    /// * `url` - The URL to request
    /// * `data` - Optional data to send (will be serialized to JSON if present)
    /// * `headers` - Optional additional headers
    ///
    /// # Returns
    /// Returns the HTTP response on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the request failed.
    #[instrument(level = "debug", skip(self, data, headers))]
    pub async fn request<T: Serialize>(
        &self,
        method: Method,
        url: &str,
        data: Option<&T>,
        headers: Option<HeaderMap>,
    ) -> Result<Response, HttpError> {
        // Parse the URL
        let url = Url::parse(url)?;

        // Check the rate limiter
        if let Err(err) = self.rate_limiter.check() {
            return Err(HttpError::RateLimitExceeded(format!(
                "Rate limit exceeded: {}",
                err
            )));
        }

        // Acquire the concurrency semaphore
        let _concurrency_guard = self.concurrency_limit.lock().await;

        // Build the request
        let mut request_builder = self.client.request(method.clone(), url.clone());

        // Add headers
        if let Some(headers) = headers {
            request_builder = request_builder.headers(headers);
        }

        // Add client-specific headers
        for entry in self.client_headers.iter() {
            request_builder = request_builder.header(entry.key(), entry.value().as_str());
        }

        // Add JSON data if present
        if let Some(data) = data {
            request_builder = request_builder.json(data);
        }

        // Use backoff for retries
        let backoff = self.backoff_config.clone();
        let retry_op = || async {
            let request = request_builder.try_clone().ok_or_else(|| {
                backoff::Error::permanent(HttpError::Other("Failed to clone request".to_string()))
            })?;

            debug!(
                "Sending {} request to {}",
                method,
                url.to_string().trim_end_matches('/')
            );

            // Send the request
            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    // Check if the response is successful
                    if status.is_success() {
                        debug!("Request succeeded with status {}", status);
                        Ok(response)
                    } else if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                        // Server errors and rate limiting can be retried
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        warn!(
                            "Request failed with status {}: {}",
                            status,
                            error_text.trim()
                        );
                        Err(backoff::Error::transient(HttpError::StatusError {
                            status,
                            message: error_text,
                        }))
                    } else {
                        // Client errors are permanent
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        error!(
                            "Request failed with status {}: {}",
                            status,
                            error_text.trim()
                        );
                        Err(backoff::Error::permanent(HttpError::StatusError {
                            status,
                            message: error_text,
                        }))
                    }
                }
                Err(err) => {
                    // Handle network errors
                    if err.is_timeout() {
                        warn!("Request timed out: {}", err);
                        Err(backoff::Error::transient(HttpError::Timeout(
                            self.config.timeout_secs,
                        )))
                    } else if err.is_connect() {
                        warn!("Connection error: {}", err);
                        Err(backoff::Error::transient(HttpError::ConnectionError(
                            err.to_string(),
                        )))
                    } else if err.is_request() {
                        warn!("Request error: {}", err);
                        Err(backoff::Error::transient(HttpError::NetworkError(err)))
                    } else {
                        error!("Network error: {}", err);
                        Err(backoff::Error::permanent(HttpError::NetworkError(err)))
                    }
                }
            }
        };

        // Execute the operation with retries
        backoff::future::retry(backoff, retry_op)
            .await
            .map_err(|err| {
                error!("Request failed after retries: {}", err);
                err
            })
    }

    /// Parse an HTTP response as JSON.
    ///
    /// # Arguments
    /// * `response` - The HTTP response to parse
    ///
    /// # Returns
    /// Returns the parsed JSON value on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the response could not be parsed as JSON.
    #[instrument(level = "debug", skip(self, response))]
    pub async fn parse_json<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, HttpError> {
        debug!("Parsing response as JSON");
        let text = response.text().await.map_err(|err| {
            error!("Failed to read response body: {}", err);
            HttpError::NetworkError(err)
        })?;

        trace!("Response body: {}", text);

        serde_json::from_str::<T>(&text).map_err(|err| {
            error!("Failed to parse JSON response: {}", err);
            HttpError::InvalidResponse(format!("Failed to parse JSON: {}", err))
        })
    }

    /// Get the raw bytes from a response.
    ///
    /// # Arguments
    /// * `response` - The HTTP response to read
    ///
    /// # Returns
    /// Returns the response bytes on success, or an error on failure.
    ///
    /// # Errors
    /// Returns `HttpError` if the response body could not be read.
    pub async fn get_bytes(&self, response: Response) -> Result<Bytes, HttpError> {
        debug!("Reading response as bytes");
        response.bytes().await.map_err(|err| {
            error!("Failed to read response body: {}", err);
            HttpError::NetworkError(err)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde::Deserialize;
    use std::sync::Arc;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_http_client_get() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create a response
        let response_body = r#"{"message":"Hello, world!"}"#;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(response_body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Create an HTTP client
        let client = HttpClient::default().unwrap();

        // Make a request
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, None).await.unwrap();

        // Check the response
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(body, response_body);
    }

    #[tokio::test]
    async fn test_http_client_get_json() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Define a response type
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestResponse {
            message: String,
        }

        // Create a response
        let response_body = r#"{"message":"Hello, world!"}"#;
        Mock::given(method("GET"))
            .and(path("/test-json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(response_body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Create an HTTP client
        let client = HttpClient::default().unwrap();

        // Make a request
        let url = format!("{}/test-json", mock_server.uri());
        let response: TestResponse = client.get_json(&url, None).await.unwrap();

        // Check the response
        assert_eq!(
            response,
            TestResponse {
                message: "Hello, world!".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_http_client_post_json() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Define request and response types
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestRequest {
            name: String,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestResponse {
            message: String,
        }

        // Create a response
        let response_body = r#"{"message":"Hello, Test!"}"#;
        Mock::given(method("POST"))
            .and(path("/test-post"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(response_body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Create an HTTP client
        let client = HttpClient::default().unwrap();

        // Make a request
        let url = format!("{}/test-post", mock_server.uri());
        let request = TestRequest {
            name: "Test".to_string(),
        };
        let response = client.post_json(&url, &request, None).await.unwrap();

        // Check the response
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(body, response_body);
    }

    #[tokio::test]
    async fn test_http_client_error_handling() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create a 404 response
        Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
            .mount(&mock_server)
            .await;

        // Create a 500 response
        Mock::given(method("GET"))
            .and(path("/server-error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
            .mount(&mock_server)
            .await;

        // Create an HTTP client with fewer retries for faster tests
        let mut config = HttpClientConfig::default();
        config.max_retries = 1;
        let client = HttpClient::new(config).unwrap();

        // Test 404 error
        let url = format!("{}/not-found", mock_server.uri());
        let result = client.get(&url, None).await;
        assert!(result.is_err());
        if let Err(HttpError::StatusError { status, message }) = result {
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(message, "Not found");
        } else {
            panic!("Expected StatusError, got {:?}", result);
        }

        // Test 500 error (with retries)
        let url = format!("{}/server-error", mock_server.uri());
        let result = client.get(&url, None).await;
        assert!(result.is_err());
        if let Err(HttpError::StatusError { status, message }) = result {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message, "Server error");
        } else {
            panic!("Expected StatusError, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_http_client_headers() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create a response that echoes headers
        Mock::given(method("GET"))
            .and(path("/headers"))
            .respond_with(|req: &wiremock::Request| {
                let headers = req
                    .headers
                    .iter()
                    .map(|(name, value)| {
                        // Echo the header value as string
                        let val = value
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",");
                        format!("{}:{}", name.as_str(), val)
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                ResponseTemplate::new(200).set_body_string(headers)
            })
            .mount(&mock_server)
            .await;

        // Create an HTTP client
        let client = HttpClient::default().unwrap();

        // Set a client-specific header
        client.set_header("X-Test", "test-value");

        // Create additional request headers
        let mut headers = HeaderMap::new();
        headers.insert("X-Request", "request-value".parse().unwrap());

        // Make a request
        let url = format!("{}/headers", mock_server.uri());
        let response = client.get(&url, Some(headers)).await.unwrap();

        // Check that our headers were sent
        let body = response.text().await.unwrap();
        assert!(body.contains("x-test:test-value"));
        assert!(body.contains("x-request:request-value"));
    }
}
