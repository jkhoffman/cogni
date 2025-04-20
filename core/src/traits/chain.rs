use crate::error::ChainError;
use async_trait::async_trait;

/// A trait for types that can be executed as part of a chain
#[async_trait]
pub trait ChainExecutor {
    /// The input type for this executor
    type Input;
    /// The output type for this executor
    type Output;

    /// Execute this chain step with the given input
    async fn execute(&self, input: Self::Input) -> Result<Self::Output, ChainError>;

    /// Cancel execution of this chain step
    fn cancel(&self);
}
