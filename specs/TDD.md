# Cogni Technical Design Document

## 1. Introduction

### 1.1 Purpose
Cogni is a Rust crate that provides a unified, provider-agnostic abstraction for interacting with Large Language Models (LLMs). The crate serves as a complementary component to an orchestration library, facilitating seamless integration with various LLM providers while maintaining a consistent interface.

### 1.2 Design Philosophy
The design of Cogni is guided by the following principles:
- **Flexibility**: Support for multiple LLM providers without tight coupling
- **Minimalism**: Focus on essential functionality without unnecessary bloat
- **Asynchronicity**: Fully async-first design to enable efficient resource utilization
- **Extensibility**: Easy integration of new providers and features
- **Type Safety**: Leverage Rust's type system to catch errors at compile time

### 1.3 Relationship to Orchestration Library
Cogni is designed to work harmoniously with the orchestration library. While the orchestration library handles the execution flow, state management, and coordination of tasks, Cogni focuses specifically on providing a unified interface for LLM interactions. This separation of concerns allows each component to excel in its domain.

### 1.4 Support Scope
Cogni will support the following LLM providers:
- Cloud-based APIs: OpenAI, Anthropic, and others
- Local models: Exclusively through Ollama integration
- External tools: Via Model Context Protocol (MCP) servers (both HTTP and stdio-based)

## 2. Architecture Overview

### 2.1 High-Level Design
Cogni follows a trait-based abstraction pattern, with a core `LlmProvider` trait defining the interface for all LLM interactions. Concrete implementations of this trait handle the specifics of communicating with different LLM providers. The architecture is organized into the following main components:

```
cogni/
├── core/          # Core traits and data structures
│   ├── types/     # Newtype wrappers for primitive types
│   ├── provider/  # Provider traits and common implementations
│   └── error/     # Error types and handling
├── providers/     # Implementations for specific LLM providers
├── tools/         # Tool execution traits and implementations
│   └── mcp/       # MCP-specific transport and protocol implementations
├── utils/         # Utility functions and helpers
└── model/         # Model-specific data structures and behavior
```

### 2.2 Core Components
1. **LlmProvider Trait**: The central abstraction defining common operations for LLM providers
2. **Provider Implementations**: Generic implementations of the LlmProvider trait
3. **ToolExecutor Trait**: Abstraction for tool execution, including MCP tools
4. **McpTransport Trait**: Abstraction for different MCP communication mechanisms (HTTP, stdio)
5. **Standardized Data Structures**: Common types for requests and responses
6. **Error Handling**: Comprehensive error types and propagation

### 2.3 Integration with External Systems
Cogni is designed to integrate with:
- **Orchestration Library**: Via NodeExecutable implementation
- **LLM APIs**: Through provider-specific HTTP clients
- **Model Context Protocol (MCP) Servers**: Via the ToolExecutor trait with multiple transport options
- **Local Models**: Support exclusively through Ollama integration

## 3. Core Abstractions

### 3.1 Newtype Wrappers
To provide better type safety and clarity, we use newtype wrappers for primitive types:

```rust
/// Strongly-typed wrapper for API keys
#[derive(Clone, Debug)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ApiKey {
    fn from(key: String) -> Self {
        Self(key)
    }
}

impl From<&str> for ApiKey {
    fn from(key: &str) -> Self {
        Self(key.to_string())
    }
}

/// Strongly-typed wrapper for base URLs
#[derive(Clone, Debug)]
pub struct BaseUrl(String);

impl BaseUrl {
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for BaseUrl {
    fn from(url: String) -> Self {
        Self(url)
    }
}

impl From<&str> for BaseUrl {
    fn from(url: &str) -> Self {
        Self(url.to_string())
    }
}

/// Default trait implementations to support optional fields
impl Default for BaseUrl {
    fn default() -> Self {
        Self("".to_string())
    }
}

/// Organization ID wrapper
#[derive(Clone, Debug)]
pub struct OrganizationId(String);

impl OrganizationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for OrganizationId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for OrganizationId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}
```

### 3.2 LlmProvider Trait
The `LlmProvider` trait defines the core interface for all LLM interactions:

```rust
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Core trait for LLM providers
/// 
/// Implementations of this trait provide access to specific LLM providers
/// such as OpenAI, Anthropic, or Ollama.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// The type of response chunk returned when streaming chat completions
    type ChatChunk: Send + 'static;
    
    /// Execute a chat completion request
    /// 
    /// If the provider has a tool executor configured and the request includes tools,
    /// this method will handle tool execution automatically.
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError>;
    
    /// Execute a text completion request
    async fn completion(&self, request: &CompletionRequest) -> Result<CompletionResponse, LlmError>;
    
    /// Generate embeddings for given input
    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, LlmError>;
    
    /// Stream a chat completion response
    /// 
    /// Returns a Stream of response chunks that can be processed as they arrive.
    async fn stream_chat<'a>(
        &'a self, 
        request: &'a ChatRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError>;
    
    /// Returns the supported models by this provider
    fn supported_models(&self) -> Vec<String>;
    
    /// Check if a specific feature is supported by this provider
    fn supports_feature(&self, feature: ProviderFeature) -> bool;
}

/// Features that providers may optionally support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFeature {
    ToolCalling,
    Streaming,
    Embeddings,
    FunctionCalling,
    VisionModality,
    AudioModality,
}
```

### 3.3 ToolExecutor Trait
The `ToolExecutor` trait defines the interface for executing external tools, including those exposed by MCP servers:

```rust
/// Trait for executing external tools
/// 
/// This trait is implemented by components that can execute tools,
/// such as MCP clients or custom tool implementations.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool with the given name and arguments
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError>;
    
    /// Get a list of available tools
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError>;
}

/// No-op implementation of ToolExecutor that does nothing
/// 
/// This is used as a default type parameter for providers that don't need tools
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopToolExecutor;

#[async_trait]
impl ToolExecutor for NoopToolExecutor {
    async fn execute_tool(&self, _name: &str, _arguments: &str) -> Result<String, ToolError> {
        Err(ToolError::UnsupportedOperation("This provider does not support tools".into()))
    }
    
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
        Ok(Vec::new())
    }
}

/// Error type for tool-related operations
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Process error: {0}")]
    ProcessError(#[from] std::io::Error),
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

impl From<ToolError> for LlmError {
    fn from(error: ToolError) -> Self {
        match error {
            ToolError::ToolNotFound(msg) => LlmError::ToolExecutionError(msg),
            ToolError::InvalidArguments(msg) => LlmError::ToolExecutionError(msg),
            ToolError::ExecutionFailed(msg) => LlmError::ToolExecutionError(msg),
            ToolError::SerializationError(err) => LlmError::SerializationError(err),
            ToolError::NetworkError(err) => LlmError::NetworkError(err),
            ToolError::ProcessError(err) => LlmError::ToolExecutionError(format!("Process error: {}", err)),
            ToolError::UnsupportedOperation(msg) => LlmError::UnsupportedFeature(msg),
        }
    }
}
```

