# TODO: Agentic Features Implementation Plan

This document outlines the implementation plan for transforming Cogni from an LLM library into a powerful agentic framework.

## Overview

Three core features that will enable sophisticated agent development:
1. **Conversation State Persistence** - Memory across sessions
2. **Context Management** - Intelligent token management
3. **Structured Output** - Type-safe, reliable responses

## Phase A: Conversation State Persistence

### A.1: Core State Types
- [ ] Create new crate `cogni-state`
- [ ] Define `ConversationState` struct with id, messages, metadata, timestamps
- [ ] Add serde derives for serialization
- [ ] Write unit tests for state serialization/deserialization

### A.2: StateStore Trait
- [ ] Define `StateStore` trait with save/load/delete/list methods
- [ ] Add state-specific error types
- [ ] Create `MockStore` for testing
- [ ] Document trait usage patterns

### A.3: Memory State Store
- [ ] Implement `MemoryStore` with Arc<RwLock<HashMap>>
- [ ] Add full CRUD operations
- [ ] Implement TTL/expiration support
- [ ] Write tests for concurrent access

### A.4: File System State Store
- [ ] Implement `FileStore` with JSON persistence
- [ ] Add file locking for concurrent safety
- [ ] Handle directory creation and permissions
- [ ] Test error recovery scenarios

### A.5: State-Aware Client
- [ ] Extend `Client` with `with_state()` method
- [ ] Create `StatefulClient` wrapper
- [ ] Implement auto-save after interactions
- [ ] Add state lifecycle tests

### A.6: State Middleware
- [ ] Create `StateMiddleware` for automatic tracking
- [ ] Intercept and update conversation state
- [ ] Add configuration for save frequency
- [ ] Test with all providers

## Phase B: Context Management

### B.1: Token Counter Trait
- [ ] Create new crate `cogni-context`
- [ ] Define `TokenCounter` trait
- [ ] Add message-level counting methods
- [ ] Create mock implementation

### B.2: Tiktoken Integration
- [ ] Add tiktoken-rs dependency
- [ ] Implement `TiktokenCounter` for GPT models
- [ ] Cache encoders for performance
- [ ] Benchmark token counting speed

### B.3: Context Manager Core
- [ ] Implement `ContextManager` struct
- [ ] Add token budget tracking
- [ ] Implement overflow detection
- [ ] Write unit tests for edge cases

### B.4: Pruning Strategies
- [ ] Define `PruningStrategy` trait
- [ ] Implement `SlidingWindowStrategy`
- [ ] Implement `ImportanceBasedStrategy`
- [ ] Test information preservation

### B.5: Smart Summarization
- [ ] Create `SummarizationStrategy` using LLM
- [ ] Implement message compression
- [ ] Preserve key information
- [ ] Test with long conversations

### B.6: Context-Aware Client
- [ ] Add `with_context_manager()` to Client
- [ ] Implement automatic pruning
- [ ] Add context limit warnings
- [ ] Integration tests with providers

## Phase C: Structured Output

### C.1: StructuredOutput Trait
- [ ] Add trait to cogni-core
- [ ] Define schema generation interface
- [ ] Add optional examples method
- [ ] Plan derive macro implementation

### C.2: JSON Schema Generation
- [ ] Integrate schemars crate
- [ ] Implement automatic schema derivation
- [ ] Handle complex/nested types
- [ ] Test schema validation

### C.3: Provider Integration
- [ ] Extend Request with structured output
- [ ] Add provider-specific format hints
- [ ] Handle response format parameters
- [ ] Test with each provider

### C.4: Response Validation
- [ ] Implement `ValidatedResponse` type
- [ ] Add JSON schema validation
- [ ] Graceful error handling
- [ ] Test malformed responses

### C.5: Retry with Corrections
- [ ] Create `StructuredRetryMiddleware`
- [ ] Auto-retry on validation failure
- [ ] Include errors in retry prompt
- [ ] Test recovery success rate

### C.6: High-Level API
- [ ] Add `chat_structured()` method
- [ ] Support builder pattern
- [ ] Type-safe response handling
- [ ] Create usage examples

## Phase D: Integration & Polish

### D.1: Combined State + Context
- [ ] State tracks token usage
- [ ] Auto-prune on state load
- [ ] Test memory efficiency
- [ ] Benchmark performance

### D.2: Structured State Schemas
- [ ] Define state format schemas
- [ ] Validate on load/save
- [ ] Add migration support
- [ ] Version compatibility

### D.3: Comprehensive Examples
- [ ] Agent with persistent memory
- [ ] Context-aware chatbot
- [ ] Structured tool-calling agent
- [ ] Multi-agent collaboration

### D.4: Performance Optimization
- [ ] Benchmark all features
- [ ] Profile memory usage
- [ ] Optimize hot paths
- [ ] Document performance tips

### D.5: Documentation
- [ ] Architecture guide for agents
- [ ] State management best practices
- [ ] Context strategy guide
- [ ] Migration from v0.1

## Implementation Guidelines

### For Each Task:
- Write tests first (TDD)
- Keep changes atomic
- Maintain backwards compatibility
- Document as you code
- Benchmark if performance-critical

### Time Estimates:
- Phase A: 5-6 days
- Phase B: 4-5 days
- Phase C: 4-5 days
- Phase D: 2-3 days

**Total: 15-20 days**

### Success Metrics:
- [ ] All tests passing
- [ ] No breaking changes
- [ ] Performance benchmarks included
- [ ] Examples demonstrate value
- [ ] Documentation complete

## Future Considerations

After these core features:
- **Usage Tracking** - Cost awareness for agents
- **Advanced State Stores** - Redis, PostgreSQL backends
- **State Branching** - For exploratory agents
- **Context Streaming** - For real-time pruning
- **Multi-Modal Context** - Image/audio awareness
