use crate::llm::LanguageModel;
use crate::prompt::PromptTemplate;
use crate::tool::Tool;
use async_trait::async_trait;
use futures::{StreamExt, stream::FuturesUnordered};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;
use tracing::{Instrument, Level, Span, debug, error, field, info, info_span, instrument, warn};

/// Metrics for chain execution
#[derive(Debug, Clone)]
pub struct ChainMetrics {
    /// Total execution time
    pub total_duration: Duration,
    /// Number of steps executed
    pub total_steps: usize,
    /// Number of successful steps
    pub successful_steps: usize,
    /// Number of failed steps
    pub failed_steps: usize,
    /// Number of parallel chains executed
    pub parallel_chains: usize,
    /// Number of LLM steps
    pub llm_steps: usize,
    /// Number of tool steps
    pub tool_steps: usize,
    /// Total tokens used across all LLM steps
    pub total_tokens: usize,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: Option<usize>,
    /// Step-level telemetry if enabled
    pub step_telemetry: Vec<StepTelemetry>,
}

/// Resource usage metrics
#[derive(Debug, Default, Clone)]
pub struct ResourceMetrics {
    /// Number of LLM calls
    pub llm_calls: u32,
    /// Number of tool invocations
    pub tool_calls: u32,
    /// Number of parallel chains
    pub parallel_chains: u32,
    /// Peak memory usage (bytes)
    pub peak_memory: u64,
    /// Total compute time
    pub compute_time: Duration,
}

/// Telemetry data for a single step
#[derive(Debug, Clone)]
pub struct StepTelemetry {
    /// Step type
    pub step_type: StepType,
    /// Start time
    pub start_time: SystemTime,
    /// Duration
    pub duration: Option<Duration>,
    /// Success/failure
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Tokens used in the step
    pub tokens: Option<usize>,
}

/// Resource usage for a single step
#[derive(Debug, Default)]
struct ResourceUsage {
    /// Memory usage in bytes
    memory_bytes: u64,
    /// CPU time in milliseconds
    cpu_time_ms: u64,
}

impl ChainMetrics {
    /// Record a step execution
    fn record_step(&mut self, telemetry: StepTelemetry) {
        self.total_steps += 1;
        if telemetry.success {
            self.successful_steps += 1;
        } else {
            self.failed_steps += 1;
        }

        match telemetry.step_type {
            StepType::LLM => {
                self.llm_steps += 1;
                if let Some(tokens) = telemetry.tokens {
                    self.total_tokens += tokens;
                }
            }
            StepType::Tool => {
                self.tool_steps += 1;
            }
            StepType::Parallel => {
                self.parallel_chains += 1;
            }
        }

        if let Some(mem) = get_current_memory_usage() {
            if self.peak_memory_bytes.map_or(true, |peak| mem > peak) {
                self.peak_memory_bytes = Some(mem);
            }
        }

        self.step_telemetry.push(telemetry);
    }
}

