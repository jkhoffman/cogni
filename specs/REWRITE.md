# Cogni Ground-Up Rewrite Specification

## Table of Contents

1. [Vision & Goals](#vision--goals)
2. [Architecture Design](#architecture-design)
3. [Core Abstractions](#core-abstractions)
4. [Module Design](#module-design)
5. [Implementation Details](#implementation-details)
6. [Performance Architecture](#performance-architecture)
7. [Testing Strategy](#testing-strategy)
8. [Development Roadmap](#development-roadmap)

## Vision & Goals

### Vision
Build the most elegant, performant, and extensible Rust library for LLM interactions, designed from first principles without legacy constraints.

### Primary Goals
1. **Simplicity**: Minimal API surface with maximum capability
2. **Performance**: Zero-cost abstractions and optimal resource usage
3. **Composability**: Small, focused components that combine powerfully
4. **Type Safety**: Leverage Rust's type system to prevent errors at compile time
5. **Extensibility**: Easy to add providers, middleware, and tools without modifying core

### Design Philosophy
- Start with the simplest possible abstractions
- Add complexity only when necessary
- Every design decision must have a clear rationale
- Performance is a feature, not an afterthought
- Developer experience is paramount

## Architecture Design

### Crate Structure

```
cogni/
├── cogni-core/           # Core traits and types (no dependencies)
├── cogni-http/           # HTTP transport layer
├── cogni-providers/      # Provider implementations
├── cogni-middleware/     # Middleware system
├── cogni-tools/          # Tool execution
├── cogni-stream/         # Streaming utilities
├── cogni-client/         # High-level client API
└── cogni/                # Main crate re-exporting everything
```

**Note**: This structure could be simplified by merging:
- `cogni-stream` into `cogni-core` (streaming is fundamental)
- `cogni-http` into `cogni-providers` (all current providers use HTTP)

This would reduce to 6 crates while maintaining clean separation.

### Dependency Principles

1. **cogni-core** has zero external dependencies
2. Each crate depends only on what it needs
3. Circular dependencies are forbidden
4. Optional features for heavy dependencies

### Layer Architecture

```
┌─────────────────────────────────────┐
│          Application Code           │
├─────────────────────────────────────┤
│         cogni-client               │
├─────────────────────────────────────┤
│   cogni-middleware │ cogni-tools   │
├─────────────────────────────────────┤
│        cogni-providers             │
├─────────────────────────────────────┤
│   cogni-stream   │   cogni-http    │
├─────────────────────────────────────┤
│          cogni-core                │
└─────────────────────────────────────┘
```

## Core Abstractions

### cogni-core: Pure Abstractions

```rust
// cogni-core/src/provider.rs
use futures::Stream;
use std::pin::Pin;

/// The fundamental trait for LLM interactions
pub trait Provider: Send + Sync {
    type Stream: Stream<Item = Result<StreamEvent, Error>> + Send + Unpin;
    
    /// Send a request and get a response
    async fn request(&self, request: Request) -> Result<Response, Error>;
    
    /// Send a request and get a stream of events
    async fn stream(&self, request: Request) -> Result<Self::Stream, Error>;
}

// cogni-core/src/types.rs

/// A request to an LLM
#[derive(Debug, Clone)]
pub struct Request {
    pub messages: Vec<Message>,
    pub model: Model,
    pub parameters: Parameters,
    pub tools: Vec<Tool>,
}

/// A message in a conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Content,
    pub metadata: Metadata,
}

/// Content can be text, images, or other media
#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image(Image),
    Audio(Audio),
    Multiple(Vec<Content>),
}

/// Streaming events
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Content(ContentDelta),
    ToolCall(ToolCallDelta),
    Metadata(MetadataDelta),
    Done,
}

// cogni-core/src/error.rs

/// All errors in the system
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
}
```

### cogni-http: Transport Layer

```rust
// cogni-http/src/client.rs

/// HTTP client abstraction
pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, body: Value) -> Result<Response, Error>;
    async fn stream(&self, url: &str, body: Value) -> Result<ResponseStream, Error>;
}

/// Default implementation using reqwest
pub struct ReqwestClient {
    client: reqwest::Client,
}

// cogni-http/src/auth.rs

/// Authentication methods
pub enum Auth {
    Bearer(String),
    Basic { username: String, password: String },
    Custom(Box<dyn AuthProvider>),
}

pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, request: &mut reqwest::Request) -> Result<(), Error>;
}
```

### cogni-providers: Clean Provider Implementations

```rust
// cogni-providers/src/openai.rs

pub struct OpenAI {
    client: Arc<dyn HttpClient>,
    config: OpenAIConfig,
}

impl Provider for OpenAI {
    type Stream = OpenAIStream;
    
    async fn request(&self, request: Request) -> Result<Response, Error> {
        let body = self.convert_request(request).await?;
        let response = self.client.post(&self.config.url, body).await?;
        self.parse_response(response).await
    }
    
    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        let body = self.convert_request(request).await?;
        let stream = self.client.stream(&self.config.url, body).await?;
        Ok(OpenAIStream::new(stream))
    }
}

// cogni-providers/src/traits.rs
use async_trait::async_trait;

/// Shared traits for provider implementations
#[async_trait]
trait RequestConverter {
    async fn convert_request(&self, request: Request) -> Result<Value, Error>;
}

#[async_trait]
trait ResponseParser {
    async fn parse_response(&self, response: Value) -> Result<Response, Error>;
}

trait StreamEventParser {
    fn parse_event(&self, data: &str) -> Result<Option<StreamEvent>, Error>;
}
```

### cogni-middleware: Composable Processing

```rust
// cogni-middleware/src/lib.rs

/// Middleware that can process requests and responses
pub trait Middleware: Send + Sync {
    /// Process a request before sending
    async fn process_request(&self, request: Request) -> Result<Request, Error> {
        Ok(request)
    }
    
    /// Process a response after receiving
    async fn process_response(&self, response: Response) -> Result<Response, Error> {
        Ok(response)
    }
    
    /// Process streaming events
    async fn process_event(&self, event: StreamEvent) -> Result<StreamEvent, Error> {
        Ok(event)
    }
}

/// A provider wrapped with middleware
pub struct MiddlewareProvider<P: Provider> {
    inner: P,
    middleware: Vec<Box<dyn Middleware>>,
}

impl<P: Provider> Provider for MiddlewareProvider<P> {
    type Stream = MiddlewareStream<P::Stream>;
    
    async fn request(&self, request: Request) -> Result<Response, Error> {
        // Apply middleware in order for requests
        let mut processed_request = request;
        for m in &self.middleware {
            processed_request = m.process_request(processed_request).await?;
        }
        
        // Call inner provider
        let response = self.inner.request(processed_request).await?;
        
        // Apply middleware in reverse order for responses
        let mut response = response;
        for m in self.middleware.iter().rev() {
            response = m.process_response(response).await?;
        }
        
        Ok(response)
    }
}
```

### cogni-tools: Function Execution

```rust
// cogni-tools/src/lib.rs
use std::future::Future;

/// Execute tools/functions
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, Error>;
    fn describe(&self) -> Vec<Tool>;
}

/// Simple function-based executor
pub struct FunctionExecutor<F> {
    function: F,
    description: Tool,
}

impl<F, Fut> ToolExecutor for FunctionExecutor<F>
where
    F: Fn(Value) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Value, Error>> + Send + 'static,
{
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, Error> {
        let args = serde_json::from_str(&call.arguments)?;
        let result = (self.function)(args).await?;
        Ok(ToolResult {
            call_id: call.id,
            content: serde_json::to_string(&result)?,
        })
    }
}

/// Combine multiple executors
pub struct ToolRegistry {
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
}
```

### cogni-stream: Streaming Utilities

```rust
// cogni-stream/src/lib.rs

/// Accumulate streaming events into complete responses
pub struct StreamAccumulator {
    content: String,
    tool_calls: Vec<ToolCall>,
    metadata: Metadata,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn process_event(&mut self, event: StreamEvent) -> Result<(), Error> {
        match event {
            StreamEvent::Content(delta) => self.content.push_str(&delta.text),
            StreamEvent::ToolCall(delta) => self.merge_tool_call(delta)?,
            StreamEvent::Metadata(delta) => self.merge_metadata(delta),
            StreamEvent::Done => {},
        }
        Ok(())
    }
    
    pub fn finalize(self) -> Response {
        Response {
            content: self.content,
            tool_calls: self.tool_calls,
            metadata: self.metadata,
        }
    }
}

/// Transform streams
pub trait StreamTransformer: Send + Sync {
    type Output;
    
    fn transform(&mut self, event: StreamEvent) -> Option<Self::Output>;
    fn finalize(&mut self) -> Option<Self::Output>;
}
```

### cogni-client: High-Level API

```rust
// cogni-client/src/lib.rs
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// High-level client for LLM interactions
pub struct Client<P: Provider> {
    provider: P,
    default_model: Model,
    default_parameters: Parameters,
}

impl<P: Provider> Client<P> {
    /// Create a new client
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            default_model: Model::default(),
            default_parameters: Parameters::default(),
        }
    }
    
    /// Simple chat interface
    pub async fn chat(&self, messages: impl Into<Vec<Message>>) -> Result<String, Error> {
        let request = Request {
            messages: messages.into(),
            model: self.default_model.clone(),
            parameters: self.default_parameters.clone(),
            tools: Vec::new(),
        };
        
        let response = self.provider.request(request).await?;
        Ok(response.content)
    }
    
    /// Streaming chat interface
    pub async fn stream_chat(
        &self,
        messages: impl Into<Vec<Message>>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, Error>> + Send>>, Error> {
        let request = Request {
            messages: messages.into(),
            model: self.default_model.clone(),
            parameters: self.default_parameters.clone(),
            tools: Vec::new(),
        };
        
        let stream = self.provider.stream(request).await?;
        Ok(Box::pin(stream.filter_map(|event| async move {
            match event {
                Ok(StreamEvent::Content(delta)) => Some(Ok(delta.text)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        })))
    }
}

/// Builder for complex scenarios
pub struct RequestBuilder {
    messages: Vec<Message>,
    model: Option<Model>,
    parameters: Parameters,
    tools: Vec<Tool>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn message(mut self, role: Role, content: impl Into<Content>) -> Self {
        self.messages.push(Message {
            role,
            content: content.into(),
            metadata: Metadata::default(),
        });
        self
    }
    
    pub fn model(mut self, model: Model) -> Self {
        self.model = Some(model);
        self
    }
    
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.parameters.temperature = Some(temperature);
        self
    }
    
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }
    
    pub fn build(self) -> Request {
        Request {
            messages: self.messages,
            model: self.model.unwrap_or_default(),
            parameters: self.parameters,
            tools: self.tools,
        }
    }
}
```

## Module Design

### Design Principles for Each Module

1. **Single Responsibility**: Each module does one thing well
2. **Minimal Dependencies**: Only depend on what's necessary
3. **Clear Interfaces**: Public API should be obvious and hard to misuse
4. **Testability**: Design for testing from the start
5. **Performance**: Consider performance implications in design

### Module Boundaries

```rust
// Bad: Tight coupling
pub struct OpenAIProvider {
    http_client: reqwest::Client,  // Direct dependency
    logger: Logger,                 // Cross-cutting concern
    cache: Cache,                   // Feature creep
}

// Good: Loose coupling
pub struct OpenAI<C: HttpClient> {
    client: C,                      // Injected dependency
    config: OpenAIConfig,           // Provider-specific only
}
```

### Error Handling Strategy

```rust
// Each module defines its own error type
// cogni-http/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

// Modules convert errors at boundaries
impl From<HttpError> for cogni_core::Error {
    fn from(err: HttpError) -> Self {
        cogni_core::Error::Network(err.to_string())
    }
}
```

## Implementation Details

### Provider Implementation Pattern

```rust
// cogni-providers/src/anthropic.rs

pub struct Anthropic<C: HttpClient> {
    client: C,
    config: AnthropicConfig,
    converter: AnthropicConverter,
    parser: AnthropicParser,
}

struct AnthropicConverter;

#[async_trait]
impl RequestConverter for AnthropicConverter {
    async fn convert_request(&self, request: Request) -> Result<Value, Error> {
        // Convert generic request to Anthropic format
        let mut body = json!({
            "model": request.model.to_string(),
            "max_tokens": request.parameters.max_tokens.unwrap_or(1000),
        });
        
        // Handle messages
        let messages = self.convert_messages(request.messages)?;
        body["messages"] = messages;
        
        // Handle tools if present
        if !request.tools.is_empty() {
            body["tools"] = self.convert_tools(request.tools)?;
        }
        
        Ok(body)
    }
}

struct AnthropicParser;

impl StreamEventParser for AnthropicParser {
    fn parse_event(&self, data: &str) -> Result<Option<StreamEvent>, Error> {
        // Parse SSE format
        if data.starts_with("data: ") {
            let json_str = &data[6..];
            if json_str == "[DONE]" {
                return Ok(Some(StreamEvent::Done));
            }
            
            let value: Value = serde_json::from_str(json_str)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            self.value_to_event(value).map(Some)
        } else {
            Ok(None)
        }
    }
}
```

### Middleware Implementation Pattern

```rust
// cogni-middleware/src/logging.rs

pub struct LoggingMiddleware {
    level: LogLevel,
}

impl Middleware for LoggingMiddleware {
    async fn process_request(&self, request: Request) -> Result<Request, Error> {
        log::debug!("Request: {} messages to {}", 
            request.messages.len(), 
            request.model
        );
        Ok(request)
    }
    
    async fn process_response(&self, response: Response) -> Result<Response, Error> {
        log::debug!("Response: {} chars, {} tool calls", 
            response.content.len(),
            response.tool_calls.len()
        );
        Ok(response)
    }
}

// cogni-middleware/src/retry.rs

pub struct RetryMiddleware {
    max_retries: u32,
    backoff: ExponentialBackoff,
    // Note: Actual retry logic would be implemented at the provider level
    // This middleware just marks the request as retryable
}

impl Middleware for RetryMiddleware {
    async fn process_request(&self, request: Request) -> Result<Request, Error> {
        // Add retry metadata to request
        Ok(request)
    }
    
    async fn process_response(&self, response: Response) -> Result<Response, Error> {
        // Check if response indicates a retryable error
        Ok(response)
    }
}
```

### Streaming Implementation

```rust
// cogni-stream/src/transform.rs

/// Transform content stream to token stream
pub struct TokenTransformer {
    buffer: String,
    tokenizer: Tokenizer,
}

impl StreamTransformer for TokenTransformer {
    type Output = Vec<Token>;
    
    fn transform(&mut self, event: StreamEvent) -> Option<Self::Output> {
        match event {
            StreamEvent::Content(delta) => {
                self.buffer.push_str(&delta.text);
                let tokens = self.tokenizer.tokenize(&self.buffer);
                Some(tokens)
            }
            _ => None,
        }
    }
}

/// Rate limit stream events
pub struct RateLimitTransformer {
    limiter: RateLimiter,
}

impl StreamTransformer for RateLimitTransformer {
    type Output = StreamEvent;
    
    fn transform(&mut self, event: StreamEvent) -> Option<Self::Output> {
        if self.limiter.check() {
            Some(event)
        } else {
            None
        }
    }
}
```

### Tool Execution Patterns

```rust
// cogni-tools/src/patterns.rs

/// Parallel tool executor
pub struct ParallelExecutor {
    executors: Arc<ToolRegistry>,
    max_concurrent: usize,
}

impl ToolExecutor for ParallelExecutor {
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, Error> {
        let executor = self.executors.get(&call.name)
            .ok_or_else(|| Error::ToolExecution("Unknown tool".into()))?;
        
        executor.execute(call).await
    }
}

/// Cached tool executor
pub struct CachedExecutor<E: ToolExecutor> {
    inner: E,
    cache: Cache<String, ToolResult>,
}

impl<E: ToolExecutor> ToolExecutor for CachedExecutor<E> {
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, Error> {
        let key = format!("{}:{}", call.name, call.arguments);
        
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }
        
        let result = self.inner.execute(call).await?;
        self.cache.insert(key, result.clone()).await;
        Ok(result)
    }
}
```

## Performance Architecture

### Zero-Cost Abstractions

```rust
// Use generics instead of trait objects where possible
pub struct Client<P: Provider> {
    provider: P,  // Zero-cost generic
}

// Instead of
pub struct Client {
    provider: Box<dyn Provider>,  // Runtime dispatch cost
}
```

### Memory Efficiency

```rust
// Stream processing without buffering
impl Stream for ProviderStream {
    type Item = Result<StreamEvent, Error>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Process one event at a time
        // No internal buffering unless necessary
    }
}

// Reuse allocations
pub struct MessagePool {
    pool: Vec<Message>,
}

impl MessagePool {
    pub fn acquire(&mut self) -> Message {
        self.pool.pop().unwrap_or_default()
    }
    
    pub fn release(&mut self, msg: Message) {
        self.pool.push(msg);
    }
}
```

### Async Optimization

```rust
// Avoid unnecessary allocations in hot paths
pub async fn process_stream<S: Stream<Item = Result<StreamEvent, Error>>>(
    mut stream: S,
) -> Result<String, Error> {
    let mut content = String::with_capacity(1024);  // Pre-allocate
    
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Content(delta) => {
                content.push_str(&delta.text);
            }
            _ => {}
        }
    }
    
    Ok(content)
}

// Use tokio::spawn for parallel work
pub async fn parallel_requests<P: Provider + 'static>(
    providers: Vec<P>,
    request: Request,
) -> Vec<Result<Response, Error>> {
    let mut handles = Vec::new();
    
    for provider in providers {
        let req = request.clone();
        let handle = tokio::spawn(async move {
            provider.request(req).await
        });
        handles.push(handle);
    }
    
    futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect()
}
```

### Benchmarking Strategy

```rust
// benches/provider_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_request(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let provider = create_mock_provider();
    
    c.bench_function("simple_request", |b| {
        b.iter(|| {
            runtime.block_on(async {
                provider.request(test_request()).await
            })
        })
    });
}

fn benchmark_streaming(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let provider = create_mock_provider();
    
    c.bench_function("stream_processing", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let stream = provider.stream(test_request()).await.unwrap();
                process_stream(stream).await
            })
        })
    });
}

criterion_group!(benches, benchmark_request, benchmark_streaming);
criterion_main!(benches);
```

## Testing Strategy

### Unit Testing Philosophy

```rust
// Each module has comprehensive unit tests
// cogni-core/src/types.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_creation() {
        let msg = Message {
            role: Role::User,
            content: Content::Text("Hello".into()),
            metadata: Metadata::default(),
        };
        
        assert_eq!(msg.role, Role::User);
        assert!(matches!(msg.content, Content::Text(_)));
    }
    
    #[test]
    fn test_request_builder() {
        let request = RequestBuilder::new()
            .message(Role::User, "Hello")
            .model(Model::from("gpt-4"))
            .temperature(0.7)
            .build();
            
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.model.to_string(), "gpt-4");
        assert_eq!(request.parameters.temperature, Some(0.7));
    }
}
```

### Integration Testing

```rust
// tests/integration/provider_test.rs

#[tokio::test]
async fn test_openai_provider() {
    let client = MockHttpClient::new();
    let provider = OpenAI {
        client: Arc::new(client),
        config: OpenAIConfig::default(),
    };
    
    let request = RequestBuilder::new()
        .message(Role::User, "Hello")
        .build();
        
    let response = provider.request(request).await.unwrap();
    assert!(!response.content.is_empty());
}

#[tokio::test]
async fn test_streaming() {
    let client = MockHttpClient::new();
    let provider = OpenAI {
        client: Arc::new(client),
        config: OpenAIConfig::default(),
    };
    
    let request = RequestBuilder::new()
        .message(Role::User, "Tell me a story")
        .build();
        
    let mut stream = provider.stream(request).await.unwrap();
    let mut events = Vec::new();
    
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }
    
    assert!(!events.is_empty());
    assert!(matches!(events.last().unwrap(), StreamEvent::Done));
}
```

### Property-Based Testing

```rust
// tests/property/message_test.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_message_serialization(
        role in prop::sample::select(vec![Role::User, Role::Assistant, Role::System]),
        content in "[a-zA-Z0-9 ]{0,100}",
    ) {
        let msg = Message {
            role,
            content: Content::Text(content),
            metadata: Metadata::default(),
        };
        
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(msg.role, deserialized.role);
        assert_eq!(msg.content, deserialized.content);
    }
}
```

### Mock Implementations

```rust
// cogni-testing/src/mocks.rs
use tokio::sync::Mutex;

pub struct MockProvider {
    responses: Arc<Mutex<Vec<Response>>>,
    delay: Option<Duration>,
}

impl Provider for MockProvider {
    type Stream = MockStream;
    
    async fn request(&self, _request: Request) -> Result<Response, Error> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        
        self.responses
            .lock()
            .await
            .pop()
            .ok_or_else(|| Error::Provider("No mock response".into()))
    }
    
    async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
        Ok(MockStream::new(vec![
            StreamEvent::Content(ContentDelta { text: "Hello".into() }),
            StreamEvent::Content(ContentDelta { text: " world".into() }),
            StreamEvent::Done,
        ]))
    }
}
```

## Development Roadmap

### Phase 1: Foundation (Week 1-2)

**Goal**: Establish core abstractions and basic functionality

1. **Day 1-2**: Set up repository and CI/CD
   - Workspace configuration
   - GitHub Actions for testing and linting
   - Documentation structure

2. **Day 3-5**: Implement cogni-core
   - Core traits and types
   - Error types
   - Basic serialization

3. **Day 6-8**: Implement cogni-http
   - HTTP client abstraction
   - Authentication
   - Basic tests

4. **Day 9-10**: First provider (OpenAI)
   - Basic request/response
   - Streaming support
   - Integration tests

### Phase 2: Core Features (Week 3-4)

**Goal**: Complete provider ecosystem and middleware

1. **Day 11-13**: Additional providers
   - Anthropic implementation
   - Ollama implementation
   - Provider tests

2. **Day 14-16**: Middleware system
   - Core middleware trait
   - Logging and metrics
   - Retry middleware

3. **Day 17-19**: Streaming utilities
   - Stream accumulator
   - Stream transformers
   - Backpressure handling

4. **Day 20**: Integration testing
   - Cross-provider tests
   - Middleware composition tests
   - Performance benchmarks

### Phase 3: Advanced Features (Week 5-6)

**Goal**: Tool execution and high-level client

1. **Day 21-23**: Tool execution
   - Core executor trait
   - Function executor
   - Tool registry

2. **Day 24-26**: High-level client
   - Client implementation
   - Request builder
   - Convenience methods

3. **Day 27-28**: Advanced patterns
   - Parallel execution
   - Caching
   - Rate limiting

4. **Day 29-30**: Documentation
   - API documentation
   - Usage guide
   - Examples

### Phase 4: Polish (Week 7-8)

**Goal**: Production readiness

1. **Week 7**: Performance optimization
   - Profiling and benchmarks
   - Memory optimization
   - Async optimization

2. **Week 8**: Final polish
   - API review and cleanup
   - Documentation completion
   - Release preparation

### Development Practices

1. **Test-Driven Development**: Write tests first
2. **Continuous Integration**: All PRs must pass CI
3. **Code Review**: All code reviewed before merge
4. **Documentation**: Document as you code
5. **Performance**: Benchmark critical paths

### Release Criteria

- [ ] All tests passing (100% of tests)
- [ ] Code coverage >85%
- [ ] Documentation complete
- [ ] Performance benchmarks meet targets
- [ ] API stability guaranteed
- [ ] Security audit passed
- [ ] Examples for all major features

## Additional Considerations

### Missing Features to Consider

1. **Context Management**: Token counting and context window management
   ```rust
   // cogni-context/src/lib.rs
   pub trait TokenCounter: Send + Sync {
       fn count_tokens(&self, text: &str) -> usize;
   }
   
   pub struct ContextManager<C: TokenCounter> {
       counter: C,
       max_tokens: usize,
       messages: Vec<Message>,
   }
   ```

2. **Usage Tracking**: Monitor API usage and costs
   ```rust
   // cogni-usage/src/lib.rs
   pub struct UsageTracker {
       tokens_used: AtomicU64,
       requests_made: AtomicU64,
       estimated_cost: AtomicU64,
   }
   ```

3. **Structured Output**: Type-safe response parsing
   ```rust
   // cogni-core/src/structured.rs
   pub trait StructuredOutput: DeserializeOwned {
       fn schema() -> Value;
   }
   ```

4. **Conversation State**: Persistent conversation management
   ```rust
   // cogni-state/src/lib.rs
   pub trait StateStore: Send + Sync {
       async fn save(&self, id: &str, state: ConversationState) -> Result<(), Error>;
       async fn load(&self, id: &str) -> Result<ConversationState, Error>;
   }
   ```

### Error Handling Improvements

```rust
// cogni-core/src/error.rs
use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    #[error("Provider error ({provider}): {message}")]
    Provider {
        provider: String,
        message: String,
        retry_after: Option<Duration>,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    
    // ... other variants
}
```

### Cancellation and Timeouts

```rust
// cogni-core/src/provider.rs
pub trait Provider: Send + Sync {
    type Stream: Stream<Item = Result<StreamEvent, Error>> + Send + Unpin;
    
    async fn request_with_timeout(
        &self, 
        request: Request,
        timeout: Duration,
    ) -> Result<Response, Error> {
        tokio::time::timeout(timeout, self.request(request))
            .await
            .map_err(|_| Error::Timeout)?
    }
}
```

### Resource Cleanup

```rust
// cogni-stream/src/lib.rs
pub struct ManagedStream<S> {
    stream: S,
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl<S> Drop for ManagedStream<S> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}
```

## Success Metrics

### Technical Excellence
- Clean, idiomatic Rust code
- Minimal dependencies
- Fast compile times (<30s incremental)
- Small binary size
- Excellent performance
- Proper error handling with sources
- Resource cleanup guarantees

### Developer Experience
- Intuitive API
- Comprehensive documentation
- Helpful error messages
- Easy to extend
- Great examples
- Type-safe interfaces
- Good debugging support

### Community
- Active contributors
- Responsive to issues
- Regular releases
- Clear roadmap
- Welcoming environment
- Comprehensive test suite
- Performance benchmarks