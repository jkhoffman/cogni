use crate::llm::LanguageModel;
use crate::prompt::PromptTemplate;
use crate::tool::Tool;
use async_trait::async_trait;
use futures::{StreamExt, stream::FuturesUnordered};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
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
    #[error("Resource cleanup failed: {0}")]
    CleanupError(String),
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
    /// Maximum number of parallel chains
    pub max_parallel_chains: usize,
    /// Whether to perform cleanup on drop
    pub cleanup_on_drop: bool,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            total_timeout: None,
            default_step_timeout: Duration::from_secs(30),
            fail_fast: true,
            max_parallel_chains: 10,
            cleanup_on_drop: true,
        }
    }
}

/// Resource handle for tracking active resources
#[derive(Debug)]
struct ResourceHandle {
    id: String,
    resource_type: &'static str,
    cleanup_tx: mpsc::Sender<String>,
}

impl Drop for ResourceHandle {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup_tx.try_send(self.id.clone()) {
            error!(
                "Failed to send cleanup signal for resource {}: {}",
                self.id, e
            );
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
    cleanup_tx: mpsc::Sender<String>,
    cleanup_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    active_resources: Arc<Mutex<Vec<ResourceHandle>>>,
}

impl<I, O> Chain<I, O> {
    /// Create a new chain with default configuration
    pub fn new() -> Self {
        Self::with_config(ChainConfig::default())
    }

