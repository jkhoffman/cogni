//! Chain execution for the Cogni framework.

use crate::{
    error::{LlmError, ToolConfigError, ToolError},
    traits::{
        llm::{GenerateOptions, LanguageModel},
        prompt::PromptTemplate,
        tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
    },
};

use anyhow::Result;
use async_trait::async_trait;
use futures::{
    stream::{self, FuturesUnordered, Stream},
    StreamExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::{self, Debug, Display},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};
use tracing::{debug, error, info, info_span, warn, Instrument};

/// Type alias for a language model with specific input and output types
pub type LanguageModelArc<I, O> = Arc<
    dyn LanguageModel<
            Prompt = I,
            Response = O,
            TokenStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send + 'static>>,
        > + Send
        + Sync,
>;

/// Type alias for tool with specific input and output types
pub type ToolArc<I, O> = Arc<dyn Tool<Input = I, Output = O, Config = ()> + Send + Sync>;

/// Placeholder for NoopLanguageModel
#[derive(Debug, Default, Clone)]
struct NoopLanguageModel;

#[async_trait]
impl Tool for NoopLanguageModel {
    type Input = String;
    type Output = String;
    type Config = ();

    fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError> {
        Ok(Self)
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Stateless, ToolCapability::ThreadSafe]
    }

    async fn invoke(&self, _input: Self::Input) -> Result<Self::Output, ToolError> {
        Ok(String::new())
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "noop".into(),
            description: "A no-op language model".into(),
            input_schema: serde_json::json!({"type": "string"}),
            output_schema: serde_json::json!({"type": "string"}),
            examples: vec![],
        }
    }
}

#[async_trait]
impl LanguageModel for NoopLanguageModel {
    type Prompt = String;
    type Response = String;
    type TokenStream = stream::Empty<Result<String, LlmError>>;

    async fn generate(
        &self,
        _prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Self::Response, LlmError> {
        Ok(String::new())
    }

    async fn stream_generate(
        &self,
        _prompt: Self::Prompt,
        _opts: GenerateOptions,
    ) -> Result<Pin<Box<Self::TokenStream>>, LlmError> {
        Ok(Box::pin(stream::empty::<Result<String, LlmError>>()))
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

/// Metrics for chain execution
#[derive(Debug, Clone, PartialEq, Default)]
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
#[derive(Debug, Default, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResourceUsage {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU time in seconds
    pub cpu_seconds: f64,
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
            if self.peak_memory_bytes.is_none_or(|peak| mem > peak) {
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
        step_type: StepType,
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
    #[error("LLM error")]
    LlmError(#[from] LlmError),
    #[error("Tool error")]
    ToolError(#[from] ToolError),
}

/// Configuration for a chain
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone)]
pub struct ResourceHandle {
    /// Resource ID
    pub id: String,
    /// Resource type
    pub resource_type: String,
    /// Creation time
    pub created_at: SystemTime,
    /// Sender for cleanup signals
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
pub enum ChainStep<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    Llm {
        model: LanguageModelArc<I, O>,
        _prompt: Arc<PromptTemplate>,
        timeout: Duration,
    },
    Tool {
        tool: ToolArc<I, O>,
        timeout: Duration,
    },
    Parallel(Vec<Arc<Chain<I, O>>>),
}

impl<I, O> fmt::Debug for ChainStep<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainStep::Llm {
                model,
                _prompt,
                timeout,
            } => f
                .debug_struct("Llm")
                .field("model", &model.name())
                .field("prompt", &**_prompt)
                .field("timeout", timeout)
                .finish(),
            ChainStep::Tool { tool, timeout } => f
                .debug_struct("Tool")
                .field("tool", &tool.spec().name)
                .field("timeout", timeout)
                .finish(),
            ChainStep::Parallel(chains) => f.debug_tuple("Parallel").field(chains).finish(),
        }
    }
}

/// A chain of steps that can be executed sequentially or in parallel
pub struct Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    pub tools: Vec<ToolArc<I, O>>,
    pub parallel_chains: Vec<Arc<Chain<I, O>>>,
    pub resources: Vec<ResourceHandle>,
    pub config: ChainConfig,
    pub steps: Vec<ChainStep<I, O>>,
    metrics: Arc<Mutex<ChainMetrics>>,
    telemetry: Arc<Mutex<Vec<StepTelemetry>>>,
    current_step: Mutex<Option<StepType>>,
    pub error: Option<ChainError>,
    cancel_tx: broadcast::Sender<()>,
    cleanup_tx: mpsc::Sender<String>,
    cleanup_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    active_resources: Arc<tokio::sync::Mutex<Vec<ResourceHandle>>>,
}

