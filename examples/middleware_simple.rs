//! Simple middleware example showing the pattern

use cogni_core::{Error, Message, Provider, Request, Response};
use cogni_providers::OpenAI;
use std::env;
use std::future::Future;
use std::time::Instant;

/// A simple timing middleware that measures request duration
struct TimingProvider<P: Provider> {
    inner: P,
}

impl<P: Provider> TimingProvider<P> {
    fn new(provider: P) -> Self {
        Self { inner: provider }
    }
}

impl<P: Provider> Provider for TimingProvider<P> {
    type Stream = P::Stream;

    fn request(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send {
        async move {
            println!("⏱️  Starting request to {}", request.model.0);
            let start = Instant::now();

            let result = self.inner.request(request).await;

            let duration = start.elapsed();
            println!("⏱️  Request completed in {:?}", duration);

            result
        }
    }

    fn stream(&self, request: Request) -> impl Future<Output = Result<Self::Stream, Error>> + Send {
        async move {
            println!("⏱️  Starting stream to {}", request.model.0);
            let start = Instant::now();

            let result = self.inner.stream(request).await;

            let duration = start.elapsed();
            println!("⏱️  Stream initiated in {:?}", duration);

            result
        }
    }
}

/// A simple logging middleware
struct LoggingProvider<P: Provider> {
    inner: P,
}

impl<P: Provider> LoggingProvider<P> {
    fn new(provider: P) -> Self {
        Self { inner: provider }
    }
}

impl<P: Provider> Provider for LoggingProvider<P> {
    type Stream = P::Stream;

    fn request(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send {
        async move {
            println!(
                "📝 Request: {} messages to {}",
                request.messages.len(),
                request.model.0
            );

            match self.inner.request(request).await {
                Ok(response) => {
                    println!("📝 Response: {} chars", response.content.len());
                    if let Some(usage) = &response.metadata.usage {
                        println!(
                            "📝 Tokens: {} prompt + {} completion = {} total",
                            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                        );
                    }
                    Ok(response)
                }
                Err(e) => {
                    println!("📝 Error: {}", e);
                    Err(e)
                }
            }
        }
    }

    fn stream(&self, request: Request) -> impl Future<Output = Result<Self::Stream, Error>> + Send {
        self.inner.stream(request)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get API key
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    // Create base provider
    let openai = OpenAI::new(&api_key);

    // Wrap with middleware (applied in reverse order)
    let provider = TimingProvider::new(LoggingProvider::new(openai));

    // Create a request
    let request = Request::builder()
        .message(Message::system("You are a helpful assistant."))
        .message(Message::user("What is 2 + 2?"))
        .max_tokens(50)
        .build();

    println!("Making request with middleware:\n");

    let response = provider.request(request).await?;
    println!("\nFinal response: {}", response.content);

    Ok(())
}
