# Technical Design Document

## Project: Rust LLM Orchestration Crate v1.0

**Document Version:** 0.1‑draft   |  **Date:** 2025‑04‑19  |  **Authors:** Core Engineering Team

---

### 1  Purpose

This Technical Design Document (TDD) translates the Product Requirements (PRD) for **Rust LLM Orchestration Crate v1.0** into an implementable design. It specifies architecture, module boundaries, public APIs, data models, concurrency strategy, security posture, and the plan for testing, deployment, and maintenance.\
Readers should be able to:

- Understand how each PRD capability will be realized in code.
- Navigate the codebase and extend it safely.
- Gauge non‑functional qualities (performance, safety, extensibility).

---

### 2  Scope

Included:

- v1.0 runtime crate(s) published on crates.io.
- Procedural‑macro crate for compile‑time helpers.
- Example & test projects.

Excluded (future work):

- Autonomous planner / ReAct loop helper.
- Vector‑search memory backend.
- Web UI components.

---

### 3  System Context

```
┌─────────────────────┐          ┌──────────────────────────┐
│   Application Code  │─────────▶│  rust‑llm‑orchestrator   │
└─────────────────────┘  lib.rs  │  (this project)          │
                                 └──────────────────────────┘
          ▲                                     ▲
          │ HTTP/gRPC/IPC                        │ async/REST/local
          │                                     │
┌─────────┴───────┐        ┌────────────────────┴───────────┐
│  External Tools │        │      LLM Providers & Candle    │
└─────────────────┘        └────────────────────────────────┘
```

- **Upstream callers:** Any Rust binary/web‑service that links the crate.
- **Downstream deps:** 3rd‑party crates (reqwest, sqlx, redis‑rs, tokio), provider APIs, MCP servers, etc.

---

### 4  High‑Level Architecture

```
crate workspace
├── orchestrator‑core   (pure Rust, no heavy deps)
│   ├── prompt.rs
│   ├── chain.rs
│   ├── agent.rs
│   ├── tool.rs
│   └── memory.rs
├── providers‑openai    (feature="openai")
├── providers‑anthropic (feature="anthropic")
├── providers‑ollama    (feature="ollama")
├── tools‑search        (feature="search_tool")
├── tools‑code          (feature="code_tool")
├── tools‑math          (feature="math_tool")
├── memory‑sqlite       (feature="mem_sqlite")
├── memory‑redis        (feature="mem_redis")
├── macros              (proc‑macro crate)
└── examples
```

- **Core** exposes stable traits & data types; all optional integrations compile‑gate behind Cargo *feature* flags.
- **Macros** compile‑time‑generate strongly‑typed wrappers for prompts & chains.
- **Async boundary:** entire public API is `async` (Tokio 1.37 LTS).

---

### 5  Key Interfaces

#### 5.1 LanguageModel Trait (crate::llm)

```rust
#[async_trait]
pub trait LanguageModel: Send + Sync + 'static {
    type Prompt;
    type Response;

    async fn generate(&self, prompt: Self::Prompt, opts: GenerateOptions)
        -> Result<Self::Response, LlmError>;

    fn name(&self) -> &'static str;
}
```

- **Associated types** allow provider‑specific metadata while upholding generic workflows.
- Blanket `impl<T: LanguageModel> Clone` when underlying client is `Arc`‑wrapped.

#### 5.2 Tool Trait (crate::tool)

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    type Input: DeserializeOwned + Send + Sync;
    type Output: Serialize + Send + Sync;

    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError>;
    fn spec(&self) -> ToolSpec;      // name, description, JSON schema
}
```

- JSON‑schema in `ToolSpec` enables automatic function‑calling prompts.

#### 5.3 MemoryStore Trait (crate::memory)

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn load(&self, session: &SessionId, n: usize)
        -> Result<Vec<MemoryEntry>, MemoryError>;
    async fn save(&self, session: &SessionId, entry: MemoryEntry)
        -> Result<(), MemoryError>;
}
```

---

### 6  Data Types

| Type             | Purpose                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------- |
| `PromptTemplate` | Parsed template with placeholders; exposes `render(&ctx) -> String` and compile‑time check by macro. |
| `ChainStep<I,O>` | Enum { LlmCall, ToolCall } with phantom types for I/O; enforces sequence typing.                     |
| `Chain<I,O>`     | Vec\<ChainStep<…>>; generic over initial & final types.                                              |
| `Agent`          | Runtime executor holding model, tools registry, memory, concurrency policy.                          |
| `ToolInvocation` | Captures tool name, serialized input, start/end timestamps for tracing.                              |
| `MemoryEntry`    | `{ role: Role, content: String, ts: DateTime<Utc> }`                                                 |

---

### 7  Concurrency Model

- Tokio multi‑thread runtime is required (`features=["rt-multi-thread"]`).
- I/O‑bound ops (`reqwest`, `redis`, `sqlx`) use async natively.
- CPU‑bound ops (local Candle inference, sandboxed code) are wrapped via `spawn_blocking`.
- Parallel branches: `Chain::parallel(Vec<Chain<…>>)` returns `FuturesUnordered` internally.
- Cancellation: propagate `tokio::time::timeout` from `GenerateOptions` & tool options.

---

### 8  Error Handling & Result Types

- All public fns return `Result<T, Error>` where `Error` is an enum tree with `
    Llm(LlmError) | Tool(ToolError) | Memory(MemoryError) | Chain(ChainError) | …`.
- Implements `thiserror::Error` and `std::error::Error`.
- `retry` util (exponential backoff, jitter) auto‑wraps transient HTTP 5xx / rate‑limit codes.

