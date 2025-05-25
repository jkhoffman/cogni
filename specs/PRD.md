# Cogni: Universal LLM Client Library for Rust  
## Project Requirements Document (PRD)

**Version:** 2.0  
**Date:** May 14, 2025  
**Status:** Revised

## 1. Introduction

### 1.1. Purpose  
Cogni is an async-first Rust client library designed to provide a unified, ergonomic, and type-safe interface for interacting with a variety of Large Language Model (LLM) providers, including OpenAI, Anthropic, Google AI, Ollama, and others.

### 1.2. Problem Statement  
Rust developers currently face a fragmented landscape when integrating LLMs into their applications. Each LLM provider offers its own SDK or API specifics, leading to:  
* Increased development time to learn and implement multiple provider interfaces.  
* Difficulty in switching between or experimenting with different LLM providers.  
* Inconsistent error handling, data structures, and API patterns.  
* Boilerplate code for common tasks like request building, streaming, and authentication.

Ollama Oxide demonstrated the value of a well-structured Rust client for a single provider (Ollama). Cogni aims to generalize this approach to create a universal LLM access layer for the Rust ecosystem.

### 1.3. Vision  
Cogni will empower Rust developers to seamlessly integrate and leverage the capabilities of diverse LLMs, fostering innovation and simplifying the development of AI-powered applications. It will be the go-to Rust library for robust, consistent, and developer-friendly LLM API interactions.

### 1.4. Relationship to Orchestration Library
Cogni is designed to work harmoniously with the orchestration library. While the orchestration library handles the execution flow, state management, and coordination of tasks, Cogni focuses specifically on providing a unified interface for LLM interactions. This separation of concerns allows each component to excel in its domain.

### 1.5. Support Scope
Cogni will support the following LLM providers:
- Cloud-based APIs: OpenAI, Anthropic, and others
- Local models: Exclusively through Ollama integration
- External tools: Via Model Context Protocol (MCP) servers

## 2. Goals

### 2.1. Project Goals  
* **Unified Interface:** Provide a single, consistent API for common LLM operations across multiple providers.  
* **Developer Ergonomics:** Offer a fluent, type-safe, and easy-to-use Rust library that minimizes boilerplate and cognitive overhead.  
* **Extensibility:** Design an architecture that allows for straightforward addition of new LLM providers and features in the future.  
* **Robustness:** Implement comprehensive error handling and ensure reliability when interacting with external APIs.  
* **Asynchronous Performance:** Leverage Rust's async capabilities for efficient, non-blocking I/O.  
* **Modern Rust Practices:** Utilize current Rust idioms, features (e.g., `async-trait`), and best practices for library development.

### 2.2. MVP Goals  
* Successfully implement core LLM functionalities (generate, chat, embeddings, model listing, and basic function/tool usage) for at least three major providers (OpenAI, Anthropic, and Ollama) to validate the abstraction layer.  
* Provide a clear and usable API for developers to configure and interact with these providers.  
* Ensure robust streaming capabilities for generation and chat.  
* Establish a solid architectural foundation for future expansion.  
* Deliver well-documented code and examples for MVP features.
* Implement basic error handling and retry mechanisms.
* Support for MCP tool integration for external tools.

## 3. Target Audience

* **Rust Application Developers:** Building applications (web services, CLIs, desktop apps, etc.) that require LLM capabilities.  
* **AI/ML Researchers & Engineers:** Using Rust for prototyping, experimentation, or building LLM-powered tools and requiring access to various models.  
* **Teams Evaluating LLMs:** Needing a simple way to benchmark or switch between different LLM providers with minimal code changes.  
* **Developers Seeking Abstraction:** Wanting to abstract away the specific details of individual LLM provider SDKs.

## 4. Project Scope

### 4.1. MVP - In Scope

* **Core Architecture & Design:**
    * Trait-based abstraction pattern with a core `LlmProvider` trait defining the interface for all LLM interactions
    * Type safety through newtype wrappers for primitive types like `ApiKey`, `BaseUrl`, and `OrganizationId`
    * Provider factory pattern for simplified provider instantiation
    * Enhanced error handling with retry mechanisms and rate limiting

* **Supported Providers (Initial Set):**  
    * OpenAI  
    * Anthropic  
    * Ollama (exclusive method for local model integration)

