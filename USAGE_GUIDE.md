# Cogni Usage Guide

Welcome to the Cogni usage guide! This document provides comprehensive examples and best practices for using the Cogni LLM library.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Usage](#basic-usage)
3. [Providers](#providers)
4. [Advanced Features](#advanced-features)
5. [Tool Execution](#tool-execution)
6. [Middleware](#middleware)
7. [Parallel Execution](#parallel-execution)
8. [Streaming](#streaming)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

## Getting Started

Add Cogni to your `Cargo.toml`:

```toml
[dependencies]
cogni = { version = "0.1", features = ["all-providers", "tools"] }
tokio = { version = "1", features = ["full"] }
```

## Basic Usage

### Simple Chat

The simplest way to use Cogni is through the high-level client API:

```rust
use cogni_client::Client;
use cogni_providers::OpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a provider
    let provider = OpenAI::with_api_key("your-api-key".to_string());
    
    // Create a client
    let client = Client::new(provider);
    
    // Send a message
    let response = client.chat("Hello, how are you?").await?;
    println!("{}", response);
    
    Ok(())
}
```

### Using Multiple Providers

```rust
use cogni_client::Client;
use cogni_providers::{OpenAI, Anthropic, Ollama};

// OpenAI
let openai_client = Client::new(OpenAI::with_api_key("key".to_string()));

// Anthropic
let anthropic_client = Client::new(Anthropic::with_api_key("key".to_string()));

// Ollama (local)
let ollama_client = Client::new(Ollama::default());
```

### Request Builder

For more control over your requests:

```rust
use cogni_client::Client;
use cogni_providers::OpenAI;

let client = Client::new(OpenAI::with_api_key("key".to_string()));

let response = client
    .request()
    .system("You are a helpful assistant")
    .user("What's the weather like?")
    .model("gpt-4")
    .temperature(0.7)
    .max_tokens(150)
    .send()
    .await?;
```

## Providers

### OpenAI

```rust
use cogni_providers::{OpenAI, OpenAIConfig};

// Simple initialization
let provider = OpenAI::with_api_key("key".to_string());

// With custom configuration
let config = OpenAIConfig {
    api_key: "key".to_string(),
    base_url: "https://api.openai.com".to_string(),
    default_model: "gpt-4".to_string(),
    ..Default::default()
};
let provider = OpenAI::new(config);
```

### Anthropic

```rust
use cogni_providers::{Anthropic, AnthropicConfig};

// Simple initialization
let provider = Anthropic::with_api_key("key".to_string());

// With custom configuration
let config = AnthropicConfig {
    api_key: "key".to_string(),
    base_url: "https://api.anthropic.com".to_string(),
    default_model: "claude-3-opus-20240229".to_string(),
    ..Default::default()
};
let provider = Anthropic::new(config);
```

### Ollama

```rust
use cogni_providers::{Ollama, OllamaConfig};

// Default configuration (localhost:11434)
let provider = Ollama::default();

// Custom configuration
let config = OllamaConfig {
    base_url: "http://localhost:11434".to_string(),
    default_model: "llama2".to_string(),
    ..Default::default()
};
let provider = Ollama::new(config);
```

## Advanced Features

### Multi-turn Conversations

```rust
use cogni::{Request, Message};
use cogni_providers::OpenAI;

let provider = OpenAI::with_api_key("key".to_string());

let request = Request::builder()
    .message(Message::system("You are a helpful assistant"))
    .message(Message::user("What is Rust?"))
    .message(Message::assistant("Rust is a systems programming language..."))
    .message(Message::user("What makes it special?"))
    .build();

let response = provider.request(request).await?;
```

### Custom Parameters

```rust
use cogni::{Request, Message, Parameters};

let params = Parameters::builder()
    .temperature(0.8)
    .max_tokens(200)
    .top_p(0.9)
    .presence_penalty(0.1)
    .frequency_penalty(0.1)
    .build();

let request = Request::builder()
    .message(Message::user("Write a haiku"))
    .parameters(params)
    .build();
```

## Tool Execution

### Defining Tools

```rust
use cogni_tools::{Tool, ToolBuilder, FunctionExecutor};
use serde_json::{json, Value};

// Define a calculator tool
let calculator = ToolBuilder::new("calculator", "Perform basic arithmetic")
    .parameter("operation", "string", "The operation to perform", true)
    .parameter("a", "number", "First operand", true)
    .parameter("b", "number", "Second operand", true)
    .build();

// Create an executor
let executor = FunctionExecutor::new_sync(calculator, |args: Value| {
    let op = args["operation"].as_str().unwrap();
    let a = args["a"].as_f64().unwrap();
    let b = args["b"].as_f64().unwrap();
    
    let result = match op {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => a / b,
        _ => return Err("Unknown operation".into()),
    };
    
    Ok(json!({ "result": result }))
});
```

### Using Tools with Requests

```rust
use cogni_tools::ToolRegistry;

// Create a registry
let registry = ToolRegistry::new();
registry.register(executor).await?;

// Get tools for request
let tools = registry.list_tools().await;

// Add tools to request
let request = Request::builder()
    .message(Message::user("What is 25 * 4?"))
    .tool(tools[0].clone())
    .build();

// Execute with tool support
let response = provider.request(request).await?;

// Execute tool calls if any
if !response.tool_calls.is_empty() {
    let results = registry.execute_many(&response.tool_calls).await;
    // Handle tool results...
}
```

## Middleware

### Logging Middleware

```rust
use cogni_middleware::{LoggingLayer, LogLevel};
use cogni_client::{Client, MiddlewareProvider};

let logging = LoggingLayer::with_level(LogLevel::Debug)
    .with_content(); // Log request/response content

let provider = OpenAI::with_api_key("key".to_string());
let middleware_provider = MiddlewareProvider::builder()
    .layer(logging)
    .service(provider)
    .build();

let client = Client::new(middleware_provider);
```

### Retry Middleware

```rust
use cogni_middleware::{RetryLayer, RetryConfig};
use std::time::Duration;

let retry = RetryLayer::new(RetryConfig {
    max_attempts: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(10),
    exponential_base: 2.0,
});

let middleware_provider = MiddlewareProvider::builder()
    .layer(retry)
    .service(provider)
    .build();
```

### Rate Limiting

```rust
use cogni_middleware::RateLimitLayer;

// 10 requests per second
let rate_limit = RateLimitLayer::new(10, Duration::from_secs(1));

let middleware_provider = MiddlewareProvider::builder()
    .layer(rate_limit)
    .service(provider)
    .build();
```

### Caching

```rust
use cogni_middleware::CacheLayer;

// Cache up to 100 responses for 1 hour
let cache = CacheLayer::new(100, Duration::from_secs(3600));

let middleware_provider = MiddlewareProvider::builder()
    .layer(cache)
    .service(provider)
    .build();
```

### Combining Middleware

```rust
let middleware_provider = MiddlewareProvider::builder()
    .layer(logging)
    .layer(retry)
    .layer(rate_limit)
    .layer(cache)
    .service(provider)
    .build();
```

## Parallel Execution

### Parallel Requests

Execute the same request across multiple providers:

```rust
use cogni_client::parallel::parallel_requests;

let providers = vec![
    OpenAI::with_api_key("key1".to_string()),
    Anthropic::with_api_key("key2".to_string()),
];

let request = Request::builder()
    .message(Message::user("Explain quantum computing"))
    .build();

let results = parallel_requests(providers, request).await;

for (i, result) in results.iter().enumerate() {
    match result {
        Ok(response) => println!("Provider {}: {}", i, response.content),
        Err(e) => println!("Provider {} failed: {}", i, e),
    }
}
```

### Parallel Client

Use different execution strategies:

```rust
use cogni_client::parallel::{ParallelClient, ExecutionStrategy};

let providers = vec![provider1, provider2, provider3];

// Get first successful response
let client = ParallelClient::new(providers.clone())
    .with_strategy(ExecutionStrategy::FirstSuccess);

let response = client.chat("Hello").await?;

// Get all responses
let client = ParallelClient::new(providers.clone())
    .with_strategy(ExecutionStrategy::All);

let responses = client.chat_all("Hello").await;

// Race for fastest response
let client = ParallelClient::new(providers)
    .with_strategy(ExecutionStrategy::Race);

let response = client.chat("Hello").await?;
```

## Streaming

### Basic Streaming

```rust
use futures::StreamExt;

let mut stream = client.stream_chat("Tell me a story").await?;

while let Some(chunk) = stream.next().await {
    match chunk? {
        StreamEvent::Content(delta) => print!("{}", delta.text),
        StreamEvent::Done => println!("\n[Done]"),
        _ => {}
    }
}
```

### Stream Processing

```rust
use cogni::StreamAccumulator;

let mut accumulator = StreamAccumulator::new();
let mut stream = provider.stream(request).await?;

while let Some(event) = stream.next().await {
    accumulator.process_event(event?)?;
    
    // Get accumulated content so far
    println!("Current: {}", accumulator.content());
}

// Get final results
let content = accumulator.content();
let tool_calls = accumulator.tool_calls();
```

### Streaming with Tools

```rust
let request = Request::builder()
    .message(Message::user("Calculate 15 * 7 and explain"))
    .tool(calculator_tool)
    .build();

let mut stream = provider.stream(request).await?;
let mut accumulator = StreamAccumulator::new();

while let Some(event) = stream.next().await {
    let event = event?;
    
    match &event {
        StreamEvent::ToolCall(delta) => {
            println!("Tool call: {:?}", delta);
        }
        StreamEvent::Content(delta) => {
            print!("{}", delta.text);
        }
        _ => {}
    }
    
    accumulator.process_event(event)?;
}
```

## Error Handling

### Error Types

```rust
use cogni::Error;

match provider.request(request).await {
    Ok(response) => println!("{}", response.content),
    Err(e) => match e {
        Error::Network { message, .. } => {
            eprintln!("Network error: {}", message);
        }
        Error::Provider { provider, message, retry_after, .. } => {
            eprintln!("Provider {} error: {}", provider, message);
            if let Some(duration) = retry_after {
                eprintln!("Retry after: {:?}", duration);
            }
        }
        Error::Validation(msg) => {
            eprintln!("Validation error: {}", msg);
        }
        Error::Timeout => {
            eprintln!("Request timed out");
        }
        _ => eprintln!("Other error: {}", e),
    }
}
```

### With Middleware

The retry middleware automatically handles transient errors:

```rust
let retry = RetryLayer::new(RetryConfig {
    max_attempts: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(10),
    exponential_base: 2.0,
});

// Will retry on network errors, timeouts, and rate limits
let middleware_provider = MiddlewareProvider::builder()
    .layer(retry)
    .service(provider)
    .build();
```

## Best Practices

### 1. Use Environment Variables for API Keys

```rust
let api_key = std::env::var("OPENAI_API_KEY")
    .expect("OPENAI_API_KEY not set");
let provider = OpenAI::with_api_key(api_key);
```

### 2. Configure Timeouts

```rust
use std::time::Duration;
use reqwest::ClientBuilder;

// Configure HTTP client with timeout
let http_client = ClientBuilder::new()
    .timeout(Duration::from_secs(30))
    .build()?;

// Use with provider configuration
let config = OpenAIConfig {
    api_key: api_key,
    client: Some(http_client),
    ..Default::default()
};
```

### 3. Handle Rate Limits

Always use rate limiting middleware in production:

```rust
let rate_limit = RateLimitLayer::new(
    10, // requests
    Duration::from_secs(1) // per second
);
```

### 4. Cache Expensive Requests

```rust
let cache = CacheLayer::new(
    1000, // max entries
    Duration::from_secs(3600) // 1 hour TTL
);
```

### 5. Log for Debugging

```rust
// Development
let logging = LoggingLayer::with_level(LogLevel::Debug)
    .with_content();

// Production
let logging = LoggingLayer::with_level(LogLevel::Info);
```

### 6. Use Structured Errors

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("LLM error: {0}")]
    Llm(#[from] cogni::Error),
    
    #[error("Business logic error: {0}")]
    Business(String),
}
```

### 7. Stream for Long Responses

For responses that might be long, use streaming to provide better UX:

```rust
let mut stream = client.stream_chat(prompt).await?;
let mut buffer = String::new();

while let Some(chunk) = stream.next().await {
    if let Ok(StreamEvent::Content(delta)) = chunk {
        buffer.push_str(&delta.text);
        // Update UI with partial response
        update_ui(&buffer);
    }
}
```

### 8. Validate Tool Responses

```rust
use cogni_tools::JsonSchemaValidator;

let validator = JsonSchemaValidator::new(tool.parameters.clone());

match registry.execute(&tool_call).await {
    Ok(result) => {
        // Validate the result matches expected schema
        if let Err(e) = validator.validate(&result.result) {
            eprintln!("Tool returned invalid result: {}", e);
        }
    }
    Err(e) => eprintln!("Tool execution failed: {}", e),
}
```

## Examples

For more examples, check out the `examples/` directory in the repository:

- `hello_world.rs` - Simple getting started example
- `multi_provider.rs` - Using multiple providers
- `streaming_chat.rs` - Streaming responses
- `tool_calling.rs` - Using tools/functions
- `rag_example.rs` - Retrieval Augmented Generation
- `web_service.rs` - Building a web service with Cogni

## Troubleshooting

### Common Issues

1. **Connection Refused (Ollama)**
   - Ensure Ollama is running: `ollama serve`
   - Check the URL: default is `http://localhost:11434`

2. **Invalid API Key**
   - Double-check your API key
   - Ensure it's properly set in environment variables

3. **Rate Limits**
   - Use rate limiting middleware
   - Implement exponential backoff
   - Consider caching for repeated requests

4. **Timeout Errors**
   - Increase timeout in HTTP client configuration
   - Use streaming for long-running requests
   - Consider breaking large requests into smaller ones

### Debug Tips

1. Enable debug logging:
```rust
env_logger::Builder::from_env(env_logger::Env::default()
    .default_filter_or("debug"))
    .init();
```

2. Use the logging middleware to see requests/responses

3. Check provider-specific documentation for model capabilities

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.