### 3.4 McpTransport Trait
The `McpTransport` trait abstracts the underlying communication mechanism for MCP servers:

```rust
/// Trait for MCP transport mechanisms
/// 
/// This trait abstracts different ways of communicating with MCP servers,
/// such as HTTP or stdio.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Execute a tool with the given name and arguments
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError>;
    
    /// Get a list of available tools
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError>;
}
```

### 3.5 Standardized Data Structures
The crate defines provider-agnostic structures for all operations:

#### 3.5.1 Common Message Structures
```rust
/// A message in a chat conversation
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Role of a message in a chat conversation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

impl Default for MessageRole {
    fn default() -> Self {
        Self::User
    }
}

/// Content of a message, which can be text or multi-modal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    MultiModal(Vec<ContentPart>),
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// Part of a multi-modal message content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContentPart {
    Text(String),
    Image(ImageData),
    Audio(AudioData),
}

/// Image data for multi-modal content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Audio data for multi-modal content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
```

#### 3.5.2 Request Structures
```rust
/// Request for a chat completion
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<serde_json::Value>,
}

/// Request for a text completion
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<serde_json::Value>,
}

/// Request for generating embeddings
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<serde_json::Value>,
}

/// Format options for response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseFormat {
    pub type_field: ResponseFormatType,
}

/// Type of response format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResponseFormatType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
}

impl Default for ResponseFormatType {
    fn default() -> Self {
        Self::Text
    }
}

impl Default for ResponseFormat {
    fn default() -> Self {
        Self {
            type_field: ResponseFormatType::default(),
        }
    }
}
```

#### 3.5.3 Response Structures
```rust
/// Response from a chat completion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
}

/// Response from a text completion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
}

/// Response from an embedding request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
}

/// Chunk of a streaming chat response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponseChunk {
    pub delta: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
}

/// Token usage statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

#### 3.5.4 Tool and Function Calling Structures
```rust
/// Definition of a tool that can be used by the model
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Tool {
    pub r#type: ToolType,
    pub function: Function,
}

/// Type of tool 
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolType {
    Function,
}

impl Default for ToolType {
    fn default() -> Self {
        Self::Function
    }
}

/// Definition of a function that can be called by the model
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value, // JSON Schema object
}

/// Controls how the model uses tools
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolChoice {
    Auto,
    Required,
    Specific(String), // Tool/function name
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}

/// A tool call made by the model
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ToolCall {
    pub id: String,
    pub r#type: ToolType,
    pub function: ToolCallFunction,
}

/// Function call made by the model
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String, // JSON string
}
```

### 3.6 Error Handling
A comprehensive error system that captures provider-specific errors while providing a uniform interface:

```rust
/// Error type for LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API request failed: {0}")]
    ApiError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Provider-specific error: {0}")]
    ProviderSpecific(String),
    
    #[error("Feature not supported by provider: {0}")]
    UnsupportedFeature(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Stream error: {0}")]
    StreamError(String),
    
    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),
    
    #[error("Timeout error")]
    Timeout,
}
```

## 4. Provider Implementations

### 4.1 Standard Provider Implementations

#### 4.1.1 OpenAI Provider
```rust
/// Provider implementation for OpenAI API
pub struct OpenAiProvider<T = NoopToolExecutor> {
    client: reqwest::Client,
    api_key: ApiKey,
    organization: Option<OrganizationId>,
    base_url: BaseUrl,
    tool_executor: T,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given API key
    pub fn new(api_key: impl Into<ApiKey>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            organization: None,
            base_url: BaseUrl::from("https://api.openai.com/v1"),
            tool_executor: NoopToolExecutor,
        }
    }
}

impl<T> OpenAiProvider<T> {
    /// Add an organization ID to requests
    pub fn with_organization(mut self, organization: impl Into<OrganizationId>) -> Self {
        self.organization = Some(organization.into());
        self
    }
    
    /// Use a custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<BaseUrl>) -> Self {
        self.base_url = base_url.into();
        self
    }
    
    /// Send a chat request to the OpenAI API
    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Implementation details...
        todo!()
    }
}

impl<T: ToolExecutor> OpenAiProvider<T> {
    /// Create a new OpenAI provider with an API key and tool executor
    pub fn new_with_tools(api_key: impl Into<ApiKey>, tool_executor: T) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            organization: None,
            base_url: BaseUrl::from("https://api.openai.com/v1"),
            tool_executor,
        }
    }
    
    /// Create a new provider with a different tool executor
    pub fn with_tool_executor<U: ToolExecutor>(self, tool_executor: U) -> OpenAiProvider<U> {
        OpenAiProvider {
            client: self.client,
            api_key: self.api_key,
            organization: self.organization,
            base_url: self.base_url,
            tool_executor,
        }
    }
}

#[async_trait]
impl<T: ToolExecutor + Send + Sync> LlmProvider for OpenAiProvider<T> {
    type ChatChunk = ChatResponseChunk;
    
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Step 1: Determine if we need to augment the request with tools
        let mut augmented_request = request.clone();
        
        if request.tools.is_none() {
            // Get tools from the executor
            match self.tool_executor.get_available_tools().await {
                Ok(tools) if !tools.is_empty() => {
                    augmented_request.tools = Some(tools);
                },
                Err(err) => return Err(err.into()),
                _ => {}
            }
        }
        
        // Step 2: Send the request to OpenAI API
        let response = self.send_chat_request(&augmented_request).await?;
        
        // Step 3: If response contains tool calls, handle them
        if let Some(tool_calls) = &response.message.tool_calls {
            if !tool_calls.is_empty() {
                // Create a new message list with the original messages and the model's response
                let mut messages = request.messages.clone();
                messages.push(response.message.clone());
                
                // Process each tool call
                for tool_call in tool_calls {
                    // Execute the tool
                    let result = self.tool_executor.execute_tool(
                        &tool_call.function.name, 
                        &tool_call.function.arguments
                    ).await?;
                    
                    // Add the tool response to the message list
                    messages.push(ChatMessage {
                        role: MessageRole::Tool,
                        content: MessageContent::Text(result),
                        name: None,
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                
                // Create a new request with the updated messages
                let final_request = ChatRequest {
                    messages,
                    model: request.model.clone(),
                    temperature: request.temperature,
                    top_p: request.top_p,
                    max_tokens: request.max_tokens,
                    tools: augmented_request.tools.clone(),
                    tool_choice: None, // No need to force tool choice for follow-up
                    response_format: request.response_format.clone(),
                    provider_options: request.provider_options.clone(),
                };
                
                // Get the final response with tool results incorporated
                return self.send_chat_request(&final_request).await;
            }
        }
        
        // If no tool calls or no executor, return the original response
        Ok(response)
    }
    
    async fn completion(&self, request: &CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Implementation details...
        todo!()
    }
    
    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        // Implementation details...
        todo!()
    }
    
    async fn stream_chat<'a>(
        &'a self, 
        request: &'a ChatRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError> {
        // Implementation details...
        todo!()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
            // Add more models as needed
        ]
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::ToolCalling => true,
            ProviderFeature::Streaming => true,
            ProviderFeature::Embeddings => true,
            ProviderFeature::FunctionCalling => true,
            ProviderFeature::VisionModality => true,
            ProviderFeature::AudioModality => false,
        }
    }
}
```

#### 4.1.2 Anthropic Provider
```rust
/// Provider implementation for Anthropic API
pub struct AnthropicProvider<T = NoopToolExecutor> {
    client: reqwest::Client,
    api_key: ApiKey,
    base_url: BaseUrl,
    tool_executor: T,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key
    pub fn new(api_key: impl Into<ApiKey>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: BaseUrl::from("https://api.anthropic.com"),
            tool_executor: NoopToolExecutor,
        }
    }
}

