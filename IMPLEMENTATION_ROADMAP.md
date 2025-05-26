# Cogni Agentic Features Implementation Roadmap

## Overview

This roadmap provides specific implementation steps for transforming Cogni into an agentic framework while maintaining its clean architecture and backwards compatibility.

## Phase A: Conversation State Persistence (5-6 days) ✅ COMPLETED

### A.1: Create cogni-state crate
```toml
# Cargo.toml addition
[workspace.members]
cogni-state = { path = "cogni-state" }

# cogni-state/Cargo.toml
[dependencies]
cogni-core = { path = "../cogni-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tokio = { version = "1.0", features = ["sync", "fs"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### A.2: Core State Types
```rust
// cogni-state/src/types.rs
use cogni_core::{Message, Metadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub metadata: StateMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateMetadata {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub agent_config: Option<serde_json::Value>,
    pub token_count: Option<u32>,
    pub custom: std::collections::HashMap<String, String>,
}
```

### A.3: StateStore Trait
```rust
// cogni-state/src/store.rs
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save(&self, state: &ConversationState) -> Result<(), StateError>;
    async fn load(&self, id: &Uuid) -> Result<ConversationState, StateError>;
    async fn delete(&self, id: &Uuid) -> Result<(), StateError>;
    async fn list(&self) -> Result<Vec<ConversationState>, StateError>;
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<ConversationState>, StateError>;
}
```

### A.4: Integration with Client
```rust
// cogni-client/src/client.rs additions
impl Client {
    pub fn with_state(mut self, store: Arc<dyn StateStore>) -> StatefulClient {
        StatefulClient::new(self, store)
    }
}

// cogni-client/src/stateful.rs (new file)
pub struct StatefulClient {
    client: Client,
    store: Arc<dyn StateStore>,
    current_state: Option<ConversationState>,
}

impl StatefulClient {
    pub async fn load_conversation(&mut self, id: Uuid) -> Result<(), Error> {
        let state = self.store.load(&id).await?;
        self.current_state = Some(state);
        Ok(())
    }

    pub async fn chat(&mut self, message: &str) -> Result<Response, Error> {
        // Add message to state
        // Send request with full conversation history
        // Update state with response
        // Auto-save state
    }
}
```

### A.5: State Middleware
```rust
// cogni-middleware/src/state.rs
pub struct StateLayer {
    store: Arc<dyn StateStore>,
    auto_save: bool,
    save_interval: Duration,
}

