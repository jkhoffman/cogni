# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Important: Project Status

This project is **unpublished and unreleased**. When making changes:
- **Prefer breaking changes** over maintaining backwards compatibility
- **Avoid code debt** - refactor aggressively when needed
- **Don't preserve legacy patterns** - improve the API freely
- **Clean up technical debt** immediately rather than adding workarounds

The codebase should evolve to its best possible design without compatibility constraints.

## Development Principles

- Always determine the root cause of a linter warning before silencing it.
- Always run `mask check` after making changes to verify your work.

## Common Development Commands

### Using Mask (Preferred)
The project includes a `maskfile.md` that defines common development tasks. Use `mask` for convenient command execution:

```bash
# Run all tests
mask test

# Run clippy
mask clippy

# Run all checks (fmt, clippy, test)
mask check

# Generate coverage report
mask coverage

# See all available commands
mask --help
```

### Direct Cargo Commands
```bash
# Build all packages
cargo build --all-features

# Run all tests (requires API keys)
OPENAI_API_KEY=<key> ANTHROPIC_API_KEY=<key> cargo test --all-features

# Run specific test
cargo test test_name --features tools

# Run tests for a specific crate
cargo test -p cogni-providers

# Check code without building
cargo check --all-features

# Run clippy linting
cargo clippy --all-features -- -D warnings

# Format code
cargo fmt --all

# Run benchmarks
cargo bench --features tools

# Build documentation
cargo doc --all-features --open

# Run examples (from root)
cargo run --example hello_world
cargo run --example streaming_chat
```

### Environment Variables
- `OPENAI_API_KEY` - Required for OpenAI provider tests
- `ANTHROPIC_API_KEY` - Required for Anthropic provider tests
- Ollama tests require local Ollama server running on port 11434

## High-Level Architecture

### Workspace Structure
Cogni is a multi-crate workspace with clear separation of concerns:

- **cogni-core**: Foundational traits and types. Zero dependencies on other cogni crates. Defines `Provider`, `Request`, `Response`, `StreamEvent`, and `Error` types.

- **cogni-providers**: Concrete provider implementations (OpenAI, Anthropic, Ollama). Each provider has its own module with config, converter, parser, and stream handling. Providers implement the core `Provider` trait.

- **cogni-middleware**: Tower-inspired middleware system using Service/Layer pattern. Key middleware: LoggingLayer, RetryLayer, RateLimitLayer, CacheLayer. Middleware can wrap any Service<Request>.

- **cogni-tools**: Tool/function execution framework. Defines `ToolExecutor` trait, `ToolRegistry` for managing tools, and validation utilities. Tools are provider-agnostic.

- **cogni-client**: High-level client API built on top of providers. Includes `Client` for simple operations, `RequestBuilder` for fluent API, and `ParallelClient` for multi-provider execution.

- **cogni**: Root crate that re-exports everything. Users typically only need to depend on this crate.

### Key Design Patterns

1. **Provider Abstraction**: All providers implement a common `Provider` trait with `request()` and `stream()` methods. This allows provider-agnostic code.

2. **Service/Layer Pattern**: Middleware uses Tower-inspired pattern where Services process requests and Layers wrap Services. This enables composable middleware stacks.

3. **Stream Processing**: All providers support streaming via `Stream<Item = Result<StreamEvent, Error>>`. StreamAccumulator helps collect streaming responses.

4. **Tool Execution**: Tools are defined with JSON Schema for validation. ToolRegistry manages executors. Tool calls are part of the Response and can be executed separately.

5. **Error Handling**: Unified Error enum with variants for different failure modes. Errors include retry hints and are non-exhaustive for future compatibility.

### Critical Implementation Details

1. **Async Trait Workaround**: Due to Rust limitations, provider methods return `BoxFuture` and `BoxStream` for type erasure. The `Provider` trait uses `impl Future` syntax where possible.

2. **Middleware Composition**: Middleware must implement Clone to work with the Service pattern. The cache uses IndexMap for efficient LRU operations instead of HashMap+VecDeque.

3. **Request Building**: Two builder patterns exist - `Request::builder()` for direct request construction and `Client::request()` for client-connected building.

4. **Memory Optimizations**: Cache hashing uses byte representations instead of format!(), streaming parsers avoid unnecessary allocations, and collections are pre-allocated where possible.

5. **Tool Streaming**: Tool calls can arrive via streaming events. The StreamAccumulator collects partial tool call information and assembles complete ToolCall objects.

## Phase Structure

The codebase was built in 4 phases:
- Phase 1: Core types and provider implementations
- Phase 2: Middleware system
- Phase 3: High-level client API and advanced patterns
- Phase 4: Polish, optimizations, and documentation

## Testing Strategy

- Unit tests are in each module's source file
- Integration tests in `tests/` directory test cross-crate functionality
- Provider tests require real API keys but use minimal tokens
- Examples serve as both documentation and integration tests
- Benchmarks measure performance of key operations