impl<T> AnthropicProvider<T> {
    /// Use a custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<BaseUrl>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

impl<T: ToolExecutor> AnthropicProvider<T> {
    /// Create a new Anthropic provider with an API key and tool executor
    pub fn new_with_tools(api_key: impl Into<ApiKey>, tool_executor: T) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: BaseUrl::from("https://api.anthropic.com"),
            tool_executor,
        }
    }
    
    /// Create a new provider with a different tool executor
    pub fn with_tool_executor<U: ToolExecutor>(self, tool_executor: U) -> AnthropicProvider<U> {
        AnthropicProvider {
            client: self.client,
            api_key: self.api_key,
            base_url: self.base_url,
            tool_executor,
        }
    }
}

#[async_trait]
impl<T: ToolExecutor + Send + Sync> LlmProvider for AnthropicProvider<T> {
    type ChatChunk = ChatResponseChunk;
    
    // Similar implementation to OpenAiProvider's chat method
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Implementation follows the same pattern as OpenAiProvider
        todo!()
    }
    
    // Other method implementations...
    async fn completion(&self, request: &CompletionRequest) -> Result<CompletionResponse, LlmError> {
        todo!()
    }
    
    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        todo!()
    }
    
    async fn stream_chat<'a>(
        &'a self, 
        request: &'a ChatRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError> {
        todo!()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            // Add more models as needed
        ]
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::ToolCalling => true,
            ProviderFeature::Streaming => true,
            ProviderFeature::Embeddings => false,
            ProviderFeature::FunctionCalling => true,
            ProviderFeature::VisionModality => true,
            ProviderFeature::AudioModality => false,
        }
    }
}
```

#### 4.1.3 Ollama Provider
```rust
/// Provider implementation for Ollama API
/// This is the only supported method for local model integration
pub struct OllamaProvider<T = NoopToolExecutor> {
    client: reqwest::Client,
    base_url: BaseUrl,
    tool_executor: T,
}

impl OllamaProvider {
    /// Create a new Ollama provider with the given base URL
    pub fn new(base_url: impl Into<BaseUrl>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            tool_executor: NoopToolExecutor,
        }
    }
    
    /// Create a new Ollama provider with the default localhost URL
    pub fn local() -> Self {
        Self::new("http://localhost:11434")
    }
}

impl<T: ToolExecutor> OllamaProvider<T> {
    /// Create a new Ollama provider with a base URL and tool executor
    pub fn new_with_tools(base_url: impl Into<BaseUrl>, tool_executor: T) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            tool_executor,
        }
    }
    
    /// Create a new provider with a different tool executor
    pub fn with_tool_executor<U: ToolExecutor>(self, tool_executor: U) -> OllamaProvider<U> {
        OllamaProvider {
            client: self.client,
            base_url: self.base_url,
            tool_executor,
        }
    }
}

#[async_trait]
impl<T: ToolExecutor + Send + Sync> LlmProvider for OllamaProvider<T> {
    type ChatChunk = ChatResponseChunk;
    
    // Method implementations for accessing local models through Ollama's API
    // This is the only supported method for local model integration
    // Similar tool handling pattern to OpenAiProvider
    
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Similar implementation to OpenAiProvider's chat method, adapted for Ollama
        todo!()
    }
    
    // Other method implementations...
    async fn completion(&self, request: &CompletionRequest) -> Result<CompletionResponse, LlmError> {
        todo!()
    }
    
    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        todo!()
    }
    
    async fn stream_chat<'a>(
        &'a self, 
        request: &'a ChatRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError> {
        todo!()
    }
    
    fn supported_models(&self) -> Vec<String> {
        // This would typically be fetched from the Ollama API at runtime
        vec![
            "llama3".to_string(),
            "mistral".to_string(),
            "gemma".to_string(),
            // These are examples; actual models depend on what's installed in Ollama
        ]
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::ToolCalling => false, // Most Ollama models don't support tool calling natively
            ProviderFeature::Streaming => true,
            ProviderFeature::Embeddings => true,
            ProviderFeature::FunctionCalling => false,
            ProviderFeature::VisionModality => false, // Depends on the specific model
            ProviderFeature::AudioModality => false,
        }
    }
}
```

### 4.2 MCP Tools Integration

#### 4.2.1 MCP Transport Implementations

```rust
/// HTTP-based MCP transport
pub struct McpHttpTransport {
    client: reqwest::Client,
    base_url: BaseUrl,
    auth_token: Option<String>,
}

impl McpHttpTransport {
    /// Create a new HTTP-based MCP transport
    pub fn new(base_url: impl Into<BaseUrl>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            auth_token: None,
        }
    }
    
    /// Add authentication token for HTTP MCP server
    pub fn with_auth_token(mut self, auth_token: impl Into<String>) -> Self {
        self.auth_token = Some(auth_token.into());
        self
    }
}

#[async_trait]
impl McpTransport for McpHttpTransport {
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
        let request_body = serde_json::json!({
            "arguments": arguments
        });
        
        let mut request = self.client
            .post(format!("{}/tools/{}", self.base_url.as_str(), name))
            .json(&request_body);
            
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request.send()
            .await
            .map_err(ToolError::NetworkError)?;
        
        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "MCP tool execution failed with status: {}", response.status()
            )));
        }
        
        let result: serde_json::Value = response.json()
            .await
            .map_err(ToolError::NetworkError)?;
        
        Ok(result.get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }
    
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
        let mut request = self.client
            .get(format!("{}/tools", self.base_url.as_str()));
        
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request.send()
            .await
            .map_err(ToolError::NetworkError)?;
        
        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "Failed to fetch MCP tools with status: {}", response.status()
            )));
        }
        
        let tools: Vec<Tool> = response.json()
            .await
            .map_err(ToolError::SerializationError)?;
        
        Ok(tools)
    }
}

