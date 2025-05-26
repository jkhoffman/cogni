//! Cogni - A unified Rust library for LLM interactions
//!
//! This crate provides a clean, type-safe interface for interacting with various
//! Large Language Model providers including OpenAI, Anthropic, and Ollama.
//!
//! # Features
//!
//! - **Unified API**: Single interface for multiple LLM providers
//! - **Type Safety**: Leverage Rust's type system for compile-time guarantees
//! - **Async First**: Built on Tokio for efficient async operations
//! - **Streaming**: First-class support for streaming responses
//! - **Tool Calling**: Support for function/tool calling across providers
//! - **Extensible**: Easy to add new providers and middleware
//!
//! # Quick Start
//!
//! ```no_run
//! # use cogni::prelude::*;
//! # #[cfg(feature = "providers")]
//! # use cogni::providers::OpenAI;
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), cogni::Error> {
//! #     #[cfg(feature = "providers")]
//! #     {
//!     // Create a provider
//!     let provider = OpenAI::with_api_key("your-api-key");
//!
//!     // Create a request
//!     let request = Request::builder()
//!         .message(Message::user("Hello, world!"))
//!         .build();
//!
//!     // Get a response
//!     let response = provider.request(request).await?;
//!     println!("{}", response.content);
//! #     }
//! #     Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export core types
pub use cogni_core::*;

// Re-export derive macro when feature is enabled
#[cfg(feature = "derive")]
pub use cogni_core::DeriveStructuredOutput as StructuredOutput;

// Re-export feature-gated modules
#[cfg(feature = "providers")]
#[cfg_attr(docsrs, doc(cfg(feature = "providers")))]
pub mod providers {
    //! Provider implementations
    pub use cogni_providers::*;
}

#[cfg(feature = "middleware")]
#[cfg_attr(docsrs, doc(cfg(feature = "middleware")))]
pub mod middleware {
    //! Middleware for cross-cutting concerns
    pub use cogni_middleware::*;
}

#[cfg(feature = "tools")]
#[cfg_attr(docsrs, doc(cfg(feature = "tools")))]
pub mod tools {
    //! Tool execution framework
    pub use cogni_tools::*;
}

#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub mod client {
    //! High-level client API
    pub use cogni_client::*;
}

#[cfg(feature = "state")]
#[cfg_attr(docsrs, doc(cfg(feature = "state")))]
pub mod state {
    //! Conversation state persistence
    pub use cogni_state::*;
}

#[cfg(feature = "context")]
#[cfg_attr(docsrs, doc(cfg(feature = "context")))]
pub mod context {
    //! Context window management
    pub use cogni_context::*;
}

/// Prelude module for convenient imports
pub mod prelude {

    pub use cogni_core::{
        Content, Error, Message, Model, Parameters, Provider, Request, Response, Role,
        StreamAccumulator, StreamEvent,
    };

    #[cfg(feature = "providers")]
    pub use cogni_providers::openai::OpenAIConfig;

    #[cfg(feature = "client")]
    pub use cogni_client::{Client, RequestBuilder};
}
