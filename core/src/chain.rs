use crate::llm::LanguageModel;
use crate::prompt::PromptTemplate;
use crate::tool::Tool;
use async_trait::async_trait;
use futures::{StreamExt, stream::FuturesUnordered};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant, timeout};
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

/// Error type for chain execution
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("Chain execution timed out after {duration:?} in {step_type} step")]
    Timeout {
        duration: Duration,
        step_type: &'static str,
    },
    #[error("Chain execution was cancelled")]
    Cancelled,
    #[error("Parallel chain error: {message}")]
    ParallelError {
        message: String,
        successful_results: Vec<Box<dyn std::any::Any + Send>>,
    },
    #[error("Chain execution failed: {0}")]
    Other(#[from] anyhow::Error),
}

/// Configuration for chain execution
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Total timeout for the entire chain execution
    pub total_timeout: Option<Duration>,
    /// Default timeout for individual steps if not specified
    pub default_step_timeout: Duration,
    /// Whether to fail fast on parallel chain errors
    pub fail_fast: bool,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            total_timeout: None,
            default_step_timeout: Duration::from_secs(30),
            fail_fast: true,
        }
    }
}

/// A step in the chain
#[derive(Clone)]
pub enum ChainStep<I, O> {
    Llm {
        model: Arc<dyn LanguageModel<Input = I, Output = O> + Send + Sync>,
        prompt: Arc<dyn PromptTemplate<I> + Send + Sync>,
        timeout: Option<Duration>,
    },
    Tool {
        tool: Arc<dyn Tool<Input = I, Output = O> + Send + Sync>,
        timeout: Option<Duration>,
    },
    Parallel(Vec<Chain<I, O>>),
}

/// A chain of steps that can be executed sequentially or in parallel
pub struct Chain<I, O> {
    steps: Vec<ChainStep<I, O>>,
    config: ChainConfig,
    cancel_tx: broadcast::Sender<()>,
}

impl<I, O> Chain<I, O> {
    /// Create a new chain with default configuration
    pub fn new() -> Self {
        Self::with_config(ChainConfig::default())
    }

    /// Create a new chain with custom configuration
    pub fn with_config(config: ChainConfig) -> Self {
        let (cancel_tx, _) = broadcast::channel(1);
        Self {
            steps: Vec::new(),
            config,
            cancel_tx,
        }
    }

    /// Add an LLM step to the chain
    pub fn add_llm<M, P>(mut self, model: M, prompt: P, timeout: Option<Duration>) -> Self
    where
        M: LanguageModel<Input = I, Output = O> + Send + Sync + 'static,
        P: PromptTemplate<I> + Send + Sync + 'static,
    {
        self.steps.push(ChainStep::Llm {
            model: Arc::new(model),
            prompt: Arc::new(prompt),
            timeout,
        });
        self
    }

    /// Add a tool step to the chain
    pub fn add_tool<T>(mut self, tool: T, timeout: Option<Duration>) -> Self
    where
        T: Tool<Input = I, Output = O> + Send + Sync + 'static,
    {
        self.steps.push(ChainStep::Tool {
            tool: Arc::new(tool),
            timeout,
        });
        self
    }

    /// Add parallel chains to execute
    pub fn add_parallel<C>(mut self, chains: Vec<C>) -> Self
    where
        C: Into<Chain<I, O>>,
    {
        let chains: Vec<_> = chains
            .into_iter()
            .map(|c| {
                let mut chain = c.into();
                // Share configuration and cancellation
                chain.config = self.config.clone();
                chain.cancel_tx = self.cancel_tx.clone();
                chain
            })
            .collect();
        self.steps.push(ChainStep::Parallel(chains));
        self
    }