/// Stdio-based MCP transport
pub struct McpStdioTransport {
    process: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: tokio::io::BufReader<tokio::process::ChildStdout>,
}

impl McpStdioTransport {
    /// Create a new stdio-based MCP transport
    pub fn new(command: impl AsRef<str>, args: &[impl AsRef<str>]) -> Result<Self, ToolError> {
        let process_args: Vec<&str> = args.iter()
            .map(AsRef::as_ref)
            .collect();
            
        let mut process = tokio::process::Command::new(command.as_ref())
            .args(&process_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ProcessError(e))?;
            
        let stdin = process.stdin.take()
            .ok_or_else(|| ToolError::ExecutionFailed("Failed to open stdin".to_string()))?;
            
        let stdout = process.stdout.take()
            .ok_or_else(|| ToolError::ExecutionFailed("Failed to open stdout".to_string()))?;
            
        let stdout_reader = tokio::io::BufReader::new(stdout);
        
        Ok(Self {
            process,
            stdin,
            stdout_reader,
        })
    }
}

#[async_trait]
impl McpTransport for McpStdioTransport {
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
        use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
        
        let request = serde_json::json!({
            "action": "execute",
            "tool": name,
            "arguments": arguments
        });
        
        // Write request to stdin
        let mut stdin = self.stdin.try_clone().map_err(ToolError::ProcessError)?;
        stdin.write_all(format!("{}\n", request.to_string()).as_bytes())
            .await
            .map_err(ToolError::ProcessError)?;
            
        // Read response from stdout
        let mut stdout_reader = self.stdout_reader.clone();
        let mut line = String::new();
        stdout_reader.read_line(&mut line)
            .await
            .map_err(ToolError::ProcessError)?;
            
        // Parse response
        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(ToolError::SerializationError)?;
            
        if let Some(error) = response.get("error") {
            return Err(ToolError::ExecutionFailed(error.as_str()
                .unwrap_or("Unknown error")
                .to_string()));
        }
        
        Ok(response.get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }
    
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
        use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
        
        let request = serde_json::json!({
            "action": "list_tools"
        });
        
        // Write request to stdin
        let mut stdin = self.stdin.try_clone().map_err(ToolError::ProcessError)?;
        stdin.write_all(format!("{}\n", request.to_string()).as_bytes())
            .await
            .map_err(ToolError::ProcessError)?;
            
        // Read response from stdout
        let mut stdout_reader = self.stdout_reader.clone();
        let mut line = String::new();
        stdout_reader.read_line(&mut line)
            .await
            .map_err(ToolError::ProcessError)?;
            
        // Parse response
        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(ToolError::SerializationError)?;
            
        if let Some(error) = response.get("error") {
            return Err(ToolError::ExecutionFailed(error.as_str()
                .unwrap_or("Unknown error")
                .to_string()));
        }
        
        let tools: Vec<Tool> = serde_json::from_value(
            response.get("tools")
                .ok_or_else(|| ToolError::ExecutionFailed("No tools field in response".to_string()))?
                .clone()
        ).map_err(ToolError::SerializationError)?;
        
        Ok(tools)
    }
}
```

#### 4.2.2 MCP Tool Client
```rust
/// Client for Model Context Protocol (MCP) servers
pub struct McpToolClient<T: McpTransport = McpHttpTransport> {
    transport: T,
}

impl McpToolClient<McpHttpTransport> {
    /// Create a new MCP client with the given HTTP base URL
    pub fn new_http(base_url: impl Into<BaseUrl>) -> Self {
        Self {
            transport: McpHttpTransport::new(base_url),
        }
    }
    
    /// Add authentication token for HTTP MCP server
    pub fn with_auth_token(mut self, auth_token: impl Into<String>) -> Self {
        self.transport = self.transport.with_auth_token(auth_token);
        self
    }
}

impl McpToolClient<McpStdioTransport> {
    /// Create a new MCP client that communicates with a local process via stdio
    pub fn new_process(
        command: impl AsRef<str>, 
        args: &[impl AsRef<str>]
    ) -> Result<Self, ToolError> {
        Ok(Self {
            transport: McpStdioTransport::new(command, args)?,
        })
    }
}

#[async_trait]
impl<T: McpTransport + Send + Sync> ToolExecutor for McpToolClient<T> {
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
        self.transport.execute_tool(name, arguments).await
    }
    
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
        self.transport.get_available_tools().await
    }
}
```

### 4.3 Provider Factory
A factory pattern to simplify provider instantiation based on configuration:

```rust
/// Type of LLM provider
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    OpenAi,
    Anthropic,
    Ollama,
    Custom(String),
}

/// Configuration for creating an LLM provider
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<ApiKey>,
    pub base_url: Option<BaseUrl>,
    pub organization: Option<OrganizationId>,
    pub tool_executor_config: Option<ToolExecutorConfig>,
    pub additional_options: HashMap<String, String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::OpenAi,
            api_key: None,
            base_url: None,
            organization: None,
            tool_executor_config: None,
            additional_options: HashMap::new(),
        }
    }
}

/// Builder for ProviderConfig
pub struct ProviderConfigBuilder {
    config: ProviderConfig,
}

impl ProviderConfigBuilder {
    pub fn new(provider_type: ProviderType) -> Self {
        Self {
            config: ProviderConfig {
                provider_type,
                ..Default::default()
            },
        }
    }
    
    pub fn api_key(mut self, api_key: impl Into<ApiKey>) -> Self {
        self.config.api_key = Some(api_key.into());
        self
    }
    
    pub fn base_url(mut self, base_url: impl Into<BaseUrl>) -> Self {
        self.config.base_url = Some(base_url.into());
        self
    }
    
    pub fn organization(mut self, org: impl Into<OrganizationId>) -> Self {
        self.config.organization = Some(org.into());
        self
    }
    
    pub fn mcp_http(
        mut self, 
        base_url: impl Into<BaseUrl>, 
        auth_token: Option<String>
    ) -> Self {
        self.config.tool_executor_config = Some(ToolExecutorConfig::McpHttp { 
            base_url: base_url.into(), 
            auth_token 
        });
        self
    }
    
    pub fn mcp_process(
        mut self,
        command: impl Into<String>,
        args: Vec<String>
    ) -> Self {
        self.config.tool_executor_config = Some(ToolExecutorConfig::McpProcess {
            command: command.into(),
            args,
        });
        self
    }
    
    pub fn custom_tools(mut self, executor: Box<dyn ToolExecutor + Send + Sync>) -> Self {
        self.config.tool_executor_config = Some(ToolExecutorConfig::Custom(executor));
        self
    }
    