* **Tool Integration:**
    * Model Context Protocol (MCP) support for external tool execution
    * Custom tool executors through the `ToolExecutor` trait

* **Core LLM Operations:**  
    * **Text Generation:**  
        * Non-streaming (full response).  
        * Streaming (chunk-by-chunk response).  
        * Support for common generation parameters (temperature, max tokens, stop sequences) via a common configuration structure.  
    * **Chat Completions:**  
        * Non-streaming (full response).  
        * Streaming (delta-by-delta response).  
        * Support for message history with distinct roles (System, User, Assistant, Tool).  
        * Support for common chat parameters.  
        * **Function/Tool Calling:**  
            * Define common structures for tool definitions.  
            * Ability for the LLM to request tool calls.  
            * Ability for the client to send tool results back to the LLM.  
            * Initial implementation for at least one provider that strongly supports this (e.g., OpenAI).  
    * **Embeddings:**  
        * Generation of text embeddings for single and batch inputs.  
        * Support for specifying embedding models.  
    * **Model Information:**  
        * Ability to list available models for a configured provider.  
        * Return standardized `ModelInfo` (ID, provider, capabilities).  

* **Client Architecture & API Design:**  
    * Unified library structure with modules for core traits, provider implementations, and tool executors
    * Provider-specific client implementations.  
    * Unified API traits in core module with `Send`-able futures
    * Standardized core data structures with support for multi-modal content
    
* **Client Configuration:**  
    * Secure management of API keys per provider
    * Support for overriding base URLs for providers.  
    * Basic client-level configuration (e.g., request timeouts).
    
* **Error Handling:**  
    * Unified error type system
    * Clear distinction between client-side errors and provider API errors
    * Retry mechanism for handling transient errors
    * Rate limiting support

* **Streaming:**  
    * Robust handling of streaming responses, leveraging existing Rust crates for SSE and JSONL parsing per provider needs.
    
* **Batch Processing:**
    * Utilities for concurrent execution with controlled parallelism
    
* **Testing:**  
    * Integration tests against the live APIs of the supported MVP providers.  
    * API keys for testing managed via environment variables.
    
* **Basic Documentation & Examples:**  
    * README with setup and basic usage.  
    * Examples for each core feature and supported provider.

### 4.2. MVP - Out of Scope

* **Advanced Model Management:** Operations like pulling, deleting, creating, or copying models (beyond listing).
* **Complex Mocking Framework:** A comprehensive mock client is deferred post-MVP.  
* **Direct Local Model Support:** Local models are only supported through Ollama integration.
* **Hyper-Optimization:** Initial focus is on correctness and API design.  
* **Automatic Provider-Specific Option Mapping:** Complex mapping of all unique provider options to a common structure. Provider-specific options will initially be handled via a generic `options: Option<serde_json::Value>` field or similar.  
* **Support for all known LLM providers:** The MVP will target a small, representative set.  
* **Advanced caching mechanisms:** Basic retry and rate limiting are included, but complex caching is post-MVP.
* **GUI or CLI tools based on the library.**  
* **Fine-Tuning APIs.**

### 4.3. Future Considerations (Post-MVP)  
* Support for more LLM providers (Google Gemini, Cohere, Mistral, etc.).  
* Comprehensive mocking framework.  
* Advanced multimodal capabilities (extending beyond the basic support in MVP).
* Full model lifecycle management features.  
* Configurable advanced retry policies.
* Observability hooks (e.g., `tracing`).  
* Helper utilities for prompt templating.
* Fine-grained permissions system.
* Advanced caching layer.
* Provider discovery mechanisms.
* Context window management.
* MCP server implementation (not just client).
* Token management utilities.
* Middleware and plugin architecture.

## 5. Core Features & Functionality (MVP Details)

### 5.1. Unified API Design & Architecture  

* **Core Modules:**
    * **Core traits and data structures:**
        * Provider traits: `LlmProvider`, `ToolExecutor`
        * Error types: `LlmError`, `ToolError`
        * Newtype wrappers: `ApiKey`, `BaseUrl`, `OrganizationId`
    * **Standardized Data Structures:**
        * Chat: `ChatMessage`, `MessageRole`, `ChatRequest`, `ChatResponse`
        * Completion: `CompletionRequest`, `CompletionResponse`
        * Embedding: `EmbeddingRequest`, `EmbeddingResponse`
        * Tool calling: `Tool`, `Function`, `ToolCall`, `ToolCallFunction`
        * Multi-modal: `MessageContent`, `ContentPart`, `ImageData`, `AudioData`
    * **Provider implementations:**
        * Provider-specific clients: `OpenAiProvider`, `AnthropicProvider`, `OllamaProvider`
        * Provider factory for creating providers from configuration
    * **Tool executors:**
        * MCP client implementation for external tool integration
        * Custom tool executor support
    * **Utility modules:**
        * Rate limiting and retry mechanisms
        * Batch processing utilities

