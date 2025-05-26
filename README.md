# Cogni

A unified, high-performance Rust library for Large Language Model (LLM) interactions, providing a clean and type-safe interface for OpenAI, Anthropic, Ollama, and more.

## Features

- 🚀 **Unified API**: Single interface for multiple LLM providers
- 🔒 **Type Safety**: Leverage Rust's type system for compile-time guarantees
- ⚡ **Async First**: Built on Tokio for efficient async operations
- 🌊 **Streaming**: First-class support for streaming responses
- 🛠️ **Tool Calling**: Support for function/tool calling across providers
- 🔧 **Middleware**: Composable middleware for logging, retry, rate limiting, and caching
- 🎯 **High-Level Client**: Simple, intuitive API for common use cases
- 📦 **Modular**: Use only the components you need

## Quick Start

```toml
[dependencies]
cogni = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

### Simple Example

```rust
use cogni::prelude::*;
use cogni::providers::OpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Using the high-level client API
    let client = Client::new(OpenAI::with_api_key("your-api-key"))
        .with_model("gpt-4");

    let response = client.chat("Hello! How are you?").await?;
    println!("{}", response);

    Ok(())
}
```

### Streaming Example

```rust
use cogni::prelude::*;
use cogni::providers::Anthropic;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Anthropic::with_api_key("your-api-key"));

    let mut stream = client.stream_chat("Tell me a story").await?;

    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
    }

    Ok(())
}
```

## Architecture

Cogni is built with a modular architecture:

- **cogni-core**: Core traits and types (zero dependencies)
- **cogni-providers**: Provider implementations (OpenAI, Anthropic, Ollama)
- **cogni-middleware**: Middleware system for cross-cutting concerns
- **cogni-tools**: Tool/function execution framework
- **cogni-client**: High-level client API

## Advanced Features

### Tool Calling

```rust
use cogni::prelude::*;
use cogni::tools::{Tool, ToolRegistry, FunctionExecutor};

// Define a tool
let weather_tool = Tool {
    name: "get_weather".to_string(),
    description: "Get current weather".to_string(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        }
    }),
};

// Use with a request
let response = client
    .request()
    .user("What's the weather in Paris?")
    .tool(weather_tool)
    .send()
    .await?;
```

### Parallel Execution

```rust
use cogni::client::{ParallelClient, ExecutionStrategy};

let providers = vec![
    OpenAI::with_api_key("key1"),
    OpenAI::with_api_key("key2"),
];

let parallel_client = ParallelClient::new(providers)
    .with_strategy(ExecutionStrategy::Race);

// Returns the fastest response
let response = parallel_client.request(request).await?;
```

### Middleware

```rust
use cogni::middleware::{ProviderService, LoggingLayer, Layer};

let provider = OpenAI::with_api_key("your-api-key");
let service = LoggingLayer::new()
    .layer(ProviderService::new(provider));
```

## Performance

Cogni is designed for high performance:

- Zero-cost abstractions where possible
- Efficient streaming with backpressure support
- Minimal allocations in hot paths
- Concurrent request handling

Run benchmarks with:
```bash
cargo bench
```

## Status

This is a complete ground-up rewrite. Current implementation status:

### ✅ Implemented

- Core abstractions and types
- Provider implementations (OpenAI, Anthropic, Ollama)
- Streaming support for all providers
- Tool/function calling framework
- Middleware system (logging, retry, rate limiting, caching)
- High-level client API
- Parallel execution utilities
- Performance benchmarks

### 🚧 In Progress

- Comprehensive documentation
- Additional examples and tutorials

## Examples

Explore the `examples/` directory for comprehensive examples:

```bash
# Basic usage
cargo run --example basic_openai
cargo run --example streaming_openai

# High-level client
cargo run --example client/simple_chat_example
cargo run --example client/streaming_example
cargo run --example client/request_builder_example

# Advanced patterns
cargo run --example client/advanced_patterns_example
cargo run --example client/multi_provider_client_example

# Tool usage
cargo run --example tools/basic_tool_example
cargo run --example tools/advanced_tool_execution_example

# Middleware
cargo run --example middleware/middleware_simple_example
cargo run --example middleware/retry_comprehensive_example
```

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under MIT OR Apache-2.0.
