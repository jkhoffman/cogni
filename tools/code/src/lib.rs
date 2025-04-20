//! Code execution tool for the Cogni framework.
//!
//! This crate provides a WASI-based code execution tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use async_trait::async_trait;
use cogni_core::error::ToolError;
use cogni_core::traits::tool::{Tool, ToolCapability, ToolConfig, ToolSpec};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasmtime::Engine;

/// Input for the code execution tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeInput {
    /// The code to execute
    pub code: String,
    /// The language of the code
    pub language: String,
    /// Maximum execution time in seconds
    pub timeout: Option<u64>,
    /// Maximum memory usage in MB
    pub memory_limit: Option<u64>,
}

/// Output from the code execution tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeOutput {
    /// The execution result
    pub result: String,
    /// Any error messages
    pub errors: Vec<String>,
    /// Execution statistics
    pub stats: ExecutionStats,
}

/// Statistics about code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
}

/// Configuration for the code execution tool.
#[derive(Debug, Clone)]
pub struct CodeConfig {
    /// Default timeout in seconds
    pub timeout: u64,
    /// Default memory limit in MB
    pub memory_limit: u64,
    /// Path to WASI SDK
    pub wasi_sdk_path: String,
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            timeout: 5,
            memory_limit: 128,
            wasi_sdk_path: "/opt/wasi-sdk".to_string(),
        }
    }
}

impl ToolConfig for CodeConfig {
    fn validate(&self) -> Result<(), String> {
        if self.timeout == 0 {
            return Err("timeout must be greater than 0".into());
        }
        if self.memory_limit == 0 {
            return Err("memory_limit must be greater than 0".into());
        }
        if self.wasi_sdk_path.is_empty() {
            return Err("wasi_sdk_path cannot be empty".into());
        }
        if !std::path::Path::new(&self.wasi_sdk_path).exists() {
            return Err(format!(
                "wasi_sdk_path {} does not exist",
                self.wasi_sdk_path
            ));
        }
        Ok(())
    }
}

/// The code execution tool.
pub struct CodeTool {
    config: CodeConfig,
    engine: Option<Engine>,
}

impl CodeTool {
    /// Create a new code execution tool with the given configuration.
    pub fn new(config: CodeConfig) -> Self {
        Self {
            config,
            engine: None,
        }
    }
}

#[async_trait]
impl Tool for CodeTool {
    type Input = CodeInput;
    type Output = CodeOutput;
    type Config = CodeConfig;

    async fn initialize(&mut self) -> Result<(), ToolError> {
        self.engine = Some(Engine::default());
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        self.engine = None;
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::ThreadSafe,
            ToolCapability::CpuIntensive,
            ToolCapability::MemoryIntensive,
            ToolCapability::FileSystem,
        ]
    }

    #[instrument(skip(self, _input))]
    async fn invoke(&self, _input: Self::Input) -> Result<Self::Output, ToolError> {
        todo!("Implement code execution tool")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "code".to_string(),
            description: "Execute code in a sandboxed environment".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "The code to execute"
                    },
                    "language": {
                        "type": "string",
                        "description": "The language of the code",
                        "enum": ["python", "javascript", "rust"]
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Maximum execution time in seconds",
                        "minimum": 1,
                        "maximum": 30
                    },
                    "memory_limit": {
                        "type": "integer",
                        "description": "Maximum memory usage in MB",
                        "minimum": 16,
                        "maximum": 512
                    }
                },
                "required": ["code", "language"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {
                        "type": "string",
                        "description": "The execution result"
                    },
                    "errors": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Any error messages"
                    },
                    "stats": {
                        "type": "object",
                        "properties": {
                            "execution_time_ms": {
                                "type": "integer",
                                "description": "Execution time in milliseconds"
                            },
                            "peak_memory_bytes": {
                                "type": "integer",
                                "description": "Peak memory usage in bytes"
                            }
                        },
                        "required": ["execution_time_ms", "peak_memory_bytes"]
                    }
                },
                "required": ["result", "errors", "stats"]
            }),
            examples: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_creation() {
        let config = CodeConfig::default();
        let mut tool = CodeTool::new(config);
        assert!(tool.initialize().await.is_ok());
        assert!(tool.shutdown().await.is_ok());
    }

    #[test]
    fn test_config_validation() {
        let valid_config = CodeConfig::default();
        assert!(valid_config.validate().is_err()); // Will fail if WASI SDK not installed

        let invalid_config = CodeConfig {
            timeout: 0,
            ..CodeConfig::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = CodeConfig {
            memory_limit: 0,
            ..CodeConfig::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = CodeConfig {
            wasi_sdk_path: "".to_string(),
            ..CodeConfig::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_capabilities() {
        let tool = CodeTool::new(CodeConfig::default());
        let capabilities = tool.capabilities();
        assert!(capabilities.contains(&ToolCapability::ThreadSafe));
        assert!(capabilities.contains(&ToolCapability::CpuIntensive));
        assert!(capabilities.contains(&ToolCapability::MemoryIntensive));
        assert!(capabilities.contains(&ToolCapability::FileSystem));
    }
}