    /// Create a new chain with custom configuration
    pub fn with_config(config: ChainConfig) -> Self {
        let (cancel_tx, _) = broadcast::channel(1);
        let (cleanup_tx, cleanup_rx) = mpsc::channel(100);
        Self {
            steps: Vec::new(),
            config,
            cancel_tx,
            cleanup_tx,
            cleanup_rx: Arc::new(Mutex::new(cleanup_rx)),
            active_resources: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Track a new resource
    fn track_resource(&self, id: String, resource_type: &'static str) {
        if let Ok(mut resources) = self.active_resources.lock() {
            resources.push(ResourceHandle {
                id,
                resource_type,
                cleanup_tx: self.cleanup_tx.clone(),
            });
        }
    }

    /// Clean up resources
    async fn cleanup_resources(&self) -> Result<(), ChainError> {
        let mut errors = Vec::new();

        // Process cleanup signals
        if let Ok(mut rx) = self.cleanup_rx.lock() {
            while let Ok(resource_id) = rx.try_recv() {
                debug!("Cleaning up resource: {}", resource_id);
                // Actual cleanup logic would go here
                // For now we just log
                info!("Resource {} cleaned up", resource_id);
            }
        }

        // Remove tracked resources
        if let Ok(mut resources) = self.active_resources.lock() {
            resources.clear();
        }

        if !errors.is_empty() {
            Err(ChainError::CleanupError(format!(
                "Failed to clean up resources: {}",
                errors.join(", ")
            )))
        } else {
            Ok(())
        }
    }

    /// Add an LLM step to the chain
    pub fn add_llm<M, P>(mut self, model: M, prompt: P, timeout: Option<Duration>) -> Self
    where
        M: LanguageModel<Input = I, Output = O> + Send + Sync + 'static,
        P: PromptTemplate<I> + Send + Sync + 'static,
    {
        let model = Arc::new(model);
        let prompt = Arc::new(prompt);

        // Track the LLM resource
        self.track_resource(format!("llm_{}", model.name()), "llm");

        self.steps.push(ChainStep::Llm {
            model,
            prompt,
            timeout,
        });
        self
    }

    /// Add a tool step to the chain
    pub fn add_tool<T>(mut self, tool: T, timeout: Option<Duration>) -> Self
    where
        T: Tool<Input = I, Output = O> + Send + Sync + 'static,
    {
        let tool = Arc::new(tool);

        // Track the tool resource
        self.track_resource(format!("tool_{}", tool.spec().name), "tool");

        self.steps.push(ChainStep::Tool { tool, timeout });
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
                    self.cleanup_resources().await?;
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

                        // Track LLM resource
                        self.track_resource(format!("llm_{}_{}", model.name(), step_idx), "llm");

                        let result = timeout_wrapper(
                            timeout,
                            model.generate(prompt, current_input.clone()),
                            "llm",
                        )
                        .instrument(span)
                        .await?;

                        current_input = result;
                        info!("LLM step completed successfully");
                    }
                    ChainStep::Tool { tool, timeout } => {
                        let span = info_span!("tool_step", step_idx, tool = tool.spec().name);
                        let timeout = timeout.unwrap_or(self.config.default_step_timeout);
                        debug!("Executing tool step with timeout {:?}", timeout);

                        // Track tool resource
                        self.track_resource(
                            format!("tool_{}_{}", tool.spec().name, step_idx),
                            "tool",
                        );

                        let result =
                            timeout_wrapper(timeout, tool.invoke(current_input.clone()), "tool")
                                .instrument(span)
                                .await?;

                        current_input = result;
                        info!("Tool step completed successfully");
                    }
                    ChainStep::Parallel(chains) => {
                        if chains.len() > self.config.max_parallel_chains {
                            return Err(ChainError::Other(anyhow::anyhow!(
                                "Too many parallel chains: {} (max: {})",
                                chains.len(),
                                self.config.max_parallel_chains
                            )));
                        }

                        let span =
                            info_span!("parallel_step", step_idx, chain_count = chains.len());
                        debug!("Executing parallel chains");

                        let mut futures = FuturesUnordered::new();
                        for (chain_idx, chain) in chains.iter().enumerate() {
                            // Track parallel chain resource
                            self.track_resource(
                                format!("parallel_chain_{}_{}", step_idx, chain_idx),
                                "parallel_chain",
                            );

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
                                self.cleanup_resources().await?;
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
                                        self.cleanup_resources().await?;
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
                            self.cleanup_resources().await?;
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
        let result = match self.config.total_timeout {
            Some(total_timeout) => timeout_wrapper(total_timeout, execute_future, "total").await,
            None => execute_future.await,
        };

        // Clean up resources regardless of success/failure
        self.cleanup_resources().await?;

        result
    }
}

impl<I, O> Drop for Chain<I, O> {
    fn drop(&mut self) {
        if self.config.cleanup_on_drop {
            debug!("Running cleanup in drop");
            // Create a new runtime for cleanup if needed
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                if let Err(e) = rt.block_on(self.cleanup_resources()) {
                    error!("Failed to clean up resources during drop: {}", e);
                }
            }
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    static CLEANUP_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Clone)]
    struct MockLlm {
        name: String,
    }

    #[async_trait]
    impl LanguageModel for MockLlm {
        type Prompt = String;
        type Response = String;

        async fn generate(
            &self,
            prompt: Self::Prompt,
            _opts: GenerateOptions,
        ) -> Result<Self::Response, LlmError> {
            Ok(format!("LLM {} processed: {}", self.name, prompt))
        }

        fn name(&self) -> &'static str {
            "mock"
        }
    }

    #[derive(Clone)]
    struct MockTool {
        name: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        type Input = String;
        type Output = String;

        async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
            Ok(format!("Tool {} processed: {}", self.name, input))
        }

        fn spec(&self) -> crate::tool::ToolSpec {
            crate::tool::ToolSpec {
                name: self.name.clone(),
                description: "A mock tool".into(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                examples: vec![],
            }
        }
    }

    impl Drop for MockTool {
        fn drop(&mut self) {
            CLEANUP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_sequential_chain() {
        let llm = MockLlm {
            name: "llm1".into(),
        };
        let tool = MockTool {
            name: "tool1".into(),
        };

        let chain = Chain::new()
            .add_llm(
                llm.clone(),
                PromptTemplate::new("test prompt"),
                Some(Duration::from_secs(30)),
            )
            .add_tool(tool.clone(), Some(Duration::from_secs(30)));

        let result = chain.execute("input".to_string()).await.unwrap();
        assert!(result.contains("Tool tool1 processed: LLM mock processed: test prompt"));
    }

    #[tokio::test]
    async fn test_parallel_chain() {
        let llm1 = MockLlm {
            name: "llm1".into(),
        };
        let llm2 = MockLlm {
            name: "llm2".into(),
        };

        let chain1 = Chain::new().add_llm(
            llm1.clone(),
            PromptTemplate::new("prompt 1"),
            Some(Duration::from_secs(30)),
        );
        let chain2 = Chain::new().add_llm(
            llm2.clone(),
            PromptTemplate::new("prompt 2"),
            Some(Duration::from_secs(30)),
        );

        let chain = Chain::new().add_parallel(vec![chain1, chain2]);

        let result = chain.execute("input".to_string()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout() {
        let llm = MockLlm {
            name: "llm1".into(),
        };

        let chain = Chain::new().add_llm(
            llm.clone(),
            PromptTemplate::new("test prompt"),
            Some(Duration::from_nanos(1)),
        );

        let result = chain.execute("input".to_string()).await;
        assert!(matches!(result, Err(ChainError::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_resource_cleanup() {
        let initial_count = CLEANUP_COUNT.load(Ordering::SeqCst);

        let tool1 = MockTool {
            name: "tool1".into(),
        };
        let tool2 = MockTool {
            name: "tool2".into(),
        };

        let chain1 = Chain::new().add_tool(tool1.clone(), None);
        let chain2 = Chain::new().add_tool(tool2.clone(), None);

        {
            let chain = Chain::new().add_parallel(vec![chain1, chain2]);
            let _ = chain.execute("input".to_string()).await.unwrap();
        } // chain is dropped here

        let final_count = CLEANUP_COUNT.load(Ordering::SeqCst);
        assert!(final_count > initial_count, "Resources were not cleaned up");
    }

    #[tokio::test]
    async fn test_cleanup_on_cancel() {
        let initial_count = CLEANUP_COUNT.load(Ordering::SeqCst);

        let tool = MockTool {
            name: "tool1".into(),
        };
        let chain = Chain::new().add_tool(tool.clone(), Some(Duration::from_secs(30)));

        // Spawn the chain execution
        let chain_clone = chain.clone();
        let handle = tokio::spawn(async move { chain_clone.execute("input".to_string()).await });

        // Cancel the chain
        chain.cancel();

        let result = handle.await.unwrap();
        assert!(matches!(result, Err(ChainError::Cancelled)));

        let final_count = CLEANUP_COUNT.load(Ordering::SeqCst);
        assert!(
            final_count > initial_count,
            "Resources were not cleaned up after cancellation"
        );
    }

    #[tokio::test]
    async fn test_parallel_chain_limit() {
        let mut chains = Vec::new();
        for i in 0..15 {
            let tool = MockTool {
                name: format!("tool{}", i),
            };
            chains.push(Chain::new().add_tool(tool, None));
        }

        let chain = Chain::new().add_parallel(chains);
        let result = chain.execute("input".to_string()).await;

        assert!(matches!(result, Err(ChainError::Other(_))));
    }
}
