# MemoryStore Trait Extension: Efficient History Access

## Overview

This document specifies the extension to the `MemoryStore` trait for efficient conversation history access, as required by the "Memory Integration Improvements" checklist. The design supports paginated retrieval, range queries by timestamp, and optional filtering by role or content substring. All memory backends (SQLite, Redis, etc.) must implement the same API for consistency.

---

## 1. Query Struct

```rust
/// Query parameters for retrieving conversation history.
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub session: SessionId,
    pub offset: Option<usize>,           // For pagination (start index)
    pub limit: Option<usize>,            // For pagination (max results)
    pub start_time: Option<OffsetDateTime>, // Range: entries >= this timestamp
    pub end_time: Option<OffsetDateTime>,   // Range: entries <= this timestamp
    pub role: Option<Role>,              // Filter by role
    pub content_substring: Option<String>, // Filter by substring in content
}
```

---

## 2. Trait Method

```rust
/// Query conversation history with flexible filters and pagination.
///
/// Returns entries matching the query parameters, ordered by timestamp ascending.
/// - Pagination: Use `offset` and `limit` for page-based access.
/// - Range: Use `start_time` and/or `end_time` for timestamp filtering.
/// - Filtering: Use `role` and/or `content_substring` for selective retrieval.
///
/// All backends must implement efficient querying for these parameters.
/// If a parameter is `None`, it is not used as a filter.
async fn query_history(
    &self,
    query: MemoryQuery,
) -> Result<Vec<MemoryEntry>, MemoryError>;
```

---

## 3. Documentation Requirements for Backends

- All `MemoryStore` backends (SQLite, Redis, etc.) must implement `query_history` with efficient support for:
  - Pagination (offset/limit)
  - Timestamp range queries
  - Filtering by role and content substring
- If a backend cannot efficiently support a filter, it must document the limitation and provide best-effort behavior.
- The method must return entries ordered by timestamp ascending.
- Errors must use `MemoryError` variants.

---

## 4. Example Usage

```rust
let query = MemoryQuery {
    session: session_id,
    offset: Some(20),
    limit: Some(10),
    start_time: Some(time::OffsetDateTime::now_utc() - time::Duration::days(1)),
    end_time: None,
    role: Some(Role::User),
    content_substring: Some("hello".to_string()),
};
let entries = store.query_history(query).await?;
```

---

## 5. Query Flow Diagram

```mermaid
flowchart TD
    User[User/Agent] -->|builds MemoryQuery| API[MemoryStore::query_history]
    API -->|delegates| Backend[Memory Backend (SQLite/Redis/etc.)]
    Backend -->|applies filters: session, offset, limit, time, role, content| DB[Data Store]
    DB -->|returns Vec<MemoryEntry>| Backend
    Backend -->|returns result| API
    API -->|returns result| User
```

---

## 6. Error Handling

The `query_history` method must return errors using the existing `MemoryError` enum:

- `Database(String)`: Underlying storage or query error.
- `SessionNotFound(String)`: The specified session does not exist.
- `InvalidFormat(String)`: Data format or query parameter error.

---

## 7. Summary

This API extension provides a unified, efficient, and flexible interface for accessing conversation history across all memory backends. It enables advanced retrieval patterns required for LLM orchestration and agent frameworks.