### 5.2. LlmProvider Trait
The core `LlmProvider` trait defines the interface for all LLM interactions:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// The type of response chunk returned when streaming chat completions
    type ChatChunk: Send + 'static;
    
    /// Execute a chat completion request
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError>;
    
    /// Execute a text completion request
    async fn completion(&self, request: &CompletionRequest) -> Result<CompletionResponse, LlmError>;
    
    /// Generate embeddings for given input
    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, LlmError>;
    
    /// Stream a chat completion response
    async fn stream_chat<'a>(
        &'a self, 
        request: &'a ChatRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::ChatChunk, LlmError>> + Send + 'a>>, LlmError>;
    
    /// Returns the supported models by this provider
    fn supported_models(&self) -> Vec<String>;
    
    /// Check if a specific feature is supported by this provider
    fn supports_feature(&self, feature: ProviderFeature) -> bool;
}
```

### 5.3. ToolExecutor Trait
The `ToolExecutor` trait defines the interface for executing external tools, including those exposed by MCP servers:

```rust
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool with the given name and arguments
    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<String, ToolError>;
    
    /// Get a list of available tools
    async fn get_available_tools(&self) -> Result<Vec<Tool>, ToolError>;
}
```

### 5.4. Standard Data Structures

* **Message Structures:**
```rust
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: MessageContent,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

pub enum MessageContent {
    Text(String),
    MultiModal(Vec<ContentPart>),
}

pub enum ContentPart {
    Text(String),
    Image(ImageData),
    Audio(AudioData),
}
```

* **Request Structures:**
```rust
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>,
    pub provider_options: Option<serde_json::Value>,
}

pub struct CompletionRequest {
    pub prompt: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub provider_options: Option<serde_json::Value>,
}

pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
    pub provider_options: Option<serde_json::Value>,
}
```

* **Response Structures:**
```rust
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<TokenUsage>,
    pub model: String,
    pub provider_metadata: Option<serde_json::Value>,
}

pub struct CompletionResponse {
    pub text: String,
    pub usage: Option<TokenUsage>,
    pub model: String,
    pub provider_metadata: Option<serde_json::Value>,
}

pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub usage: Option<TokenUsage>,
    pub model: String,
    pub provider_metadata: Option<serde_json::Value>,
}
```

### 5.5. Error Handling
A comprehensive error system that captures provider-specific errors while providing a uniform interface:

```rust
pub enum LlmError {
    ApiError(String),
    AuthError(String),
    RateLimitExceeded,
    InvalidRequest(String),
    ModelNotFound(String),
    NetworkError(#[from] reqwest::Error),
    ConfigurationError(String),
    ProviderSpecific(String),
    UnsupportedFeature(String),
    SerializationError(#[from] serde_json::Error),
    StreamError(String),
    ToolExecutionError(String),
    Timeout,
}
```

### 5.6. Provider Factory Pattern
A factory pattern to simplify provider instantiation based on configuration:

```rust
pub enum ProviderType {
    OpenAi,
    Anthropic,
    Ollama,
    Custom(String),
}

pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<ApiKey>,
    pub base_url: Option<BaseUrl>,
    pub organization: Option<OrganizationId>,
    pub tool_executor_config: Option<ToolExecutorConfig>,
    pub additional_options: HashMap<String, String>,
}

pub fn create_provider(
    config: ProviderConfig
) -> Result<Box<dyn LlmProvider<ChatChunk = ChatResponseChunk> + Send + Sync>, LlmError> {
    // Implementation details...
}
```

### 5.7. Advanced Features

* **Retrying Provider Wrapper:**
```rust
pub struct RetryingProvider<P> {
    inner: P,
    max_retries: u32,
    retry_delay_ms: u64,
    retryable_errors: Vec<RetryableErrorType>,
}
```

* **Rate Limiting Wrapper:**
```rust
pub struct RateLimitedProvider<P> {
    inner: P,
    rate_limiter: RateLimiter,
}
```

* **Batch Processing Utility:**
```rust
pub async fn process_batch<P, T>(
    provider: &P,
    requests: &[ChatRequest],
    max_concurrency: usize
) -> Result<Vec<Result<ChatResponse, LlmError>>, LlmError> 
```

## 6. Technical Design Principles (High-Level)

* **Language:** Rust (latest stable edition, leveraging `async-trait`).  
* **Primary Async Runtime:** Tokio.  
* **HTTP Client:** `reqwest` will be used initially within each provider client. 
* **Streaming Support:** Each provider client will integrate appropriate parsers for its streaming format (e.g., `reqwest-eventsource` for SSE, or a JSONL parser).  
* **Architecture Pattern:** Trait-based abstraction with generic implementations and type-safe composition.
* **Type Safety:** Use of newtype wrappers and associated types to improve type safety and correctness.
* **Error Handling:** Comprehensive error types with support for retry mechanisms and provider-specific errors.
* **Modularity:** Logical module organization with clear separation of concerns.
* **Dependencies:** Minimize dependencies in core modules. Provider-specific dependencies are confined to provider implementation modules.
* **Testing:** Integration tests against live APIs are the primary testing method for MVP.  
* **Cross-Platform Compatibility:** Ensure functionality on Linux, macOS, and Windows.  
* **`Send` Compatibility:** All public async trait methods must return `Send`-able futures.

## 7. Success Metrics (MVP)

* Developers can successfully perform generate, chat (with basic tool usage for at least one provider), and embedding operations using at least three different LLM providers (OpenAI, Anthropic, and Ollama) through a single, unified interface.  
* MCP tool integration works correctly for external tool execution.
* Streaming functionality for generate and chat works reliably for the supported providers.  
* Developers can list available models for the configured providers.  
* API key configuration is straightforward and secure (via environment variables or direct config).  
* Error messages are clear, informative, and distinguish between client-side and provider-side issues, including provider-specific details.  
* Retry mechanisms handle transient errors effectively.
* Rate limiting prevents API quota exhaustion.
* The library is published on crates.io with basic documentation and examples.  
* Integration tests against live provider APIs pass consistently for MVP features.  
* The core abstractions are proven to be sufficiently general by supporting the initial set of providers.

## 8. Release Criteria (MVP)

* All features defined in "MVP - In Scope" (Section 4.1) are implemented for the selected initial providers (OpenAI, Anthropic, Ollama).  
* Core API traits and data structures are considered stable for the MVP feature set.  
* MCP tool integration is tested and working correctly.
* Retry and rate limiting mechanisms are implemented and tested.
* Comprehensive integration tests pass for all supported features and providers.  
* Basic documentation (README, getting started guide, API key setup guide, and code examples for each core feature) is complete and published.  
* CI pipeline is established for building and running tests (tests requiring API keys may initially be run in a controlled/manual fashion if secure CI key management is complex for MVP).

## 9. Open Questions & Risks

* **API Uniformity vs. Provider Specificity:** Balancing common structures with `provider_specific_options` for unique features.  
* **Authentication Diversity:** MVP focuses on API keys; future providers may need OAuth, etc.  
* **Rate Limiting & Quota Handling:** MVP implements basic client-side rate limiting; more advanced quota management is post-MVP.
* **Model Capability Granularity:** `supported_models` and `supports_feature` provide basic info; detailed capability discovery is complex.
* **Testing Costs and Reliability:** Live API testing has costs and provider dependencies.  
* **Maintenance of Provider Clients:** Keeping clients updated with provider API changes.  
* **Complexity of Streaming Parsers:** Robustly handling diverse streaming formats.  
* **Function/Tool Calling Abstraction:** Ensuring the common tool structures can adequately represent the mechanisms of different providers (e.g., OpenAI's JSON mode for functions vs. Anthropic's XML-based approach).
* **MCP Protocol Stability:** The Model Context Protocol may evolve, requiring adaptations to the MCP client implementation.
* **Composition Overhead:** The generic composition pattern may lead to complex type signatures; future versions may need to optimize for usability.
* **Dynamic Tool Executor Registration:** Finding the right balance between type safety and runtime flexibility for custom tool executors.