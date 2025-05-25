//! High-level client API for LLM interactions
//!
//! This module provides a simplified interface for common LLM operations,
//! with a fluent builder API and convenience methods.


mod builder;
mod client;
mod middleware;
mod parallel;

pub use builder::RequestBuilder;
pub use client::Client;
pub use middleware::MiddlewareProvider;
pub use parallel::{parallel_chat, parallel_requests, ParallelClient, ExecutionStrategy, create_parallel_client};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{Client, RequestBuilder};
    pub use cogni_core::{Content, Message, Role};
}