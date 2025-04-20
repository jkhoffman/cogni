//! Math tool for the Cogni framework.
//!
//! This crate provides a tool for mathematical computations.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::Result;
use async_trait::async_trait;
use cogni_core::{
    error::ToolError,
    traits::tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
};
use log::warn;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Configuration for the math tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathConfig {
    /// Maximum matrix size for operations
    pub max_matrix_size: usize,
    /// Maximum number of iterations for convergence
    pub max_iterations: usize,
    /// Numerical precision threshold
    pub precision: f64,
}

impl Default for MathConfig {
    fn default() -> Self {
        Self {
            max_matrix_size: 100,
            max_iterations: 1000,
            precision: 1e-10,
        }
    }
}

impl ToolConfig for MathConfig {
    fn validate(&self) -> Result<(), String> {
        if self.max_matrix_size == 0 {
            return Err("max_matrix_size must be greater than 0".into());
        }
        if self.max_iterations == 0 {
            return Err("max_iterations must be greater than 0".into());
        }
        if self.precision <= 0.0 {
            return Err("precision must be greater than 0".into());
        }
        Ok(())
    }
}

/// Input for the math tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum MathInput {
    /// Basic arithmetic operation
    #[serde(rename = "arithmetic")]
    Arithmetic {
        /// The expression to evaluate
        expression: String,
    },
    /// Matrix operation
    #[serde(rename = "matrix")]
    Matrix {
        /// The operation to perform
        operation: MatrixOperation,
        /// The matrices to operate on
        matrices: Vec<Vec<Vec<f64>>>,
    },
    /// Statistical operation
    #[serde(rename = "statistics")]
    Statistics {
        /// The operation to perform
        operation: StatOperation,
        /// The data to analyze
        data: Vec<f64>,
    },
}

/// Matrix operations supported by the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatrixOperation {
    /// Matrix multiplication
    #[serde(rename = "multiply")]
    Multiply,
    /// Matrix inverse
    #[serde(rename = "inverse")]
    Inverse,
    /// Matrix determinant
    #[serde(rename = "determinant")]
    Determinant,
    /// Eigenvalue decomposition
    #[serde(rename = "eigenvalues")]
    Eigenvalues,
}

/// Statistical operations supported by the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatOperation {
    /// Mean of the data
    #[serde(rename = "mean")]
    Mean,
    /// Standard deviation
    #[serde(rename = "std_dev")]
    StdDev,
    /// Z-score of each data point
    #[serde(rename = "z_score")]
    ZScore,
    /// Normal distribution parameters
    #[serde(rename = "normal_fit")]
    NormalFit,
}

/// Output from the math tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "result")]
pub enum MathOutput {
    /// A single real number result
    #[serde(rename = "scalar")]
    Scalar(f64),
    /// A vector of real numbers
    #[serde(rename = "vector")]
    Vector(Vec<f64>),
    /// A matrix of real numbers
    #[serde(rename = "matrix")]
    Matrix(Vec<Vec<f64>>),
    /// A complex number result with real and imaginary parts
    #[serde(rename = "complex")]
    Complex {
        /// The real part of the complex number
        real: f64,
        /// The imaginary part of the complex number
        imag: f64,
    },
}

/// A number type that can be either real or complex.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// A real number value
    Real(f64),
    /// A complex number with real and imaginary components
    Complex {
        /// The real component of the complex number
        real: f64,
        /// The imaginary component of the complex number
        imag: f64,
    },
}

/// The math tool.
pub struct MathTool {
    _config: MathConfig,
}

impl Default for MathTool {
    fn default() -> Self {
        Self::new(MathConfig::default())
    }
}

impl MathTool {
    /// Create a new math tool with the given configuration.
    pub fn new(config: MathConfig) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl Tool for MathTool {
    type Input = MathInput;
    type Output = MathOutput;
    type Config = MathConfig;

    fn try_new(config: Self::Config) -> Result<Self, ToolError> {
        Ok(Self::new(config))
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        // No initialization needed for math tool
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        // No cleanup needed for math tool
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::Stateless,
            ToolCapability::ThreadSafe,
            ToolCapability::CpuIntensive,
        ]
    }

    #[instrument(skip_all)]
    async fn invoke(&self, _input: Self::Input) -> Result<Self::Output, ToolError> {
        todo!("Implement math operations")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "math".to_string(),
            description: "Perform mathematical computations".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["arithmetic", "matrix", "statistics"]
                    },
                    "params": {
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "expression": {
                                        "type": "string",
                                        "description": "Arithmetic expression to evaluate"
                                    }
                                },
                                "required": ["expression"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "operation": {
                                        "type": "string",
                                        "enum": ["multiply", "inverse", "determinant", "eigenvalues"]
                                    },
                                    "matrices": {
                                        "type": "array",
                                        "items": {
                                            "type": "array",
                                            "items": {
                                                "type": "array",
                                                "items": { "type": "number" }
                                            }
                                        }
                                    }
                                },
                                "required": ["operation", "matrices"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "operation": {
                                        "type": "string",
                                        "enum": ["mean", "std_dev", "z_score", "normal_fit"]
                                    },
                                    "data": {
                                        "type": "array",
                                        "items": { "type": "number" }
                                    }
                                },
                                "required": ["operation", "data"]
                            }
                        ]
                    }
                },
                "required": ["type", "params"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["scalar", "vector", "matrix", "complex"]
                    },
                    "result": {
                        "oneOf": [
                            { "type": "number" },
                            {
                                "type": "array",
                                "items": { "type": "number" }
                            },
                            {
                                "type": "array",
                                "items": {
                                    "type": "array",
                                    "items": { "type": "number" }
                                }
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "real": { "type": "number" },
                                    "imag": { "type": "number" }
                                },
                                "required": ["real", "imag"]
                            }
                        ]
                    }
                },
                "required": ["type", "result"]
            }),
            examples: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let config = MathConfig::default();
        let _tool = MathTool::new(config);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = MathConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = MathConfig {
            max_matrix_size: 0,
            ..MathConfig::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = MathConfig {
            max_iterations: 0,
            ..MathConfig::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = MathConfig {
            precision: 0.0,
            ..MathConfig::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_lifecycle() {
        let mut tool = MathTool::default();
        assert!(tool.initialize().await.is_ok());
        assert!(tool.shutdown().await.is_ok());
    }

    #[test]
    fn test_capabilities() {
        let tool = MathTool::default();
        let capabilities = tool.capabilities();
        assert!(capabilities.contains(&ToolCapability::Stateless));
        assert!(capabilities.contains(&ToolCapability::ThreadSafe));
        assert!(capabilities.contains(&ToolCapability::CpuIntensive));
    }
}
