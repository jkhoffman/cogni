# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-25

### Added

#### Core Features
- **Multi-provider support**: Unified interface for OpenAI, Anthropic, and Ollama
- **Streaming support**: Efficient handling of streaming responses with `StreamAccumulator`
- **Tool/Function calling**: Full support for tool execution with validation
- **High-level Client API**: Simple interface for common operations
- **Request builder**: Fluent API for constructing complex requests
- **Message types**: Support for text, image, audio, and multi-modal content
- **Structured output**: Type-safe response handling

#### Middleware System
- **Tower-inspired architecture**: Composable middleware using Service/Layer pattern
- **Logging middleware**: Request/response logging with configurable levels
- **Retry middleware**: Automatic retry with exponential backoff
- **Rate limiting**: Token bucket algorithm for API rate limiting
- **Caching**: LRU cache with TTL support (optimized with IndexMap)

#### Provider Implementations
- **OpenAI provider**: Full support for GPT models including tools and streaming
- **Anthropic provider**: Claude support with streaming and tool execution
- **Ollama provider**: Local model support with streaming

#### Advanced Features
- **Parallel execution**: Execute requests across multiple providers
- **Execution strategies**: FirstSuccess, All, Consensus, Race
- **Tool registry**: Manage and execute tools dynamically
- **JSON Schema validation**: Validate tool arguments
- **Usage tracking**: Track token usage and costs
- **Error handling**: Comprehensive error types with retry hints

#### Developer Experience
- **Comprehensive documentation**: API docs and usage guide
- **Examples**: 20+ examples covering various use cases
- **Benchmarks**: Performance benchmarks for optimization
- **Type safety**: Strong typing throughout the API
- **Builder patterns**: Consistent builder APIs

### Optimizations
- Replaced `HashMap` + `VecDeque` with `IndexMap` for LRU cache
- Optimized string allocations in hot paths
- Pre-allocated collections where sizes are known
- Reduced unnecessary clones and allocations
- Efficient stream processing without extra buffers

### API Improvements
- Made error types non-exhaustive with `#[non_exhaustive]`
- Renamed type aliases for clarity (AsyncToolFn -> AsyncToolFunction)
- Made internal modules private
- Consistent middleware type exports
- Hidden implementation details with `#[doc(hidden)]`

### Testing
- Integration tests for all providers
- Tool execution tests with real APIs
- Streaming tests with validation
- Middleware composition tests
- Parallel execution tests

### Documentation
- Comprehensive usage guide
- API documentation for all public types
- README with quick start guide
- Examples for common patterns
- Migration notes for breaking changes

### Infrastructure
- Multi-workspace Cargo project structure
- GitHub Actions CI/CD pipeline
- Criterion benchmarks
- Feature flags for optional dependencies

## [Unreleased]

### Planned
- WebAssembly support
- Additional providers (Cohere, AI21, etc.)
- Persistent conversation management
- Advanced prompt templates
- Plugin system for custom providers
- CLI tool for testing
- Async trait migration (when stabilized)

### Known Issues
- Tool streaming for some providers may buffer entire response
- Ollama provider requires local server
- Some provider-specific features not yet exposed
- Rate limiting is per-instance, not global

## Migration Guide

### From Pre-Release Versions

If you were using development versions, note these breaking changes:

1. **Middleware trait removed**: Use Service/Layer pattern instead
   ```rust
   // Old
   impl Middleware for MyMiddleware { ... }

   // New
   impl<S> Service<Request> for MyService<S> { ... }
   impl<S> Layer<S> for MyLayer { ... }
   ```

2. **Error handling**: Errors are now non-exhaustive
   ```rust
   // Add a catch-all pattern
   match error {
       Error::Network { .. } => ...,
       Error::Provider { .. } => ...,
       _ => // handle unknown errors
   }
   ```

3. **Type aliases renamed**:
   - `AsyncToolFn` -> `AsyncToolFunction`
   - `SyncToolFn` -> `SyncToolFunction`

4. **Internal modules hidden**: Import from crate root
   ```rust
   // Old
   use cogni_providers::anthropic::config::AnthropicConfig;

   // New
   use cogni_providers::AnthropicConfig;
   ```

For more details, see the [Usage Guide](USAGE_GUIDE.md).

[0.1.0]: https://github.com/yourusername/cogni/releases/tag/v0.1.0
