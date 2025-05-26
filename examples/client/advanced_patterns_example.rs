//! Advanced patterns using the client API

use cogni_client::{create_parallel_client, Client, ExecutionStrategy, ParallelClient};
use cogni_providers::openai::OpenAI;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // Example 1: Parallel execution across multiple instances
    println!("=== Parallel Execution ===");
    demonstrate_parallel_execution(&api_key).await?;

    // Example 2: Using middleware with the client
    println!("\n=== Middleware Example ===");
    demonstrate_middleware(&api_key).await?;

    // Example 3: Advanced parallel strategies
    println!("\n=== Parallel Strategies ===");
    demonstrate_parallel_strategies(&api_key).await?;

    Ok(())
}

/// Demonstrate parallel execution with multiple providers
async fn demonstrate_parallel_execution(api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    use cogni_client::parallel_chat;

    // Create multiple provider instances (could be different providers)
    let providers = vec![
        OpenAI::with_api_key(api_key.to_string()),
        OpenAI::with_api_key(api_key.to_string()),
        OpenAI::with_api_key(api_key.to_string()),
    ];

    // Execute the same query across all providers in parallel
    let results = parallel_chat(providers, "What is 2+2? Answer in one word.").await;

    println!("Parallel results:");
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(response) => println!("  Provider {}: {}", i + 1, response.trim()),
            Err(e) => println!("  Provider {} error: {}", i + 1, e),
        }
    }

    Ok(())
}

/// Demonstrate using middleware with the client
async fn demonstrate_middleware(api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    use cogni_client::MiddlewareProvider;
    use cogni_middleware::{ProviderExt, ProviderService};

    // Create a provider with middleware
    let provider = OpenAI::with_api_key(api_key.to_string());

    // Convert to service to use with middleware
    let service = ProviderService::new(provider);

    // Wrap with middleware provider
    let middleware_provider = MiddlewareProvider::new(service);

    // Create client with the middleware-wrapped provider
    let client = Client::new(middleware_provider).with_model("gpt-4o-mini");

    let response = client.chat("Hello! What's 10 + 15?").await?;
    println!("Response with middleware: {}", response);

    // Note: To add actual middleware like logging, retry, etc., you would need
    // to ensure the middleware services implement Clone. This is a current
    // limitation that would need to be addressed in the middleware crate.

    Ok(())
}

/// Demonstrate different parallel execution strategies
async fn demonstrate_parallel_strategies(api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    use cogni_core::Message;

    // Create multiple providers
    let providers = vec![
        OpenAI::with_api_key(api_key.to_string()).with_model("gpt-4o-mini"),
        OpenAI::with_api_key(api_key.to_string()).with_model("gpt-4o-mini"),
    ];

    // Example 1: Race strategy - return the fastest response
    println!("Race strategy (fastest wins):");
    let client = create_parallel_client(providers.clone()).with_strategy(ExecutionStrategy::Race);

    let request = cogni_core::Request::builder()
        .message(Message::user(
            "What is the capital of France? Answer in one word.",
        ))
        .build();

    let start = std::time::Instant::now();
    let response = client.request(request.clone()).await?;
    println!(
        "  Response: {} (took {:?})",
        response.content.trim(),
        start.elapsed()
    );

    // Example 2: FirstSuccess strategy - return first successful response
    println!("\nFirstSuccess strategy:");
    let client =
        create_parallel_client(providers.clone()).with_strategy(ExecutionStrategy::FirstSuccess);

    let response = client.request(request.clone()).await?;
    println!("  Response: {}", response.content.trim());

    // Example 3: All strategy - wait for all responses
    println!("\nAll strategy (waits for all):");
    let client = create_parallel_client(providers).with_strategy(ExecutionStrategy::All);

    let start = std::time::Instant::now();
    let response = client.request(request).await?;
    println!(
        "  Response: {} (took {:?})",
        response.content.trim(),
        start.elapsed()
    );

    Ok(())
}
