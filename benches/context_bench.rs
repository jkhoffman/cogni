//! Performance benchmarks for context management and token counting

use cogni::{Message, Role};
use cogni_context::{
    ContextManager, ImportanceBasedStrategy, PruningStrategy, SlidingWindowStrategy,
    TiktokenCounter, TokenCounter,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Generate test messages of varying lengths
fn generate_messages(count: usize, avg_length: usize) -> Vec<Message> {
    (0..count)
        .map(|i| {
            let content = "word ".repeat(avg_length / 5); // Roughly 5 chars per word
            if i % 3 == 0 {
                Message::system(content)
            } else if i % 2 == 0 {
                Message::user(content)
            } else {
                Message::assistant(content)
            }
        })
        .collect()
}

/// Benchmark token counting for different message sizes
fn benchmark_token_counting(c: &mut Criterion) {
    let counter = TiktokenCounter::for_model("gpt-4").unwrap();

    // Small messages (typical chat)
    c.bench_function("token_count_small_messages", |b| {
        let messages = generate_messages(10, 50);
        b.iter(|| {
            let count = counter.count_messages(black_box(&messages));
            black_box(count);
        })
    });

    // Medium messages (detailed conversation)
    c.bench_function("token_count_medium_messages", |b| {
        let messages = generate_messages(50, 200);
        b.iter(|| {
            let count = counter.count_messages(black_box(&messages));
            black_box(count);
        })
    });

    // Large messages (long context)
    c.bench_function("token_count_large_messages", |b| {
        let messages = generate_messages(100, 500);
        b.iter(|| {
            let count = counter.count_messages(black_box(&messages));
            black_box(count);
        })
    });

    // Single text counting
    c.bench_function("token_count_single_text", |b| {
        let text = "This is a sample text that will be tokenized. ".repeat(100);
        b.iter(|| {
            let count = counter.count_text(black_box(&text));
            black_box(count);
        })
    });
}

/// Benchmark different pruning strategies
fn benchmark_pruning_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let counter: Arc<dyn TokenCounter> = Arc::new(TiktokenCounter::for_model("gpt-4").unwrap());
    let target_tokens = 2000;

    // Sliding window strategy
    c.bench_function("pruning_sliding_window", |b| {
        let messages = generate_messages(100, 200);
        let strategy = SlidingWindowStrategy::new(true, 20);
        let counter_ref = Arc::clone(&counter);
        b.to_async(&rt).iter(|| {
            let strategy = strategy.clone();
            let messages = messages.clone();
            let counter_ref = Arc::clone(&counter_ref);
            async move {
                let pruned = strategy
                    .prune(messages, target_tokens, counter_ref.as_ref())
                    .await
                    .unwrap();
                black_box(pruned);
            }
        })
    });

    // Importance-based strategy
    c.bench_function("pruning_importance_based", |b| {
        let messages = generate_messages(100, 200);
        let strategy = ImportanceBasedStrategy::new(|msg| match msg.role {
            Role::System => 1.0,
            Role::User => 0.8,
            Role::Assistant => 0.6,
            _ => 0.5,
        });
        let counter_ref = Arc::clone(&counter);
        b.to_async(&rt).iter(|| {
            let strategy = strategy.clone();
            let messages = messages.clone();
            let counter_ref = Arc::clone(&counter_ref);
            async move {
                let pruned = strategy
                    .prune(messages, target_tokens, counter_ref.as_ref())
                    .await
                    .unwrap();
                black_box(pruned);
            }
        })
    });
}

/// Benchmark context manager operations
fn benchmark_context_manager(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let counter: Arc<dyn TokenCounter> = Arc::new(TiktokenCounter::for_model("gpt-4").unwrap());
    let strategy = Arc::new(SlidingWindowStrategy::new(true, 50));
    let manager = ContextManager::new(Arc::clone(&counter))
        .with_max_tokens(4096)
        .with_reserve_output_tokens(1000)
        .with_strategy(strategy);

    // Fit messages within limit
    c.bench_function("context_manager_fit_within_limit", |b| {
        let messages = generate_messages(20, 100);
        let manager_ref = &manager;
        b.to_async(&rt).iter(|| {
            let messages = messages.clone();
            async move {
                let fitted = manager_ref.fit_messages(messages).await.unwrap();
                black_box(fitted);
            }
        })
    });

    // Fit messages requiring pruning
    c.bench_function("context_manager_fit_with_pruning", |b| {
        let messages = generate_messages(200, 200);
        let manager_ref = &manager;
        b.to_async(&rt).iter(|| {
            let messages = messages.clone();
            async move {
                let fitted = manager_ref.fit_messages(messages).await.unwrap();
                black_box(fitted);
            }
        })
    });
}

/// Benchmark token counter creation for different models
fn benchmark_counter_creation(c: &mut Criterion) {
    c.bench_function("tiktoken_counter_creation_gpt4", |b| {
        b.iter(|| {
            let counter = TiktokenCounter::for_model("gpt-4").unwrap();
            black_box(counter);
        })
    });

    c.bench_function("tiktoken_counter_creation_claude", |b| {
        b.iter(|| {
            let counter = TiktokenCounter::for_model("claude-3-opus-20240229").unwrap();
            black_box(counter);
        })
    });
}

criterion_group!(
    benches,
    benchmark_token_counting,
    benchmark_pruning_strategies,
    benchmark_context_manager,
    benchmark_counter_creation
);
criterion_main!(benches);