impl<S> Layer<S> for StateLayer {
    type Service = StateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StateService {
            inner,
            store: Arc::clone(&self.store),
            state_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
```

## Phase B: Context Management (4-5 days) ✅ COMPLETED

### B.1: Create cogni-context crate
```toml
# cogni-context/Cargo.toml
[dependencies]
cogni-core = { path = "../cogni-core" }
tiktoken-rs = "0.5"
async-trait = "0.1"
```

### B.2: Token Counter Implementation
```rust
// cogni-context/src/counter.rs
pub trait TokenCounter: Send + Sync {
    fn count_text(&self, text: &str) -> usize;
    fn count_message(&self, message: &Message) -> usize;
    fn count_messages(&self, messages: &[Message]) -> usize;
    fn model_context_window(&self) -> usize;
}

// cogni-context/src/tiktoken.rs
pub struct TiktokenCounter {
    encoder: Arc<CoreBPE>,
    model_limits: HashMap<String, usize>,
}

impl TiktokenCounter {
    pub fn for_model(model: &str) -> Result<Self, ContextError> {
        let encoder = tiktoken_rs::get_bpe_from_model(model)?;
        // Initialize with model limits
    }
}
```

### B.3: Context Manager
```rust
// cogni-context/src/manager.rs
pub struct ContextManager {
    counter: Box<dyn TokenCounter>,
    max_tokens: usize,
    reserve_output_tokens: usize,
    pruning_strategy: Box<dyn PruningStrategy>,
}

impl ContextManager {
    pub fn fit_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, ContextError> {
        let total_tokens = self.counter.count_messages(&messages);
        if total_tokens <= self.available_tokens() {
            return Ok(messages);
        }

        self.pruning_strategy.prune(messages, self.available_tokens(), &*self.counter)
    }
}
```

### B.4: Pruning Strategies
```rust
// cogni-context/src/strategies.rs
pub trait PruningStrategy: Send + Sync {
    fn prune(&self, messages: Vec<Message>, target_tokens: usize, counter: &dyn TokenCounter) -> Result<Vec<Message>, ContextError>;
}

pub struct SlidingWindowStrategy {
    keep_system: bool,
    keep_recent: usize,
}

pub struct ImportanceBasedStrategy {
    importance_scorer: Box<dyn Fn(&Message) -> f32>,
}

pub struct SummarizationStrategy {
    summarizer: Arc<dyn Provider>,
    chunk_size: usize,
}
```

### B.5: Client Integration
```rust
// cogni-client/src/builder.rs additions
impl RequestBuilder {
    pub fn with_context_manager(mut self, manager: ContextManager) -> Self {
        self.context_manager = Some(manager);
        self
    }
}

// Automatically prune messages before sending
```

## Phase C: Structured Output (4-5 days) ✅ COMPLETED

### C.1: Core Types
```rust
// cogni-core/src/types/structured.rs (new file)
pub trait StructuredOutput: Serialize + DeserializeOwned {
    fn schema() -> serde_json::Value;
    fn examples() -> Vec<Self> { vec![] }
}

// Derive macro placeholder
// #[derive(StructuredOutput)]
// pub struct WeatherReport {
//     temperature: f32,
//     conditions: String,
// }
```

### C.2: Request Extension
```rust
// cogni-core/src/types/request.rs additions
pub struct Request {
    // ... existing fields
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone)]
pub enum ResponseFormat {
    JsonSchema {
        schema: serde_json::Value,
        strict: bool,
    },
    JsonObject,
}
```

### C.3: Provider Updates
```rust
// cogni-providers/src/openai/converter.rs
impl RequestConverter for OpenAIConverter {
    fn convert_request(&self, request: Request) -> Result<OpenAIRequest, Error> {
        let mut openai_req = // ... existing conversion

        if let Some(format) = request.response_format {
            openai_req.response_format = Some(match format {
                ResponseFormat::JsonSchema { schema, strict } => {
                    json!({
                        "type": "json_schema",
                        "json_schema": {
                            "name": "response",
                            "strict": strict,
                            "schema": schema
                        }
                    })
                }
                ResponseFormat::JsonObject => json!({ "type": "json_object" }),
            });
        }
    }
}
```

### C.4: Response Validation
```rust
// cogni-core/src/types/response.rs additions
impl Response {
    pub fn parse_structured<T: StructuredOutput>(&self) -> Result<T, Error> {
        let json_str = self.content.as_text()
            .ok_or(Error::InvalidFormat("Expected text content"))?;

        let value: T = serde_json::from_str(json_str)?;

        // Optional: validate against schema
        if let Err(e) = validate_against_schema(&value, &T::schema()) {
            return Err(Error::ValidationError(e));
        }

        Ok(value)
    }
}
```

### C.5: Builder Support
```rust
// cogni-client/src/builder.rs additions
impl RequestBuilder {
    pub fn with_structured_output<T: StructuredOutput>() -> Self {
        self.response_format(ResponseFormat::JsonSchema {
            schema: T::schema(),
            strict: true,
        })
    }
}

// High-level API
impl Client {
    pub async fn chat_structured<T: StructuredOutput>(&self, messages: Vec<Message>) -> Result<T, Error> {
        let response = self.request()
            .messages(messages)
            .with_structured_output::<T>()
            .send()
            .await?;

        response.parse_structured()
    }
}
```

### C.6: Retry Middleware
```rust
// cogni-middleware/src/structured_retry.rs
pub struct StructuredRetryLayer {
    max_retries: usize,
    include_error_context: bool,
}

// On validation failure, retry with error context
```

## Phase D: Integration & Polish (2-3 days) (PARTIALLY COMPLETED)

### D.1: Combined Features ✅ COMPLETED
- Created comprehensive example in `examples/agentic_combined_example.rs`
- Demonstrates all three features working together:
  - Stateful conversation management with FileStore
  - Context window management with TiktokenCounter
  - Structured output generation with JSON schema validation
- Example shows realistic business analysis scenario

### D.2: Testing Strategy (IN PROGRESS)
- ✅ Integration tests for feature combinations (`tests/agentic_features_test.rs`)
  - Mock provider tests for deterministic behavior
  - Real provider integration test with OpenAI
  - Tests for all feature combinations
- ✅ Performance benchmarks for token counting and state operations
  - Created `benches/context_bench.rs` for token counting benchmarks
  - Created `benches/state_bench.rs` for state persistence benchmarks
  - Benchmarks cover various scenarios and message sizes
- ✅ Additional example agents demonstrating real-world usage
  - Created `examples/code_review_agent.rs` - AI code reviewer with structured feedback
  - Created `examples/customer_support_agent.rs` - Support ticket handling with sentiment analysis
  - Created `examples/research_assistant_agent.rs` - Research agent with tool usage and citations
  - Created `examples/data_analysis_agent.rs` - Data analysis with statistical insights

### D.3: Documentation (PENDING)
- ⏳ Architecture guide explaining the agentic features
- ⏳ Migration guide for existing users
- ⏳ Best practices for building agents
- ⏳ Performance tuning guide

## Implementation Order

1. **Week 1**: Phase A (State Persistence) ✅ COMPLETED
   - Days 1-2: Core types and traits ✅
   - Days 3-4: Store implementations ✅
   - Days 5-6: Client integration and middleware ✅

2. **Week 2**: Phase B (Context Management) ✅ COMPLETED
   - Days 1-2: Token counting infrastructure ✅
   - Days 3-4: Context manager and strategies ✅
   - Day 5: Client integration ✅

3. **Week 3**: Phase C (Structured Output) and Phase D
   - Days 1-2: Core types and provider updates
   - Days 3-4: Validation and retry logic
   - Days 5: Integration, examples, and polish

## Breaking Changes

While the goal is backwards compatibility, some changes may be necessary:

1. **Request/Response Types**: Adding optional fields should be non-breaking
2. **Client API**: New methods are additive, existing methods unchanged
3. **Provider Trait**: May need default implementations for new methods
4. **Error Enum**: Add new variants (non-exhaustive enum prevents breaking)

## Success Criteria

- [x] All existing tests pass
- [x] New features have >90% test coverage (Phase A & B)
- [ ] Performance benchmarks show acceptable overhead
- [x] Examples demonstrate practical agent implementations (Phase A & B)
- [ ] Documentation is comprehensive and clear

## Progress Summary

### Completed Features:
1. **Phase A - Conversation State Persistence** ✅
   - cogni-state crate with StateStore trait
   - Memory and file-based storage implementations
   - StatefulClient with auto-save functionality
   - State middleware for automatic tracking
   - Full test coverage and examples

2. **Phase B - Context Management** ✅
   - cogni-context crate with TokenCounter trait
   - TiktokenCounter for accurate token counting
   - ContextManager with multiple pruning strategies
   - Integration with RequestBuilder via with_context_manager()
   - Support for all major model context windows

### Additional Improvements:
- Reduced code duplication with provider utilities
- Improved API consistency with trait implementations
- Added Default and Into conversions where appropriate
- Unified builder patterns across the codebase

3. Phase C: Structured Output (✅ COMPLETED)
   - StructuredOutput trait for type-safe JSON responses
   - ResponseFormat enum supporting JsonSchema and JsonObject
   - Provider support for OpenAI, Anthropic, and Ollama
   - Response parsing with parse_structured() and parse_json()
   - Client convenience method chat_structured()
   - RequestBuilder methods: with_structured_output() and json_mode()
   - Comprehensive examples and tests

### Current Status:
Phase D is in progress with the following completed:
- D.1: Combined features example ✅
- D.2: Integration tests ✅
- D.2: Performance benchmarks (pending)
- D.2: Additional example agents (pending)
- D.3: Documentation (pending)
