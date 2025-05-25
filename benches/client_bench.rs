//! Performance benchmarks for the high-level client API

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cogni_client::{Client, parallel_requests};
use cogni::{Provider, Request, Response, StreamEvent, Error, Message};
use tokio::runtime::Runtime;
use futures::{stream, StreamExt};
use std::pin::Pin;
use async_trait::async_trait;

/// Mock provider for benchmarking
struct MockProvider {
    delay: Option<tokio::time::Duration>,
}

#[async_trait]
impl Provider for MockProvider {
    type Stream = Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, Error>> + Send>>;
    
    async fn request(&self, _request: Request) -> Result<Response, Error> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        Ok(Response {
            content: "Mock response".to_string(),
            tool_calls: vec![],
            metadata: cogni::ResponseMetadata::default(),
        })
    }
    
    async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
        let events = vec![
            Ok(StreamEvent::Content(cogni::ContentDelta {
                text: "Mock stream".to_string(),
            })),
            Ok(StreamEvent::Done),
        ];
        Ok(Box::pin(stream::iter(events)))
    }
}

/// Benchmark client chat method
fn benchmark_client_chat(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let provider = MockProvider { delay: None };
    let client = Client::new(provider);
    
    c.bench_function("client_chat", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = client.chat(black_box("Hello")).await;
            })
        })
    });
}

/// Benchmark request builder
fn benchmark_client_request_builder(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let provider = MockProvider { delay: None };
    let client = Client::new(provider);
    
    c.bench_function("client_request_builder", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = client
                    .request()
                    .system("You are helpful")
                    .user("Hello")
                    .temperature(0.7)
                    .send()
                    .await;
            })
        })
    });
}

/// Benchmark parallel execution
fn benchmark_parallel_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("parallel_requests_3_providers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let providers = vec![
                    MockProvider { delay: Some(tokio::time::Duration::from_micros(10)) },
                    MockProvider { delay: Some(tokio::time::Duration::from_micros(10)) },
                    MockProvider { delay: Some(tokio::time::Duration::from_micros(10)) },
                ];
                
                let request = Request::builder()
                    .message(Message::user("Test"))
                    .build();
                
                let _ = parallel_requests(providers, black_box(request)).await;
            })
        })
    });
}

/// Benchmark streaming with client
fn benchmark_client_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let provider = MockProvider { delay: None };
    let client = Client::new(provider);
    
    c.bench_function("client_streaming", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut stream = client.stream_chat(black_box("Hello")).await.unwrap();
                while let Some(chunk) = stream.next().await {
                    let _ = black_box(chunk);
                }
            })
        })
    });
}

criterion_group!(
    benches,
    benchmark_client_chat,
    benchmark_client_request_builder,
    benchmark_parallel_execution,
    benchmark_client_streaming
);
criterion_main!(benches);