impl<I, O> fmt::Debug for Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool_names: Vec<_> = self.tools.iter().map(|t| t.spec().name.clone()).collect();
        let current_step_val = self.current_step.lock().unwrap();
        let metrics_val = self.metrics.lock().unwrap();
        let telemetry_val = self.telemetry.lock().unwrap();

        f.debug_struct("Chain")
            .field("tools", &tool_names)
            .field("parallel_chains", &self.parallel_chains)
            .field("resources", &self.resources)
            .field("config", &self.config)
            .field("steps", &self.steps)
            .field("metrics", &*metrics_val)
            .field("telemetry", &*telemetry_val)
            .field("current_step", &*current_step_val)
            .field("error", &self.error)
            .finish()
    }
}

impl<I, O> Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    /// Create a new chain with default configuration
    pub fn new() -> Self {
        let (cancel_tx, _) = broadcast::channel(1);
        let (cleanup_tx, cleanup_rx) = mpsc::channel(100);

        Self {
            tools: Vec::new(),
            parallel_chains: Vec::new(),
            resources: Vec::new(),
            config: ChainConfig::default(),
            steps: Vec::new(),
            metrics: Arc::new(Mutex::new(ChainMetrics::default())),
            telemetry: Arc::new(Mutex::new(Vec::new())),
            current_step: Mutex::new(None),
            error: None,
            cancel_tx,
            cleanup_tx: cleanup_tx.clone(),
            cleanup_rx: Arc::new(Mutex::new(cleanup_rx)),
            active_resources: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    /// Get the current metrics
    pub fn metrics(&self) -> ChainMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Record telemetry for a step
    fn record_telemetry(&self, telemetry: StepTelemetry) {
        if self.config.collect_metrics {
            let mut metrics_guard = self.metrics.lock().unwrap();
            metrics_guard.record_step(telemetry.clone());
            if self.config.collect_step_telemetry {
                self.telemetry.lock().unwrap().push(telemetry);
            }
        }
    }

    /// Track a new resource
    pub async fn track_resource(&mut self, resource_type: &str, id: String) -> ResourceHandle {
        let handle = ResourceHandle {
            resource_type: resource_type.to_string(),
            id,
            created_at: SystemTime::now(),
            cleanup_tx: self.cleanup_tx.clone(),
        };

        let mut resources = self.active_resources.lock().await;
        resources.push(handle.clone());

        handle
    }

    /// Clean up resources
    pub async fn cleanup_resources(&mut self) {
        let mut resources_to_remove = Vec::new();

        let resources = self.active_resources.lock().await;
        for resource in resources.iter() {
            if let Ok(duration) = SystemTime::now().duration_since(resource.created_at) {
                if duration > Duration::from_secs(3600) {
                    resources_to_remove.push(resource.id.clone());
                }
            }
        }

        drop(resources);

        for id in resources_to_remove {
            let mut resources_guard = self.active_resources.lock().await;
            resources_guard.retain(|r| r.id != id);
            drop(resources_guard);
            let _ = self.cleanup_tx.send(id).await;
        }
    }

    /// Add an LLM step to the chain
    pub async fn add_llm<M, P>(mut self, model: M, prompt: P, timeout: Option<Duration>) -> Self
    where
        M: LanguageModel<
                Prompt = I,
                Response = O,
                TokenStream = Pin<
                    Box<dyn Stream<Item = Result<String, LlmError>> + Send + 'static>,
                >,
            > + Send
            + Sync
            + 'static,
        P: Into<PromptTemplate>,
    {
        let model_arc = Arc::new(model);
        let prompt_arc = Arc::new(prompt.into());

        let _llm_handle = self
            .track_resource("llm", format!("llm_{}", model_arc.name()))
            .await;

        self.steps.push(ChainStep::Llm {
            model: model_arc,
            _prompt: prompt_arc,
            timeout: timeout.unwrap_or(self.config.default_step_timeout),
        });
        self
    }

    /// Add a tool step to the chain
    pub async fn add_tool<ToolImpl>(mut self, tool: ToolImpl, timeout: Option<Duration>) -> Self
    where
        ToolImpl: Tool<Input = I, Output = O, Config = ()> + Send + Sync + 'static,
    {
        let tool_arc = Arc::new(tool);

        let _tool_handle = self
            .track_resource("tool", format!("tool_{}", tool_arc.spec().name))
            .await;

        self.steps.push(ChainStep::Tool {
            tool: tool_arc,
            timeout: timeout.unwrap_or(self.config.default_step_timeout),
        });
        self
    }

    /// Add parallel chains to execute
    pub async fn add_parallel(mut self, chains: Vec<Chain<I, O>>) -> Self
    where
        I: 'static,
        O: 'static,
    {
        let mut chains_arc: Vec<Arc<Chain<I, O>>> = Vec::new();
        for c in chains.into_iter() {
            let mut chain = c;
            // Share configuration, cancellation, and cleanup mechanisms
            chain.config = self.config.clone();
            chain.cancel_tx = self.cancel_tx.clone();
            chain.cleanup_tx = self.cleanup_tx.clone();
            // Resources of sub-chains are managed by the sub-chain itself
            chains_arc.push(Arc::new(chain));
        }
        self.steps.push(ChainStep::Parallel(chains_arc));
        self
    }

    /// Cancel the chain execution
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(());
    }

    /// Execute the chain with the given input
    pub async fn execute(&self, input: I) -> Result<O, ChainError>
    where
        I: From<O>,
    {
        let span = info_span!("chain_execute");
        let _enter = span.enter();

        let mut cancel_rx = self.cancel_tx.subscribe();

        let mut current_input = input;
        let mut final_output: Option<O> = None;

        for (step_index, step) in self.steps.iter().enumerate() {
            let step_span = info_span!("chain_step", index = step_index);
            let _step_enter = step_span.enter();

            // Need to handle ownership/cloning correctly if input is consumed
            let step_input = current_input.clone();

            match step {
                ChainStep::Llm {
                    model,
                    _prompt,
                    timeout,
                } => {
                    *self.current_step.lock().unwrap() = Some(StepType::LLM);
                    debug!(timeout_ms = timeout.as_millis(), "Executing LLM step");

                    let result = self
                        .execute_llm_step(
                            model.clone(),
                            _prompt.clone(),
                            step_input,
                            Some(*timeout),
                            &mut cancel_rx,
                        )
                        .await?;
                    current_input = I::from(result.clone());
                    final_output = Some(result);
                }
                ChainStep::Tool { tool, timeout } => {
                    *self.current_step.lock().unwrap() = Some(StepType::Tool);
                    debug!(timeout_ms = timeout.as_millis(), "Executing Tool step");
                    let result = self
                        .execute_tool_step(tool.clone(), step_input, Some(*timeout), &mut cancel_rx)
                        .await?;
                    current_input = I::from(result.clone());
                    final_output = Some(result);
                }
                ChainStep::Parallel(chains) => {
                    *self.current_step.lock().unwrap() = Some(StepType::Parallel);
                    debug!(count = chains.len(), "Executing Parallel step");
                    let results = self
                        .execute_parallel_step(chains.clone(), step_input, &mut cancel_rx)
                        .await?;

                    if let Some(first_result) = results.into_iter().next() {
                        final_output = Some(first_result.clone());
                        current_input = I::from(first_result);
                    } else {
                        warn!("Parallel step finished with no successful results.");
                        return Err(ChainError::Other(anyhow::anyhow!(
                            "No successful results from parallel step"
                        )));
                    }
                }
            }

            if cancel_rx.try_recv().is_ok() {
                warn!("Chain execution cancelled");
                return Err(ChainError::Cancelled);
            }
        }

        *self.current_step.lock().unwrap() = None;
        final_output.ok_or_else(|| {
            error!("Chain finished execution but produced no final output");
            ChainError::Other(anyhow::anyhow!(
                "Chain completed without producing an output"
            ))
        })
    }

    async fn execute_llm_step(
        &self,
        model: LanguageModelArc<I, O>,
        _prompt: Arc<PromptTemplate>,
        input: I,
        timeout_duration: Option<Duration>,
        cancel_rx: &mut broadcast::Receiver<()>,
    ) -> Result<O, ChainError> {
        let start_time = SystemTime::now();
        let effective_timeout = timeout_duration.unwrap_or(self.config.default_step_timeout);
        let generate_future = model.generate(
            input,
            GenerateOptions {
                timeout: Some(effective_timeout),
                max_tokens: None,
                temperature: None,
            },
        );

        tokio::select! {
            result = timeout(effective_timeout, generate_future) => {
                let execution_result = match result {
                    Ok(Ok(output)) => Ok(output),
                    Ok(Err(e)) => Err(ChainError::LlmError(e)),
                    Err(_) => Err(ChainError::Timeout { duration: effective_timeout, step_type: StepType::LLM }),
                };

                let duration = start_time.elapsed().ok();
                let success = execution_result.is_ok();
                let error_msg = execution_result.as_ref().err().map(|e| e.to_string());

                self.record_telemetry(StepTelemetry {
                    step_type: StepType::LLM,
                    start_time,
                    duration,
                    success,
                    error: error_msg,
                    resource_usage: ResourceUsage::default(),
                    tokens: None,
                });

                execution_result
            },
            _ = cancel_rx.recv() => {
                warn!("LLM step cancelled");
                Err(ChainError::Cancelled)
            }
        }
    }

    async fn execute_tool_step(
        &self,
        tool: ToolArc<I, O>,
        input: I,
        timeout_duration: Option<Duration>,
        cancel_rx: &mut broadcast::Receiver<()>,
    ) -> Result<O, ChainError> {
        let start_time = SystemTime::now();
        let effective_timeout = timeout_duration.unwrap_or(self.config.default_step_timeout);
        let invoke_future = tool.invoke(input);

        tokio::select! {
            result = timeout(effective_timeout, invoke_future) => {
                let execution_result = match result {
                    Ok(Ok(output)) => Ok(output),
                    Ok(Err(e)) => Err(ChainError::ToolError(e)),
                    Err(_) => Err(ChainError::Timeout { duration: effective_timeout, step_type: StepType::Tool }),
                };

                let duration = start_time.elapsed().ok();
                let success = execution_result.is_ok();
                let error_msg = execution_result.as_ref().err().map(|e| e.to_string());

                self.record_telemetry(StepTelemetry {
                    step_type: StepType::Tool,
                    start_time,
                    duration,
                    success,
                    error: error_msg,
                    resource_usage: ResourceUsage::default(),
                    tokens: None,
                });

                execution_result
            },
            _ = cancel_rx.recv() => {
                warn!("Tool step cancelled");
                Err(ChainError::Cancelled)
            }
        }
    }

    async fn execute_parallel_step(
        &self,
        chains: Vec<Arc<Chain<I, O>>>,
        input: I,
        cancel_rx: &mut broadcast::Receiver<()>,
    ) -> Result<Vec<O>, ChainError>
    where
        I: From<O>,
    {
        let start_time = SystemTime::now();
        let mut futures = FuturesUnordered::new();
        let chain_results: Arc<Mutex<Vec<Result<O, ChainError>>>> =
            Arc::new(Mutex::new(Vec::new()));

        for chain_arc in chains.into_iter().take(self.config.max_parallel_chains) {
            let chain_instance = chain_arc.clone();
            let input_clone = input.clone();
            let results_clone = Arc::clone(&chain_results);
            let _chain_cancel_rx = self.cancel_tx.subscribe();

            futures.push(
                async move {
                    let result = chain_instance.execute(input_clone).await;
                    results_clone.lock().unwrap().push(result);
                }
                .instrument(info_span!("parallel_sub_chain")),
            );
        }

        loop {
            tokio::select! {
                _ = futures.next() => {
                    if futures.is_empty() {
                        debug!("All parallel sub-chains finished");
                        break;
                    }
                },
                _ = cancel_rx.recv() => {
                    warn!("Parallel step cancelled by parent");
                    return Err(ChainError::Cancelled);
                }
            }
        }

        let final_results = Arc::try_unwrap(chain_results)
            .expect("Mutex should not be locked elsewhere")
            .into_inner()
            .expect("Mutex lock failed");

        let final_results_was_not_empty = !final_results.is_empty();

        let duration = start_time.elapsed().ok();
        let errors: Vec<String> = final_results
            .iter()
            .filter_map(|r| r.as_ref().err().map(|e| e.to_string()))
            .collect();
        let successful_results: Vec<O> = final_results.into_iter().filter_map(Result::ok).collect();

        let overall_success = !successful_results.is_empty();
        let error_msg = if errors.is_empty() {
            None
        } else {
            Some(errors.join(", "))
        };

        self.record_telemetry(StepTelemetry {
            step_type: StepType::Parallel,
            start_time,
            duration,
            success: overall_success,
            error: error_msg,
            resource_usage: ResourceUsage::default(),
            tokens: None,
        });

        if successful_results.is_empty() && final_results_was_not_empty {
            Err(ChainError::ParallelError {
                message: format!("All parallel chains failed: {}", errors.join("; ")),
                successful_results: Vec::new(),
            })
        } else {
            Ok(successful_results)
        }
    }
}

