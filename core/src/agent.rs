//! Agent implementation for the Cogni framework.
//!
//! This module provides the core abstractions for implementing agents that can:
//! 1. Use language models to process information
//! 2. Select and invoke appropriate tools
//! 3. Store and retrieve information from memory
//! 4. Implement planning capabilities
//!
//! # Agent Lifecycle
//!
//! Agents follow a defined lifecycle:
//! 1. Creation - Agent is instantiated with its configuration
//! 2. Initialization - Agent performs setup (connecting to services, loading resources)
//! 3. Operation - Agent handles inputs and executes steps
//! 4. Shutdown - Agent performs cleanup
//!
//! # Usage Example
//!
//! ```rust,no_run
//! // Example will be provided as the implementation progresses
//! ```

// Re-export all agent-related traits and types from the traits module
pub use crate::traits::agent::*;