    pub fn option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.additional_options.insert(key.into(), value.into());
        self
    }
    
    pub fn build(self) -> ProviderConfig {
        self.config
    }
}

/// Configuration for tool executors
#[derive(Clone)]
pub enum ToolExecutorConfig {
    McpHttp {
        base_url: BaseUrl,
        auth_token: Option<String>,
    },
    McpProcess {
        command: String,
        args: Vec<String>,
    },
    Custom(Box<dyn ToolExecutor + Send + Sync>),
}

impl std::fmt::Debug for ToolExecutorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::McpHttp { base_url, auth_token } => {
                f.debug_struct("McpHttp")
                    .field("base_url", base_url)
                    .field("auth_token", &auth_token.as_ref().map(|_| "***"))
                    .finish()
            }
            Self::McpProcess { command, args } => {
                f.debug_struct("McpProcess")
                    .field("command", command)
                    .field("args", args)
                    .finish()
            }
            Self::Custom(_) => f.debug_struct("Custom").finish_non_exhaustive(),
        }
    }
}

/// Create a provider from a configuration
pub fn create_provider(
    config: ProviderConfig
) -> Result<Box<dyn LlmProvider<ChatChunk = ChatResponseChunk> + Send + Sync>, LlmError> {
    match config.provider_type {
        ProviderType::OpenAi => create_openai_provider(config),
        ProviderType::Anthropic => create_anthropic_provider(config),
        ProviderType::Ollama => create_ollama_provider(config),
        ProviderType::Custom(name) => {
            Err(LlmError::ConfigurationError(format!(
                "Custom provider '{}' not registered",
                name
            )))
        }
    }
}

fn create_openai_provider(
    config: ProviderConfig
) -> Result<Box<dyn LlmProvider<ChatChunk = ChatResponseChunk> + Send + Sync>, LlmError> {
    let api_key = config.api_key.ok_or_else(|| {
        LlmError::ConfigurationError("API key required for OpenAI provider".to_string())
    })?;
    
    let mut provider = OpenAiProvider::new(api_key);
    
    if let Some(base_url) = config.base_url {
        provider = provider.with_base_url(base_url);
    }
    
    if let Some(org) = config.organization {
        provider = provider.with_organization(org);
    }
    
    // Add tool executor if configured
    let provider_with_tools = match config.tool_executor_config {
        Some(ToolExecutorConfig::McpHttp { base_url, auth_token }) => {
            let client = McpToolClient::new_http(base_url);
            let client = if let Some(token) = auth_token {
                client.with_auth_token(token)
            } else {
                client
            };
            provider.with_tool_executor(client)
        },
        Some(ToolExecutorConfig::McpProcess { command, args }) => {
            let args_slice: Vec<&str> = args.iter()
                .map(AsRef::as_ref)
                .collect();
            
            match McpToolClient::new_process(&command, &args_slice) {
                Ok(client) => provider.with_tool_executor(client),
                Err(e) => return Err(e.into()),
            }
        },
        Some(ToolExecutorConfig::Custom(executor)) => {
            // Requires a type erased approach because we can't name the concrete type here
            // Use dyn ToolExecutor at runtime
            struct DynToolExecutor(Box<dyn ToolExecutor + Send + Sync>);
            
            #[async_trait]
            impl ToolExecutor for DynToolExecutor {
                async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
                    self.0.execute_tool(name, arguments).await
                }
                
                async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
                    self.0.get_available_tools().await
                }
            }
            
            provider.with_tool_executor(DynToolExecutor(executor))
        },
        None => provider,
    };
    
    Ok(Box::new(provider_with_tools))
}

fn create_anthropic_provider(
    config: ProviderConfig
) -> Result<Box<dyn LlmProvider<ChatChunk = ChatResponseChunk> + Send + Sync>, LlmError> {
    // Similar to create_openai_provider but for Anthropic
    // ...
    todo!()
}

fn create_ollama_provider(
    config: ProviderConfig
) -> Result<Box<dyn LlmProvider<ChatChunk = ChatResponseChunk> + Send + Sync>, LlmError> {
    // Similar to create_openai_provider but for Ollama
    // ...
    todo!()
}
```

## 5. Advanced Features

### 5.1 Tool and Function Calling
Tool and function calling is integrated directly into each provider's implementation of the `chat` method. This design allows for:

1. **Unified Interface**: A single `chat` method handles both simple text interactions and complex tool-using scenarios
2. **Transparent Tool Execution**: When a model decides to use tools, the provider automatically executes them
3. **Composition**: Tools can be provided in the request directly, from a configured tool executor, or both

Each provider's implementation follows a common pattern:
1. Augment the request with tools from the configured executor (if any)
2. Send the request to the model
3. If the model requests tool calls, execute them using the tool executor
4. Send a follow-up request with the tool results
5. Return the final response

The generic implementation allows for type-safe composition of providers and tool executors, without runtime boxing overhead in most cases.

### 5.2 Streaming Responses
Streaming responses are supported through the `stream_chat` method, which returns a Stream of response chunks:

```rust
async fn stream_chat<'a>(
    &'a self, 
    request: &'a ChatRequest
) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError>;
```

The use of an associated type (`Self::ChatChunk`) allows each provider to define its own chunk type while maintaining a common interface.

### 5.3 Streaming with Tools
When streaming is combined with tool calling, the stream will pause when tool calls are encountered. The typical flow is:

1. Start streaming the response
2. If a tool call is detected in the stream, pause streaming
3. Execute the tool call(s) 
4. Resume streaming with a new request that includes the tool results

This is handled transparently by the provider's implementation, but applications should be aware that there may be pauses in the stream while tools are being executed.

### 5.4 Batching and Rate Limiting
For providers that support batching or require rate limiting, additional helper utilities are provided:

```rust
/// Provider wrapper that enforces rate limits
pub struct RateLimitedProvider<P> {
    inner: P,
    rate_limiter: RateLimiter,
}

impl<P> RateLimitedProvider<P> {
    /// Create a new rate-limited provider with the specified requests per minute
    pub fn new(provider: P, requests_per_minute: u32) -> Self {
        Self {
            inner: provider,
            rate_limiter: RateLimiter::new(requests_per_minute),
        }
    }
}