#[cfg(target_os = "linux")]
fn get_current_memory_usage() -> Option<usize> {
    use std::fs::File;
    use std::io::Read;

    let mut status = String::new();
    File::open("/proc/self/status")
        .and_then(|mut f| f.read_to_string(&mut status))
        .ok()?;

    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            return line
                .split_whitespace()
                .nth(1)
                .and_then(|kb| kb.parse::<usize>().ok())
                .map(|kb| kb * 1024);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_current_memory_usage() -> Option<usize> {
    None
}

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

/// Configuration for a chain
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Default timeout for each step
    pub default_step_timeout: Duration,
    /// Total timeout for the entire chain
    pub total_timeout: Option<Duration>,
    /// Maximum number of parallel chains
    pub max_parallel_chains: usize,
    /// Whether to fail fast on parallel chain errors
    pub fail_fast: bool,
    /// Whether to collect metrics and telemetry
    pub collect_metrics: bool,
    /// Whether to include resource usage metrics
    pub collect_resource_metrics: bool,
    /// Whether to include step-level telemetry
    pub collect_step_telemetry: bool,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            default_step_timeout: Duration::from_secs(30),
            total_timeout: None,
            max_parallel_chains: 10,
            fail_fast: true,
            collect_metrics: true,
            collect_resource_metrics: true,
            collect_step_telemetry: true,
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
    metrics: Arc<Mutex<ChainMetrics>>,
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
            metrics: Arc::new(Mutex::new(ChainMetrics::default())),
        }
    }

    /// Get the current metrics
    pub fn metrics(&self) -> ChainMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Record telemetry for a step
    fn record_telemetry(&self, telemetry: StepTelemetry) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_step(telemetry);
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
        let chain_span = info_span!(
            "chain_execute",
            chain_len = self.steps.len(),
            collect_metrics = self.config.collect_metrics
        );
        let _enter = chain_span.enter();

        info!("Starting chain execution");
        let start_time = SystemTime::now();
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
                        let step_start = SystemTime::now();
                        let span = info_span!("llm_step",
                            step_idx,
                            model = model.name(),
                            timeout = ?timeout
                        );
                        let _enter = span.enter();

                        let timeout = timeout.unwrap_or(self.config.default_step_timeout);
                        debug!("Executing LLM step with timeout {:?}", timeout);

                        let result = match timeout_wrapper(
                            timeout,
                            model.generate(prompt, current_input.clone()),
                            "llm",
                        )
                        .instrument(span.clone())
                        .await
                        {
                            Ok(r) => {
                                if self.config.collect_metrics {
                                    self.record_telemetry(StepTelemetry {
                                        step_type: StepType::LLM,
                                        start_time: step_start,
                                        duration: Some(step_start.elapsed()),
                                        success: true,
                                        error: None,
                                        resource_usage: ResourceUsage::default(),
                                        tokens: None,
                                    });
                                }
                                Ok(r)
                            }
                            Err(e) => {
                                if self.config.collect_metrics {
                                    self.record_telemetry(StepTelemetry {
                                        step_type: StepType::LLM,
                                        start_time: step_start,
                                        duration: Some(step_start.elapsed()),
                                        success: false,
                                        error: Some(e.to_string()),
                                        resource_usage: ResourceUsage::default(),
                                        tokens: None,
                                    });
                                }
                                Err(e)
                            }
                        }?;

                        current_input = result;
                        info!("LLM step completed successfully");
                    }
                    ChainStep::Tool { tool, timeout } => {
                        let step_start = SystemTime::now();
                        let span = info_span!("tool_step",
                            step_idx,
                            tool = tool.spec().name,
                            timeout = ?timeout
                        );
                        let _enter = span.enter();

                        let timeout = timeout.unwrap_or(self.config.default_step_timeout);
                        debug!("Executing tool step with timeout {:?}", timeout);

                        let result = match timeout_wrapper(
                            timeout,
                            tool.invoke(current_input.clone()),
                            "tool",
                        )
                        .instrument(span.clone())
                        .await
                        {
                            Ok(r) => {
                                if self.config.collect_metrics {
                                    self.record_telemetry(StepTelemetry {
                                        step_type: StepType::Tool,
                                        start_time: step_start,
                                        duration: Some(step_start.elapsed()),
                                        success: true,
                                        error: None,
                                        resource_usage: ResourceUsage::default(),
                                        tokens: None,
                                    });
                                }
                                Ok(r)
                            }
                            Err(e) => {
                                if self.config.collect_metrics {
                                    self.record_telemetry(StepTelemetry {
                                        step_type: StepType::Tool,
                                        start_time: step_start,
                                        duration: Some(step_start.elapsed()),
                                        success: false,
                                        error: Some(e.to_string()),
                                        resource_usage: ResourceUsage::default(),
                                        tokens: None,
                                    });
                                }
                                Err(e)
                            }
                        }?;

                        current_input = result;
                        info!("Tool step completed successfully");
                    }
                    ChainStep::Parallel(chains) => {
                        let step_start = SystemTime::now();
                        let span = info_span!(
                            "parallel_step",
                            step_idx,
                            chain_count = chains.len(),
                            max_chains = self.config.max_parallel_chains
                        );
                        let _enter = span.enter();

                        if chains.len() > self.config.max_parallel_chains {
                            let err = ChainError::Other(anyhow::anyhow!(
                                "Too many parallel chains: {} (max: {})",
                                chains.len(),
                                self.config.max_parallel_chains
                            ));

                            if self.config.collect_metrics {
                                self.record_telemetry(StepTelemetry {
                                    step_type: StepType::Parallel,
                                    start_time: step_start,
                                    duration: Some(step_start.elapsed()),
                                    success: false,
                                    error: Some(err.to_string()),
                                    resource_usage: ResourceUsage::default(),
                                    tokens: None,
                                });
                            }

                            return Err(err);
                        }

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
                            if cancel_rx.try_recv().is_ok() {
                                for chain in chains {
                                    chain.cancel();
                                }
                                info!("Parallel chains cancelled");
                                self.cleanup_resources().await?;

                                if self.config.collect_metrics {
                                    self.record_telemetry(StepTelemetry {
                                        step_type: StepType::Parallel,
                                        start_time: step_start,
                                        duration: Some(step_start.elapsed()),
                                        success: false,
                                        error: Some("Cancelled".into()),
                                        resource_usage: ResourceUsage::default(),
                                        tokens: None,
                                    });
                                }

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
                                        for chain in chains {
                                            chain.cancel();
                                        }
                                        error!("Some parallel chains failed");
                                        self.cleanup_resources().await?;

                                        if self.config.collect_metrics {
                                            self.record_telemetry(StepTelemetry {
                                                step_type: StepType::Parallel,
                                                start_time: step_start,
                                                duration: Some(step_start.elapsed()),
                                                success: false,
                                                error: Some("Chain failed".into()),
                                                resource_usage: ResourceUsage::default(),
                                                tokens: None,
                                            });
                                        }

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

                            if self.config.collect_metrics {
                                self.record_telemetry(StepTelemetry {
                                    step_type: StepType::Parallel,
                                    start_time: step_start,
                                    duration: Some(step_start.elapsed()),
                                    success: false,
                                    error: Some("Multiple chains failed".into()),
                                    resource_usage: ResourceUsage::default(),
                                    tokens: None,
                                });
                            }

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

                        if self.config.collect_metrics {
                            self.record_telemetry(StepTelemetry {
                                step_type: StepType::Parallel,
                                start_time: step_start,
                                duration: Some(step_start.elapsed()),
                                success: true,
                                error: None,
                                resource_usage: ResourceUsage::default(),
                                tokens: None,
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

        // Update total duration in metrics
        if self.config.collect_metrics {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.total_duration = start_time.elapsed();
            }
        }

        // Clean up resources regardless of success/failure
        self.cleanup_resources().await?;

        result
    }
}

impl<I, O> Drop for Chain<I, O> {
    fn drop(&mut self) {
        if self.config.collect_metrics {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    LLM,
    Tool,
    Parallel,
}

impl Default for ChainMetrics {
    fn default() -> Self {
        Self {
            total_steps: 0,
            successful_steps: 0,
            failed_steps: 0,
            llm_steps: 0,
            tool_steps: 0,
            parallel_chains: 0,
            total_tokens: 0,
            peak_memory_bytes: None,
            step_telemetry: Vec::new(),
        }
    }
}
