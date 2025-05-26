use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Model not supported: {0}")]
    UnsupportedModel(String),

    #[error("Token counting error: {0}")]
    TokenCountingError(String),

    #[error("Context exceeded: {current} tokens > {max} tokens")]
    ContextExceeded { current: usize, max: usize },

    #[error("Pruning failed: {0}")]
    PruningError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Tiktoken error: {0}")]
    TiktokenError(String),
}
