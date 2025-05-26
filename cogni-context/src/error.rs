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

impl From<ContextError> for cogni_core::Error {
    fn from(err: ContextError) -> Self {
        cogni_core::Error::Validation(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ContextError::UnsupportedModel("gpt-5".to_string());
        assert_eq!(err.to_string(), "Model not supported: gpt-5");

        let err = ContextError::TokenCountingError("Failed to initialize".to_string());
        assert_eq!(
            err.to_string(),
            "Token counting error: Failed to initialize"
        );

        let err = ContextError::ContextExceeded {
            current: 5000,
            max: 4096,
        };
        assert_eq!(
            err.to_string(),
            "Context exceeded: 5000 tokens > 4096 tokens"
        );

        let err = ContextError::PruningError("No messages to prune".to_string());
        assert_eq!(err.to_string(), "Pruning failed: No messages to prune");

        let err = ContextError::InvalidConfiguration("Invalid strategy".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: Invalid strategy");

        let err = ContextError::TiktokenError("Model not found".to_string());
        assert_eq!(err.to_string(), "Tiktoken error: Model not found");
    }

    #[test]
    fn test_error_conversion_to_core_error() {
        let context_err = ContextError::UnsupportedModel("test-model".to_string());
        let core_err: cogni_core::Error = context_err.into();

        match core_err {
            cogni_core::Error::Validation(msg) => {
                assert_eq!(msg, "Model not supported: test-model");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_all_error_variants_convert() {
        let errors = vec![
            ContextError::UnsupportedModel("model".to_string()),
            ContextError::TokenCountingError("error".to_string()),
            ContextError::ContextExceeded {
                current: 100,
                max: 50,
            },
            ContextError::PruningError("error".to_string()),
            ContextError::InvalidConfiguration("error".to_string()),
            ContextError::TiktokenError("error".to_string()),
        ];

        for err in errors {
            let core_err: cogni_core::Error = err.into();
            assert!(matches!(core_err, cogni_core::Error::Validation(_)));
        }
    }
}
