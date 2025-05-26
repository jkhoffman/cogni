//! Performance benchmarks for state persistence operations

use cogni::Message;
use cogni_state::{
    store::{FileStore, MemoryStore},
    ConversationState, StateStore,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Generate a test conversation state
fn generate_conversation_state(message_count: usize) -> ConversationState {
    let mut state = ConversationState::new();
    for i in 0..message_count {
        if i % 2 == 0 {
            state.add_message(Message::user(format!("User message {}", i)));
        } else {
            state.add_message(Message::assistant(format!("Assistant response {}", i)));
        }
    }
    state
}

/// Benchmark memory store operations
fn benchmark_memory_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = Arc::new(MemoryStore::new());

    // Save operation
    c.bench_function("memory_store_save", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = generate_conversation_state(20);
                store.save(black_box(&state)).await.unwrap();
            })
        })
    });

    // Load operation
    c.bench_function("memory_store_load", |b| {
        // Pre-populate with states
        let states: Vec<_> = (0..100)
            .map(|i| {
                let mut state = generate_conversation_state(20);
                state.metadata.title = Some(format!("Conversation {}", i));
                state
            })
            .collect();

        rt.block_on(async {
            for state in &states {
                store.save(state).await.unwrap();
            }
        });

        let id = states[50].id;

        b.iter(|| {
            rt.block_on(async {
                let state = store.load(black_box(&id)).await.unwrap();
                black_box(state);
            })
        })
    });

    // List operation
    c.bench_function("memory_store_list", |b| {
        b.iter(|| {
            rt.block_on(async {
                let states = store.list().await.unwrap();
                black_box(states);
            })
        })
    });

    // Find by tags
    c.bench_function("memory_store_find_by_tags", |b| {
        // Add some tagged states
        rt.block_on(async {
            for i in 0..50 {
                let mut state = generate_conversation_state(10);
                state.metadata.tags = vec![format!("tag{}", i % 5), format!("category{}", i % 3)];
                store.save(&state).await.unwrap();
            }
        });

        b.iter(|| {
            rt.block_on(async {
                let states = store
                    .find_by_tags(black_box(&["tag2".to_string()]))
                    .await
                    .unwrap();
                black_box(states);
            })
        })
    });
}

/// Benchmark file store operations
fn benchmark_file_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let store = Arc::new(FileStore::new(temp_dir.path()).unwrap());

    // Save operation
    c.bench_function("file_store_save", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = generate_conversation_state(20);
                store.save(black_box(&state)).await.unwrap();
            })
        })
    });

    // Load operation
    c.bench_function("file_store_load", |b| {
        // Pre-populate with states
        let states: Vec<_> = (0..20)
            .map(|i| {
                let mut state = generate_conversation_state(20);
                state.metadata.title = Some(format!("Conversation {}", i));
                state
            })
            .collect();

        rt.block_on(async {
            for state in &states {
                store.save(state).await.unwrap();
            }
        });

        let id = states[10].id;

        b.iter(|| {
            rt.block_on(async {
                let state = store.load(black_box(&id)).await.unwrap();
                black_box(state);
            })
        })
    });

    // List operation with many files
    c.bench_function("file_store_list", |b| {
        b.iter(|| {
            rt.block_on(async {
                let states = store.list().await.unwrap();
                black_box(states);
            })
        })
    });
}

/// Benchmark state serialization/deserialization
fn benchmark_state_serialization(c: &mut Criterion) {
    // Small state
    c.bench_function("state_serialize_small", |b| {
        let state = generate_conversation_state(5);
        b.iter(|| {
            let json = serde_json::to_string(black_box(&state)).unwrap();
            black_box(json);
        })
    });

    // Large state
    c.bench_function("state_serialize_large", |b| {
        let state = generate_conversation_state(100);
        b.iter(|| {
            let json = serde_json::to_string(black_box(&state)).unwrap();
            black_box(json);
        })
    });

    // Deserialization
    c.bench_function("state_deserialize", |b| {
        let state = generate_conversation_state(20);
        let json = serde_json::to_string(&state).unwrap();
        b.iter(|| {
            let state: ConversationState = serde_json::from_str(black_box(&json)).unwrap();
            black_box(state);
        })
    });
}

/// Benchmark conversation state operations
fn benchmark_state_operations(c: &mut Criterion) {
    // State creation
    c.bench_function("state_creation", |b| {
        b.iter(|| {
            let state = generate_conversation_state(black_box(50));
            black_box(state);
        })
    });

    // Add message
    c.bench_function("state_add_message", |b| {
        let mut state = generate_conversation_state(50);
        b.iter(|| {
            state.add_message(black_box(Message::user("New message")));
        })
    });

    // Metadata updates
    c.bench_function("state_update_metadata", |b| {
        let mut state = generate_conversation_state(20);
        b.iter(|| {
            state.metadata.title = Some("Updated title".to_string());
            state.metadata.tags.push("new-tag".to_string());
            state.metadata.token_count = Some(1500);
            // updated_at is automatically updated when adding messages
        })
    });
}

criterion_group!(
    benches,
    benchmark_memory_store,
    benchmark_file_store,
    benchmark_state_serialization,
    benchmark_state_operations
);
criterion_main!(benches);