#[async_trait]
impl<P: LlmProvider + Send + Sync> LlmProvider for RateLimitedProvider<P> {
    type ChatChunk = P::ChatChunk;
    
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        self.rate_limiter.acquire().await?;
        self.inner.chat(request).await
    }
    
    // Other method implementations delegate to inner provider after acquiring a permit
    // ...
}
```

For batch processing, utilities for concurrent execution with controlled parallelism are available:

```rust
/// Process multiple requests concurrently
pub async fn process_batch<P, T>(
    provider: &P,
    requests: &[ChatRequest],
    max_concurrency: usize
) -> Result<Vec<Result<ChatResponse, LlmError>>, LlmError> 
where
    P: LlmProvider<ChatChunk = T> + Send + Sync,
    T: Send + 'static,
{
    use futures::stream::{self, StreamExt};
    
    // Create a stream of futures
    let futures = requests.iter().map(|req| {
        let provider_ref = provider;
        async move { provider_ref.chat(req).await }
    });
    
    // Process with controlled concurrency
    let results = stream::iter(futures)
        .buffer_unordered(max_concurrency)
        .collect::<Vec<_>>()
        .await;
    
    Ok(results)
}
```

### 5.5 Error Handling and Retries
A retry mechanism for handling transient errors:

```rust
/// Provider wrapper that automatically retries failed requests
pub struct RetryingProvider<P> {
    inner: P,
    max_retries: u32,
    retry_delay_ms: u64,
    retryable_errors: Vec<RetryableErrorType>,
}

/// Types of errors that can be retried
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryableErrorType {
    RateLimit,
    NetworkTimeout,
    ServerError,
}

impl<P> RetryingProvider<P> {
    /// Create a new retrying provider with the specified maximum retries
    pub fn new(provider: P, max_retries: u32) -> Self {
        Self {
            inner: provider,
            max_retries,
            retry_delay_ms: 1000,
            retryable_errors: vec![
                RetryableErrorType::RateLimit,
                RetryableErrorType::NetworkTimeout,
                RetryableErrorType::ServerError,
            ],
        }
    }
    
    /// Customize the retry delay
    pub fn with_retry_delay_ms(mut self, delay_ms: u64) -> Self {
        self.retry_delay_ms = delay_ms;
        self
    }
    
    /// Customize which error types should be retried
    pub fn with_retryable_errors(mut self, error_types: Vec<RetryableErrorType>) -> Self {
        self.retryable_errors = error_types;
        self
    }
    
    /// Execute a function with retries
    async fn retry<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, E>> + Send,
        E: std::fmt::Debug,
        Self: RetryCheck<E>,
    {
        let mut attempts = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(err) if self.is_retryable(&err) && attempts < self.max_retries => {
                    attempts += 1;
                    // Exponential backoff
                    let delay = self.retry_delay_ms * 2u64.pow(attempts - 1);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

/// Trait for checking if an error is retryable
pub trait RetryCheck<E> {
    fn is_retryable(&self, error: &E) -> bool;
}

impl<P> RetryCheck<LlmError> for RetryingProvider<P> {
    fn is_retryable(&self, error: &LlmError) -> bool {
        match error {
            LlmError::RateLimitExceeded => {
                self.retryable_errors.contains(&RetryableErrorType::RateLimit)
            },
            LlmError::NetworkError(_) => {
                self.retryable_errors.contains(&RetryableErrorType::NetworkTimeout)
            },
            LlmError::ApiError(msg) if msg.contains("5") => {
                self.retryable_errors.contains(&RetryableErrorType::ServerError)
            },
            LlmError::Timeout => {
                self.retryable_errors.contains(&RetryableErrorType::NetworkTimeout)
            },
            _ => false,
        }
    }
}

#[async_trait]
impl<P: LlmProvider + Send + Sync> LlmProvider for RetryingProvider<P> {
    type ChatChunk = P::ChatChunk;
    
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        self.retry(|| async { self.inner.chat(request).await }).await
    }
    
    // Other method implementations similarly use retry...
    // ...
}
```

## 6. Integration with Orchestration

### 6.1 NodeExecutable Implementation
Implementation of `NodeExecutable` for `NodeData::Llm`:

```rust
/// Implementation of NodeExecutable for LLM nodes
#[async_trait]
impl NodeExecutable for NodeData::Llm {
    async fn execute(
        &self,
        context: &mut ExecutionContext,
    ) -> Result<(), OrchestrationError> {
        // Build the provider configuration
        let mut config_builder = ProviderConfigBuilder::new(self.config.provider.clone());
        
        // Add API key if provided
        if let Some(api_key) = &self.config.api_key {
            config_builder = config_builder.api_key(api_key);
        }
        
        // Add base URL if provided
        if let Some(base_url) = &self.config.base_url {
            config_builder = config_builder.base_url(base_url);
        }
        
        // Add MCP tool executor if configured
        if self.config.use_mcp_tools {
            let base_url = self.config.mcp_base_url.clone()
                .ok_or_else(|| OrchestrationError::ConfigurationError(
                    "MCP base URL is required when use_mcp_tools is true".to_string()
                ))?;
            
            // Use MCP over HTTP by default
            config_builder = config_builder.mcp_http(base_url, self.config.mcp_auth_token.clone());
            
            // Check if we should use stdio instead (for local MCP servers)
            if let Some(true) = self.config.mcp_use_stdio {
                if let Some(command) = &self.config.mcp_command {
                    let args = self.config.mcp_args.clone().unwrap_or_default();
                    config_builder = config_builder.mcp_process(command, args);
                }
            }
        }
        
        // Create provider instance
        let provider = create_provider(config_builder.build())
            .map_err(|e| OrchestrationError::NodeExecutionError(Box::new(e)))?;
        
        // Construct request from context
        let request = self.build_request_from_context(context)?;
        
        // Execute request (tool execution happens automatically if needed)
        let response = provider.chat(&request)
            .await
            .map_err(|e| OrchestrationError::NodeExecutionError(Box::new(e)))?;
        
        // Update context with response
        self.update_context_with_response(context, response)?;
        
        Ok(())
    }
}
```

### 6.2 Configuration
Node configuration schema for LLM nodes:

```rust
/// Configuration for LLM nodes in the orchestration graph
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmNodeConfig {
    pub provider: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub use_mcp_tools: bool,
    pub mcp_base_url: Option<String>,
    pub mcp_auth_token: Option<String>,
    #[serde(default)]
    pub mcp_use_stdio: Option<bool>,
    pub mcp_command: Option<String>,
    pub mcp_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub input_variables: Vec<String>,
    pub output_variable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>, // Additional tools to include in the request
}
```

### 6.3 Context Management
The `ExecutionContext` from the orchestration library is used to pass data between nodes:

```rust
impl NodeData::Llm {
    /// Build a chat request from the execution context
    fn build_request_from_context(&self, context: &ExecutionContext) -> Result<ChatRequest, OrchestrationError> {
        let mut messages = Vec::new();
        
        // Add system prompt if configured
        if let Some(system_prompt) = &self.config.system_prompt {
            messages.push(ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt.clone()),
                ..Default::default()
            });
        }
        
        // Add user message from input variables
        let mut user_content = String::new();
        for var_name in &self.config.input_variables {
            if let Some(value) = context.get_variable(var_name) {
                user_content.push_str(&value.to_string());
                user_content.push_str("\n");
            } else {
                return Err(OrchestrationError::VariableNotFound(var_name.clone()));
            }
        }
        
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Text(user_content),
            ..Default::default()
        });
        
        // Construct the request
        Ok(ChatRequest {
            messages,
            model: self.config.model.clone(),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            tools: self.config.tools.clone(), // Include any tools specified in the node config
            ..Default::default()
        })
    }
    
    /// Update the execution context with the response
    fn update_context_with_response(
        &self, 
        context: &mut ExecutionContext, 
        response: ChatResponse
    ) -> Result<(), OrchestrationError> {
        // Extract text content from response
        let content = match &response.message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::MultiModal(parts) => {
                parts.iter()
                    .filter_map(|part| {
                        match part {
                            ContentPart::Text(text) => Some(text.clone()),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        };
        
        // Update output variable
        context.set_variable(&self.config.output_variable, VariableValue::String(content));
        
        // Store any tool calls if present
        if let Some(tool_calls) = &response.message.tool_calls {
            context.set_variable(
                &format!("{}_tool_calls", self.config.output_variable),
                VariableValue::Json(serde_json::to_value(tool_calls).unwrap()),
            );
        }
        
        // Store token usage if available
        if let Some(usage) = response.usage {
            context.set_variable(
                &format!("{}_token_usage", self.config.output_variable),
                VariableValue::Json(serde_json::to_value(usage).unwrap()),
            );
        }
        
        Ok(())
    }
}
```

## 7. Examples

### 7.1 Basic Usage

```rust
use cogni::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a provider
    let provider = OpenAiProvider::new(std::env::var("OPENAI_API_KEY")?);
    
    // Create a chat request using Default for optional fields
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hello, how are you?".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = provider.chat(&request).await?;
    
    // Extract the response content
    match response.message.content {
        MessageContent::Text(text) => println!("Response: {}", text),
        MessageContent::MultiModal(_) => println!("Received multimodal response"),
    }
    
    Ok(())
}
```

### 7.2 MCP Tool Integration with HTTP Example

```rust
use cogni::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an MCP tool client that uses HTTP
    let mcp_client = McpToolClient::new_http("http://localhost:8080/mcp")
        .with_auth_token(std::env::var("MCP_AUTH_TOKEN")?);
    
    // Create a provider with the MCP tool client
    let provider = OpenAiProvider::new(std::env::var("OPENAI_API_KEY")?)
        .with_tool_executor(mcp_client);
    
    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text("You are an assistant that uses tools to help the user.".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("What's the weather in New York today?".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = provider.chat(&request).await?;
    
    // Extract the response content
    match response.message.content {
        MessageContent::Text(text) => println!("Response: {}", text),
        MessageContent::MultiModal(_) => println!("Received multimodal response"),
    }
    
    Ok(())
}
```

### 7.3 MCP Tool Integration with Stdio Example

```rust
use cogni::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an MCP tool client that communicates with a local process
    let mcp_client = McpToolClient::new_process(
        "python", 
        &["mcp_server.py", "--tools", "weather,calculator"]
    )?;
    
    // Create a provider with the MCP tool client
    let provider = OpenAiProvider::new(std::env::var("OPENAI_API_KEY")?)
        .with_tool_executor(mcp_client);
    
    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text("You are an assistant that uses tools to help the user.".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("What's the weather in New York today?".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = provider.chat(&request).await?;
    
    // Extract the response content
    match response.message.content {
        MessageContent::Text(text) => println!("Response: {}", text),
        MessageContent::MultiModal(_) => println!("Received multimodal response"),
    }
    
    Ok(())
}
```

### 7.4 Custom Tool Executor Example

```rust
use cogni::prelude::*;
use async_trait::async_trait;

/// Custom tool executor implementation
struct LocalToolExecutor {
    tools: Vec<Tool>,
}

impl LocalToolExecutor {
    fn new() -> Self {
        // Define available tools
        let calculator_tool = Tool {
            r#type: ToolType::Function,
            function: Function {
                name: "calculator".to_string(),
                description: Some("A calculator that can add, subtract, multiply, and divide".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"]
                        },
                        "a": {
                            "type": "number"
                        },
                        "b": {
                            "type": "number"
                        }
                    },
                    "required": ["operation", "a", "b"]
                }),
            },
        };
        
        Self {
            tools: vec![calculator_tool],
        }
    }
}