---

### 9  Security Design

- **Secrets:** Provider keys read from `std::env` or supplied via builder; never logged.
- **Code Interpreter Tool:** Runs in isolated process launched through `wasmtime` (WASI) or fallback to `subprocess` with `seccomp` + `ulimit`. Max CPU sec & mem MB configurable.
- **Input validation:** Tool inputs deserialized with `serde_json` strict mode; refuse unknown fields.
- **MCP client:** TLS by default; allows only whitelisted function names.

---

### 10  Configuration

```toml
[llm]
provider = "openai"   # or "anthropic", "ollama"
model    = "gpt-4o"
api_key  = "${OPENAI_API_KEY}"

[memory]
backend = "sqlite"
path    = "chat.db"

[tools.search]
api_key = "${SERPAPI_KEY}"
```

- Parsed via `config` crate; injected into builders.

---

### 11  Testing Strategy

| Level            | What                                                           | Tooling                                     |
| ---------------- | -------------------------------------------------------------- | ------------------------------------------- |
| **Unit**         | Template rendering, macro expansion, error mapping             | `cargo test`, `trybuild` for macro UI tests |
| **Integration**  | OpenAI mock server (via `wiremock`), SQLite & Redis containers | GitHub Actions matrix                       |
| **E2E Examples** | Compile & run `examples/` with real keys on schedule           | self‑hosted runner (secrets)                |
| **Benchmarks**   | `criterion` compare chain vs raw call; regression gate         | nightly CI optional                         |

---

### 12  CI/CD Pipeline

1. **Lint + Fmt:** `cargo fmt --check`, `clippy --all-targets -- -D warnings`.
2. **Tests:** Unit + integration (mock).
3. **Doc:** `cargo doc --no-deps`; deploy to GitHub Pages.
4. **MSRV check:** 1.74.
5. **Publish:** manual gated job tags commit, runs `cargo publish --dry-run` then publish.

---

### 13  Risks & Mitigations

| Risk                          | Impact | Mitigation                                                           |
| ----------------------------- | ------ | -------------------------------------------------------------------- |
| Provider API breaking changes | High   | Abstract via traits; version‑pin; smoke tests run nightly            |
| Sandbox escape in Code Tool   | High   | Default to WASI; document security warnings; allow disabling feature |
| Compile‑time macro complexity | Medium | Keep macros thin; maintain trybuild test suite                       |
| Performance regressions       | Medium | Benchmark CI; profiling with `tokio-console`, flamegraphs            |

---

### 14  Milestones

| Date (2025) | Deliverable                                            |
| ----------- | ------------------------------------------------------ |
| May 02      | Workspace skeleton, core traits, OpenAI provider spike |
| May 30      | Prompt macros, Chain executor, SQLite memory           |
| Jun 27      | Tool plugin framework + SearchTool │                   |
| Jul 18      | CodeInterpreter & Math tools, Candle local model       |
| Aug 08      | Redis & Postgres memory, feature flags stabilized      |
| Aug 29      | Beta 0.1 crates.io, docs preview                       |
| Sep 26      | v1.0 release, announce, blog & examples                |

---

### 15  Resolved Design Decisions

| Topic | Decision |
|-------|----------|
| **Proc‑macro hygiene** | The `prompt!{}` procedural macro will *generate a dedicated args struct* (e.g. `struct GreetingArgs { name: String, age: u32 }`) that implements an auto‑derived `PromptArgs` trait. Compile‑time placeholder validation falls out naturally because the generated struct’s constructor requires every placeholder and exposes typed fields. We will not rely on const‑eval hacks. Macro expansions are unit‑tested with `trybuild` to guarantee helpful error messages. |
| **Streaming responses** | `LanguageModel` acquires an associated type <br>`type TokenStream: Stream<Item = Result<String, LlmError>> + Send + 'static;` <br>and a second async method: <br>`async fn stream_generate(&self, prompt: Self::Prompt, opts: GenerateOptions) -> Result<Self::TokenStream, LlmError>;` <br>The default blanket impl adapts non‑streaming models by wrapping the full text into a single‑item stream, so existing providers remain source‑compatible. Providers with native SSE/gRPC streaming override this for true token‑level streaming. |
| **Tool autonomy metadata** | `ToolSpec` is promoted to a first‑class, serializable struct: `{ name, description, input_schema, output_schema, examples }`. A `ToolRegistry` (HashMap<String, Arc<dyn Tool>> + Vec<ToolSpec>) is exposed via `Agent::registry()`. Even though v1.0 keeps developer‑scripted chains, the registry’s machine‑readable JSON enables future agent planners or LLM function‑calling prompts without breaking the public API. |

These decisions have been propagated to Sections 5 (Key Interfaces), 6 (Data Types), and 9 (Security).  

### 16  Appendix: Trait Example – SearchTool  Appendix: Trait Example – SearchTool

```rust
pub struct SearchTool {
    client: SerpClient,
}

#[derive(Deserialize)]
pub struct SearchInput { query: String, top_k: usize }

#[derive(Serialize)]
pub struct SearchOutput { snippets: Vec<String>, links: Vec<String> }

#[async_trait]
impl Tool for SearchTool {
    type Input = SearchInput;
    type Output = SearchOutput;

    async fn invoke(&self, input: Self::Input) -> ToolResult<Self::Output> {
        let resp = self.client.search(&input.query, input.top_k).await?;
        Ok(SearchOutput { snippets: resp.snippets, links: resp.urls })
    }

    fn spec(&self) -> ToolSpec { … }
}
```

---

*End of Document*

