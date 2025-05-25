//! Ollama provider implementation

mod config;
mod converter;
mod parser;
mod provider;
mod stream;

pub use config::OllamaConfig;
pub use provider::Ollama;