#[async_trait]
impl ToolExecutor for LocalToolExecutor {
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
        match name {
            "calculator" => {
                let args: serde_json::Value = serde_json::from_str(arguments)?;
                
                let operation = args["operation"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing operation".to_string())
                })?;
                
                let a = args["a"].as_f64().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing or invalid 'a' parameter".to_string())
                })?;
                
                let b = args["b"].as_f64().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing or invalid 'b' parameter".to_string())
                })?;
                
                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b == 0.0 {
                            return Err(ToolError::ExecutionFailed("Division by zero".to_string()));
                        }
                        a / b
                    },
                    _ => return Err(ToolError::InvalidArguments(format!("Unknown operation: {}", operation))),
                };
                
                Ok(result.to_string())
            },
            _ => Err(ToolError::ToolNotFound(format!("Unknown tool: {}", name))),
        }
    }
    
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
        Ok(self.tools.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a tool executor
    let tool_executor = LocalToolExecutor::new();
    
    // Create a provider with the tool executor
    let provider = OpenAiProvider::new_with_tools(
        std::env::var("OPENAI_API_KEY")?,
        tool_executor
    );
    
    // Create a chat request (using Default for optional fields)
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("What is 42 * 56?".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = provider.chat(&request).await?;
    
    println!("Response: {:?}", response.message.content);
    
    Ok(())
}
```

### 7.5 Streaming Example

```rust
use cogni::prelude::*;
use futures::StreamExt;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a provider
    let provider = AnthropicProvider::new(std::env::var("ANTHROPIC_API_KEY")?);
    
    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Write a short poem about Rust programming language".to_string()),
                ..Default::default()
            }
        ],
        model: "claude-3-opus-20240229".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Stream the response
    let mut stream = provider.stream_chat(&request).await?;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let MessageContent::Text(text) = chunk.delta.content {
                    print!("{}", text);
                    std::io::stdout().flush()?;
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    println!();
    
    Ok(())
}
```

### 7.6 Combined MCP and Custom Tools Example

```rust
use cogni::prelude::*;
use async_trait::async_trait;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a combined tool executor
    struct CombinedToolExecutor {
        mcp_client: McpToolClient<McpStdioTransport>,
        calculator_tool: Tool,
    }
    
    impl CombinedToolExecutor {
        fn new(command: &str, args: &[&str]) -> Result<Self, ToolError> {
            let mcp_client = McpToolClient::new_process(command, args)?;
            
            let calculator_tool = Tool {
                r#type: ToolType::Function,
                function: Function {
                    name: "calculator".to_string(),
                    description: Some("A calculator for basic math operations".to_string()),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "operation": {
                                "type": "string",
                                "enum": ["add", "subtract", "multiply", "divide"]
                            },
                            "a": { "type": "number" },
                            "b": { "type": "number" }
                        },
                        "required": ["operation", "a", "b"]
                    }),
                },
            };
            
            Ok(Self {
                mcp_client,
                calculator_tool,
            })
        }
    }
    
    #[async_trait]
    impl ToolExecutor for CombinedToolExecutor {
        async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError> {
            match name {
                "calculator" => {
                    let args: serde_json::Value = serde_json::from_str(arguments)?;
                    let operation = args["operation"].as_str().ok_or_else(|| {
                        ToolError::InvalidArguments("Missing operation".to_string())
                    })?;
                    
                    let a = args["a"].as_f64().ok_or_else(|| {
                        ToolError::InvalidArguments("Missing or invalid 'a' parameter".to_string())
                    })?;
                    
                    let b = args["b"].as_f64().ok_or_else(|| {
                        ToolError::InvalidArguments("Missing or invalid 'b' parameter".to_string())
                    })?;
                    
                    let result = match operation {
                        "add" => a + b,
                        "subtract" => a - b,
                        "multiply" => a * b,
                        "divide" => {
                            if b == 0.0 {
                                return Err(ToolError::ExecutionFailed("Division by zero".to_string()));
                            }
                            a / b
                        },
                        _ => return Err(ToolError::InvalidArguments(format!("Unknown operation: {}", operation))),
                    };
                    
                    Ok(result.to_string())
                },
                // All other tools are delegated to MCP
                _ => self.mcp_client.execute_tool(name, arguments).await,
            }
        }
        
        async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError> {
            // Get MCP tools
            let mut tools = match self.mcp_client.get_available_tools().await {
                Ok(t) => t,
                Err(_) => Vec::new(), // If MCP fails, just use local tools
            };
            
            // Add calculator tool
            tools.push(self.calculator_tool.clone());
            
            Ok(tools)
        }
    }
    
    // Create the combined executor
    let combined_executor = CombinedToolExecutor::new(
        "python", 
        &["mcp_server.py", "--tools", "weather,news"]
    )?;
    
    // Create a provider with the combined executor
    let provider = OpenAiProvider::new_with_tools(
        std::env::var("OPENAI_API_KEY")?,
        combined_executor
    );
    
    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("What's 42 * 56 and what's the weather in New York?".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = provider.chat(&request).await?;
    
    println!("Response: {:?}", response.message.content);
    
    Ok(())
}
```

### 7.7 Error Handling and Retries Example

```rust
use cogni::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a basic provider
    let base_provider = OpenAiProvider::new(std::env::var("OPENAI_API_KEY")?);
    
    // Wrap it with rate limiting
    let rate_limited = RateLimitedProvider::new(base_provider, 60); // 60 requests per minute
    
    // Wrap again with retries
    let provider = RetryingProvider::new(rate_limited, 3) // Retry up to 3 times
        .with_retry_delay_ms(500); // Start with 500ms delay (will increase exponentially)
    
    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Tell me about Rust programming language".to_string()),
                ..Default::default()
            }
        ],
        model: "gpt-4".to_string(),
        ..Default::default()
    };
    
    // Send the request - it will automatically retry on transient errors
    match provider.chat(&request).await {
        Ok(response) => {
            if let MessageContent::Text(text) = response.message.content {
                println!("Response: {}", text);
            }
        },
        Err(e) => {
            eprintln!("Error after retries: {}", e);
            
            // Handle specific error types
            match e {
                LlmError::RateLimitExceeded => {
                    eprintln!("Rate limit exceeded. Consider reducing request frequency.");
                },
                LlmError::Timeout => {
                    eprintln!("Request timed out. The server might be overloaded.");
                },
                LlmError::ApiError(msg) => {
                    eprintln!("API error: {}", msg);
                },
                _ => {
                    eprintln!("Unexpected error: {:?}", e);
                }
            }
        }
    }
    
    Ok(())
}
```

## 8. Future Directions

### 8.1 Potential Improvements
1. **Fine-grained Permissions System**: Implement a permissions system to control which operations and models a client can access.
2. **Caching Layer**: Add support for caching responses to reduce API costs and improve performance.
3. **Metrics and Telemetry**: Integrate with popular observability frameworks for tracking usage, latency, and errors.
4. **Provider Discovery**: Implement a discovery mechanism for dynamically finding and registering providers.
5. **Context Window Management**: Intelligent handling of context window limitations, including automatic token counting and message pruning.
6. **MCP Server Implementation**: Create a module for implementing MCP servers, not just clients.
7. **Enhanced Ollama Integration**: Improve the Ollama provider to better support model management and advanced features.
8. **Streaming Tool Calls**: Support for tool calls in streaming responses, which would require more complex stream processing.
9. **Token Management**: Add utilities for tracking token usage and optimizing requests to stay within limits.
10. **Multi-modal Support**: Enhance support for image, audio, and video inputs and outputs.
11. **MCP Protocol Standardization**: Contribute to standardizing the MCP protocol for better interoperability.

### 8.2 Extensibility Points
1. **Custom Tool Executor Registration**: Allow users to register custom tool executors at runtime.
2. **Middleware System**: Implement a middleware system for intercepting and modifying requests and responses.
3. **Plugin Architecture**: Develop a plugin system for extending core functionality without modifying the main library.
4. **Provider Composition**: Enable composition of multiple providers for advanced scenarios like fallback, load balancing, and A/B testing.
5. **Custom Serialization**: Allow customization of request/response serialization for provider-specific requirements.
6. **Event Hooks**: Add hooks for events like before/after requests, tool calls, etc.
7. **Custom MCP Transports**: Allow users to implement additional MCP transport mechanisms.

## 9. Conclusion

The Cogni crate provides a flexible, type-safe, and efficient abstraction for interacting with Large Language Models. By leveraging Rust's strengths in concurrency, error handling, and type safety, it offers a robust foundation for building LLM-powered applications.

Key strengths of the design include:
- Clean separation of concerns between the orchestration library and LLM interactions
- Comprehensive support for modern LLM features like tool calling and streaming
- Extensibility through traits, generics, and provider implementations
- Strong error handling and recovery mechanisms
- Efficient asynchronous operations with async/await
- Proper integration with MCP servers for external tool access via both HTTP and stdio
- Unified API for handling both simple text interactions and complex tool-calling scenarios
- Support for local models exclusively through Ollama integration
- Ergonomic API through the use of the `Default` trait and builder patterns
- Type-safe composability through generics rather than runtime boxing
- Improved memory usage through reference-based APIs

This design aligns perfectly with the orchestration library's focus on flexibility, minimalism, and asynchronous operations, creating a powerful ecosystem for building complex LLM applications.
