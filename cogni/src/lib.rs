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
//! # use cogni::providers::{OpenAI, openai::OpenAIConfig};
//! # 
//! # #[tokio::main]
//! # async fn main() -> Result<(), cogni::Error> {
//! #     #[cfg(feature = "providers")]
//! #     {
//!     // Create a provider
//!     let provider = OpenAI::new(
//!         OpenAIConfig::new("your-api-key")
//!     )?;
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

/// Prelude module for convenient imports
pub mod prelude {
    
    pub use cogni_core::{
        Error, Provider, Request, Response, StreamEvent, StreamAccumulator,
        Message, Content, Role, Model, Parameters,
    };
    
    #[cfg(feature = "providers")]
    pub use cogni_providers::{openai::OpenAIConfig};
    
    #[cfg(feature = "client")]
    pub use cogni_client::{Client, RequestBuilder};
}