impl<I, O> Drop for Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    fn drop(&mut self) {
        info!("Dropping chain, triggering resource cleanup");
        self.cancel();

        if let Ok(mut cleanup_rx_guard) = self.cleanup_rx.lock() {
            while let Ok(resource_id) = cleanup_rx_guard.try_recv() {
                debug!(resource_id = %resource_id, "Cleanup signal received on drop");
            }
        }

        // Also clean up resources tracked directly by this chain instance
        // This requires iterating `active_resources` which is async, cannot do in sync drop.
        // Resource cleanup primarily relies on the ResourceHandle's Drop impl sending
        // to the cleanup channel.
    }
}

impl<I, O> Default for Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Type of step executed in a chain.
#[derive(Debug, Clone, PartialEq)]
pub enum StepType {
    LLM,
    Tool,
    Parallel,
}

// Manual Display impl for StepType
impl Display for StepType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepType::LLM => write!(f, "LLM Step"),
            StepType::Tool => write!(f, "Tool Step"),
            StepType::Parallel => write!(f, "Parallel Step"),
        }
    }
}

impl ToolConfig for () {
    fn validate(&self) -> Result<(), ToolConfigError> {
        Ok(())
    }
}

impl<I, O> Chain<I, O>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
    O: Clone + Send + Sync + Serialize + DeserializeOwned + Debug + 'static,
{
    pub fn with_config(mut self, config: ChainConfig) -> Self {
        self.config = config;
        self
    }
}

