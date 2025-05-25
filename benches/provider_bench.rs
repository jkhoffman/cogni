//! Performance benchmarks for provider implementations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cogni::{Provider, Request, Message, StreamAccumulator};
use tokio::runtime::Runtime;
use futures::StreamExt;
use async_trait::async_trait;

/// Create a test request
fn create_test_request(prompt: &str) -> Request {
    Request::builder()
        .message(Message::user(prompt))
        .max_tokens(50)
        .build()
}

/// Benchmark simple request/response
fn benchmark_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Mock provider for consistent benchmarking
    struct MockProvider;
    
    #[async_trait]
    impl Provider for MockProvider {
        type Stream = futures::stream::Iter<std::vec::IntoIter<Result<cogni::StreamEvent, cogni::Error>>>;
        
        async fn request(&self, _request: Request) -> Result<cogni::Response, cogni::Error> {
            // Simulate some work
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
            Ok(cogni::Response {
                content: "Mock response".to_string(),
                tool_calls: vec![],
                metadata: cogni::ResponseMetadata::default(),
            })
        }
        
        async fn stream(&self, _request: Request) -> Result<Self::Stream, cogni::Error> {
            let events = vec![
                Ok(cogni::StreamEvent::Content(cogni::ContentDelta {
                    text: "Mock ".to_string(),
                })),
                Ok(cogni::StreamEvent::Content(cogni::ContentDelta {
                    text: "stream".to_string(),
                })),
                Ok(cogni::StreamEvent::Done),
            ];
            Ok(futures::stream::iter(events))
        }
    }
    
    let provider = MockProvider;
    
    c.bench_function("provider_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                let request = create_test_request("Hello");
                let _ = provider.request(black_box(request)).await;
            })
        })
    });
}

/// Benchmark streaming response
fn benchmark_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    struct MockProvider;
    
    #[async_trait]
    impl Provider for MockProvider {
        type Stream = futures::stream::Iter<std::vec::IntoIter<Result<cogni::StreamEvent, cogni::Error>>>;
        
        async fn request(&self, _request: Request) -> Result<cogni::Response, cogni::Error> {
            Ok(cogni::Response {
                content: "Mock response".to_string(),
                tool_calls: vec![],
                metadata: cogni::ResponseMetadata::default(),
            })
        }
        
        async fn stream(&self, _request: Request) -> Result<Self::Stream, cogni::Error> {
            let events = (0..100).map(|i| {
                Ok(cogni::StreamEvent::Content(cogni::ContentDelta {
                    text: format!("chunk{} ", i),
                }))
            }).chain(std::iter::once(Ok(cogni::StreamEvent::Done)))
            .collect::<Vec<_>>();
            Ok(futures::stream::iter(events))
        }
    }
    
    let provider = MockProvider;
    
    c.bench_function("stream_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let request = create_test_request("Hello");
                let mut stream = provider.stream(black_box(request)).await.unwrap();
                let mut accumulator = StreamAccumulator::new();
                
                while let Some(event) = stream.next().await {
                    if let Ok(event) = event {
                        accumulator.process_event(event).unwrap();
                    }
                }
                
                // Get final results
                let _ = accumulator.content();
                let _ = accumulator.tool_calls();
            })
        })
    });
}

/// Benchmark request building
fn benchmark_request_building(c: &mut Criterion) {
    c.bench_function("request_builder", |b| {
        b.iter(|| {
            let request = Request::builder()
                .message(Message::system("You are a helpful assistant"))
                .message(Message::user("Hello"))
                .message(Message::assistant("Hi! How can I help you?"))
                .message(Message::user("What's the weather?"))
                .temperature(0.7)
                .max_tokens(150)
                .build();
            black_box(request);
        })
    });
}

/// Benchmark message creation
fn benchmark_message_creation(c: &mut Criterion) {
    c.bench_function("message_creation", |b| {
        b.iter(|| {
            let messages = vec![
                Message::system("You are helpful"),
                Message::user("Hello"),
                Message::assistant("Hi there!"),
            ];
            black_box(messages);
        })
    });
}

criterion_group!(
    benches,
    benchmark_request,
    benchmark_streaming,
    benchmark_request_building,
    benchmark_message_creation
);
criterion_main!(benches);