//! Example demonstrating middleware usage with Tower-inspired pattern

use cogni_core::{Provider, Request, Message, Error};
use cogni_providers::OpenAI;
use cogni_middleware::{
    ServiceBuilder, ProviderExt,
    LoggingLayer, LogLevel,
    RetryLayer, RetryConfig,
    RateLimitLayer, CacheLayer
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Get API key
    let api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");
    
    // Create base provider
    let openai = OpenAI::new(&api_key);
    
    // Create retry configuration
    let retry_config = RetryConfig {
        max_attempts: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
    };
    
    // Build a service with multiple middleware layers
    let service = ServiceBuilder::new()
        .layer(CacheLayer::new(100, Duration::from_secs(300))) // 100 items, 5 min TTL
        .layer(RateLimitLayer::new(2.0)) // 2 requests per second
        .layer(RetryLayer::with_config(retry_config))
        .layer(LoggingLayer::with_level(LogLevel::Info))
        .service(openai.into_service());
    
    // Now we can use the service to make requests
    // Note: For this example to work, we need a wrapper that implements Provider
    // This is where the simple wrapper pattern comes in handy
    
    println!("Example showing middleware composition with Service/Layer pattern");
    println!("For a working example, see middleware_simple.rs");
    
    Ok(())
}