mod tests {
    use super::*;
    use crate::error::ToolError;
    use crate::traits::tool::{Tool, ToolCapability, ToolSpec};
    use async_trait::async_trait;

    #[derive(Clone, Debug)]
    struct MockTool {
        name: String,
        invocations: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Tool for MockTool {
        type Input = String;
        type Output = String;
        type Config = ();

        fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError> {
            Ok(Self {
                name: "mock".to_string(),
                invocations: Arc::new(Mutex::new(Vec::new())),
            })
        }

        async fn initialize(&mut self) -> Result<(), ToolError> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), ToolError> {
            Ok(())
        }

        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability::Stateless, ToolCapability::ThreadSafe]
        }

        async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
            self.invocations.lock().unwrap().push(input.clone());
            Ok(format!("Processed: {}", input))
        }

        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: self.name.clone(),
                description: "A mock tool for testing".into(),
                input_schema: serde_json::json!({"type": "string"}),
                output_schema: serde_json::json!({"type": "string"}),
                examples: vec![],
            }
        }
    }

    #[allow(dead_code)]
    fn create_test_chain() -> Chain<String, String> {
        Chain::new()
    }

    #[allow(dead_code)]
    async fn test_resource_cleanup() -> anyhow::Result<()> {
        let mut chain: Chain<String, String> = create_test_chain();
        let _handle = chain.track_resource("test", "res1".to_string()).await;

        assert_eq!(chain.active_resources.lock().await.len(), 1);

        let tool = MockTool {
            name: "tool1".into(),
            invocations: Arc::new(Mutex::new(Vec::new())),
        };
        let chain_with_tool: Chain<String, String> = Chain::new()
            .add_tool(tool.clone(), Some(Duration::from_secs(30)))
            .await;

        assert_eq!(chain_with_tool.steps.len(), 1);

        Ok(())
    }
}