    /// Cancel the chain execution
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(());
    }

    /// Execute the chain with the given input
    #[instrument(skip_all, fields(chain_len = self.steps.len()))]
    pub async fn execute(&self, input: I) -> Result<O, ChainError> {
        info!("Starting chain execution");
        let start_time = Instant::now();
        let mut current_input = input;
        let mut cancel_rx = self.cancel_tx.subscribe();

        // Apply total timeout if configured
        let execute_future = async {
            for (step_idx, step) in self.steps.iter().enumerate() {
                // Check remaining time for total timeout
                if let Some(total_timeout) = self.config.total_timeout {
                    let elapsed = start_time.elapsed();
                    if elapsed >= total_timeout {
                        error!("Total chain timeout exceeded");
                        return Err(ChainError::Timeout {
                            duration: total_timeout,
                            step_type: "total",
                        });
                    }
                }

                // Check for cancellation
                if cancel_rx.try_recv().is_ok() {
                    info!("Chain execution cancelled");
                    return Err(ChainError::Cancelled);
                }

                match step {
                    ChainStep::Llm {
                        model,
                        prompt,
                        timeout,
                    } => {
                        let span = info_span!("llm_step", step_idx, model = model.name());
                        let timeout = timeout.unwrap_or(self.config.default_step_timeout);
                        debug!("Executing LLM step with timeout {:?}", timeout);

                        let result =
                            timeout_wrapper(timeout, model.generate(prompt, current_input), "llm")
                                .instrument(span)
                                .await?;

                        current_input = result;
                        info!("LLM step completed successfully");
                    }
                    ChainStep::Tool { tool, timeout } => {
                        let span = info_span!("tool_step", step_idx, tool = tool.spec().name);
                        let timeout = timeout.unwrap_or(self.config.default_step_timeout);
                        debug!("Executing tool step with timeout {:?}", timeout);

                        let result = timeout_wrapper(timeout, tool.invoke(current_input), "tool")
                            .instrument(span)
                            .await?;

                        current_input = result;
                        info!("Tool step completed successfully");
                    }
                    ChainStep::Parallel(chains) => {
                        let span =
                            info_span!("parallel_step", step_idx, chain_count = chains.len());
                        debug!("Executing parallel chains");

                        let mut futures = FuturesUnordered::new();
                        for (chain_idx, chain) in chains.iter().enumerate() {
                            let chain_span = info_span!("parallel_chain", chain_idx);
                            futures
                                .push(chain.execute(current_input.clone()).instrument(chain_span));
                        }

                        let mut results = Vec::new();
                        let mut errors = Vec::new();

                        while let Some(result) = futures.next().await {
                            // Check for cancellation
                            if cancel_rx.try_recv().is_ok() {
                                // Cancel all remaining chains
                                for chain in chains {
                                    chain.cancel();
                                }
                                info!("Parallel chains cancelled");
                                return Err(ChainError::Cancelled);
                            }

                            match result {
                                Ok(output) => {
                                    debug!("Parallel chain completed successfully");
                                    results.push(output);
                                }
                                Err(e) => {
                                    warn!("Parallel chain failed: {}", e);
                                    errors.push(e);
                                    if self.config.fail_fast {
                                        // Cancel remaining chains on first error
                                        for chain in chains {
                                            chain.cancel();
                                        }
                                        error!("Some parallel chains failed");
                                        return Err(ChainError::ParallelError {
                                            message: format!("Chain failed: {}", e),
                                            successful_results: results
                                                .into_iter()
                                                .map(|r| {
                                                    Box::new(r) as Box<dyn std::any::Any + Send>
                                                })
                                                .collect(),
                                        });
                                    }
                                }
                            }
                        }

                        if !errors.is_empty() && !self.config.fail_fast {
                            error!("Some parallel chains failed");
                            return Err(ChainError::ParallelError {
                                message: format!(
                                    "{} out of {} chains failed",
                                    errors.len(),
                                    chains.len()
                                ),
                                successful_results: results
                                    .into_iter()
                                    .map(|r| Box::new(r) as Box<dyn std::any::Any + Send>)
                                    .collect(),
                            });
                        }

                        current_input = results;
                        info!("All parallel chains completed successfully");
                    }
                }
            }

            info!("Chain execution completed successfully");
            Ok(current_input)
        };

        // Apply total timeout if configured
        match self.config.total_timeout {
            Some(total_timeout) => timeout_wrapper(total_timeout, execute_future, "total").await,
            None => execute_future.await,
        }
    }
}

impl<I, O> Default for Chain<I, O> {
    fn default() -> Self {
        Self::new()
    }
}

async fn timeout_wrapper<F, T>(
    duration: Duration,
    future: F,
    step_type: &'static str,
) -> Result<T, ChainError>
where
    F: Future<Output = Result<T, ChainError>>,
{
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => {
            error!("Step timed out after {:?}", duration);
            Err(ChainError::Timeout {
                duration,
                step_type,
            })
        }
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
            .add_llm(
                llm.clone(),
                PromptTemplate::new("test prompt"),
                Duration::from_secs(30),
            )
            .add_tool(tool.clone(), Duration::from_secs(30));

        let result = chain.execute("input".to_string()).await.unwrap();
        assert!(result.contains("Tool processed: LLM processed: test prompt"));
    }

    #[tokio::test]
    async fn test_parallel_chain() {
        let llm1 = Arc::new(MockLlm);
        let llm2 = Arc::new(MockLlm);

        let chain1 = Chain::new().add_llm(
            llm1.clone(),
            PromptTemplate::new("prompt 1"),
            Duration::from_secs(30),
        );
        let chain2 = Chain::new().add_llm(
            llm2.clone(),
            PromptTemplate::new("prompt 2"),
            Duration::from_secs(30),
        );

        let chain = Chain::new().add_parallel(vec![chain1, chain2]);

        let result = chain.execute("input".to_string()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout() {
        let llm = Arc::new(MockLlm);

        let chain = Chain::new().add_llm(
            llm.clone(),
            PromptTemplate::new("test prompt"),
            Duration::from_nanos(1),
        );

        let result = chain.execute("input".to_string()).await;
        assert!(matches!(result, Err(ChainError::Timeout)));
    }
}
