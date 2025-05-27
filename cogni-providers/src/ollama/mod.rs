//! Ollama provider implementation

mod config;
mod converter;
mod parser;
mod provider;
mod stream;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod direct_stream_tests;

pub use config::OllamaConfig;
pub use provider::{Ollama, OllamaBuilder};
