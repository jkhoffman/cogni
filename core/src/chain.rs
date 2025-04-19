use crate::llm::LanguageModel;
use crate::prompt::PromptTemplate;
use crate::tool::Tool;
use async_trait::async_trait;
use futures::{StreamExt, stream::FuturesUnordered};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{Duration, timeout};

/// Error type for chain execution
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("llm error: {0}")]
    Llm(#[from] crate::llm::LlmError),
    #[error("tool error: {0}")]
    Tool(#[from] crate::tool::ToolError),
    #[error("step timed out")]
    Timeout,
    #[error("chain was cancelled")]
    Cancelled,
}

/// A single step in a chain, either an LLM call or a tool invocation
#[derive(Debug)]
pub enum ChainStep<I, O> {
    /// Call an LLM with a prompt
    Llm {
        model: Arc<dyn LanguageModel>,
        prompt: PromptTemplate,
        timeout: Duration,
    },
    /// Call a tool with input
    Tool {
        tool: Arc<dyn Tool>,
        timeout: Duration,
    },
    /// Run multiple steps in parallel
    Parallel(Vec<Chain<I, O>>),
}

/// A chain of steps that can be executed sequentially or in parallel
#[derive(Debug)]
pub struct Chain<I, O> {
    steps: Vec<ChainStep<I, O>>,
}

impl<I, O> Chain<I, O> {
    /// Create a new empty chain
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add an LLM step to the chain
    pub fn llm(mut self, model: Arc<dyn LanguageModel>, prompt: PromptTemplate) -> Self {
        self.steps.push(ChainStep::Llm {
            model,
            prompt,
            timeout: Duration::from_secs(30), // Default timeout
        });
        self
    }

    /// Add a tool step to the chain
    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.steps.push(ChainStep::Tool {
            tool,
            timeout: Duration::from_secs(30), // Default timeout
        });
        self
    }

    /// Add parallel steps to the chain
    pub fn parallel(mut self, chains: Vec<Chain<I, O>>) -> Self {
        self.steps.push(ChainStep::Parallel(chains));
        self
    }

    /// Set timeout for the last added step
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        if let Some(step) = self.steps.last_mut() {
            match step {
                ChainStep::Llm { timeout, .. } => *timeout = duration,
                ChainStep::Tool { timeout, .. } => *timeout = duration,
                ChainStep::Parallel(_) => {} // Parallel steps handle their own timeouts
            }
        }
        self
    }

    /// Execute the chain with the given input
    pub async fn execute(&self, input: I) -> Result<O, ChainError> {
        let mut current_input = input;

        for step in &self.steps {
            match step {
                ChainStep::Llm {
                    model,
                    prompt,
                    timeout,
                } => {
                    let result =
                        timeout_wrapper(*timeout, model.generate(prompt, current_input)).await?;
                    current_input = result;
                }
                ChainStep::Tool { tool, timeout } => {
                    let result = timeout_wrapper(*timeout, tool.invoke(current_input)).await?;
                    current_input = result;
                }
                ChainStep::Parallel(chains) => {
                    let mut futures = FuturesUnordered::new();
                    for chain in chains {
                        futures.push(chain.execute(current_input.clone()));
                    }

                    let mut results = Vec::new();
                    while let Some(result) = futures.next().await {
                        results.push(result?);
                    }
                    current_input = results;
                }
            }
        }

        Ok(current_input)
    }
}

async fn timeout_wrapper<F, T>(duration: Duration, future: F) -> Result<T, ChainError>
where
    F: Future<Output = Result<T, ChainError>>,
{
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(ChainError::Timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{GenerateOptions, LlmError};
    use crate::tool::ToolError;
    use std::time::Duration;

    #[derive(Clone)]
    struct MockLlm;

    #[async_trait]
    impl LanguageModel for MockLlm {
        type Prompt = String;
        type Response = String;

        async fn generate(
            &self,
            prompt: Self::Prompt,
            _opts: GenerateOptions,
        ) -> Result<Self::Response, LlmError> {
            Ok(format!("LLM processed: {}", prompt))
        }

        fn name(&self) -> &'static str {
            "mock"
        }
    }

    #[derive(Clone)]
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        type Input = String;
        type Output = String;

        async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
            Ok(format!("Tool processed: {}", input))
        }

        fn spec(&self) -> crate::tool::ToolSpec {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_sequential_chain() {
        let llm = Arc::new(MockLlm);
        let tool = Arc::new(MockTool);

        let chain = Chain::new()
            .llm(llm.clone(), PromptTemplate::new("test prompt"))
            .tool(tool.clone());

        let result = chain.execute("input".to_string()).await.unwrap();
        assert!(result.contains("Tool processed: LLM processed: test prompt"));
    }

    #[tokio::test]
    async fn test_parallel_chain() {
        let llm1 = Arc::new(MockLlm);
        let llm2 = Arc::new(MockLlm);

        let chain1 = Chain::new().llm(llm1.clone(), PromptTemplate::new("prompt 1"));
        let chain2 = Chain::new().llm(llm2.clone(), PromptTemplate::new("prompt 2"));

        let chain = Chain::new().parallel(vec![chain1, chain2]);

        let result = chain.execute("input".to_string()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout() {
        let llm = Arc::new(MockLlm);

        let chain = Chain::new()
            .llm(llm.clone(), PromptTemplate::new("test prompt"))
            .with_timeout(Duration::from_nanos(1));

        let result = chain.execute("input".to_string()).await;
        assert!(matches!(result, Err(ChainError::Timeout)));
    }
}
