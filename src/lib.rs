//! Cogni - A Rust framework for LLM orchestration
//!
//! This crate provides a high-level interface for orchestrating Large Language Models (LLMs)
//! and tools in Rust. It is built on top of the core traits and implementations provided by
//! the cogni-core crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use cogni_core::{
    Chain, ChainConfig, ChainError, ChainMetrics, ChainStep, GenerateOptions, LanguageModel,
    LlmError, PromptArgs, PromptError, PromptTemplate, StepType, Tool, ToolCapability, ToolConfig,
    ToolError, ToolSpec,
};
