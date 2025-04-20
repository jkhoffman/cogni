//! Tool validation framework for the Cogni tool registry.
//!
//! This module provides utilities for validating tool specifications,
//! inputs, and outputs against JSON schema.

use serde_json::Value;
use thiserror::Error;
use tracing::error;

/// Errors that can occur during tool validation.
#[derive(Error, Debug)]
pub enum ValidationError {
    /// The tool specification does not match the schema
    #[error("Tool specification validation failed: {0}")]
    SpecificationError(String),

    /// The tool input does not match the schema
    #[error("Tool input validation failed: {0}")]
    InputError(String),

    /// The tool output does not match the schema
    #[error("Tool output validation failed: {0}")]
    OutputError(String),

    /// The JSON schema is invalid
    #[error("Invalid JSON schema: {0}")]
    SchemaError(String),

    /// The JSON value is invalid
    #[error("Invalid JSON value: {0}")]
    JsonError(String),
}

/// A trait for types that can be validated against a JSON schema.
pub trait Validatable {
    /// Validate this value against a JSON schema.
    ///
    /// # Arguments
    /// * `schema` - The JSON schema to validate against
    ///
    /// # Returns
    /// Returns `Ok(())` if the value is valid, or an error if it's not.
    fn validate(&self, schema: &Value) -> Result<(), ValidationError>;
}

impl Validatable for Value {
    fn validate(&self, schema: &Value) -> Result<(), ValidationError> {
        // Create a JSON Schema validator
        let schema_compiled = jsonschema::JSONSchema::compile(schema)
            .map_err(|e| ValidationError::SchemaError(e.to_string()))?;

        // Validate the value against the schema - extract the result separately
        // to avoid the borrow checker issue
        let validation_result = schema_compiled.validate(self);

        match validation_result {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_messages = errors
                    .map(|e| format!("{} at {}", e, e.instance_path))
                    .collect::<Vec<_>>()
                    .join("; ");

                Err(ValidationError::JsonError(error_messages))
            }
        }
    }
}

/// A tool validator for validating tool specifications, inputs, and outputs.
#[derive(Debug, Clone)]
pub struct ToolValidator {
    /// The specification schema
    spec_schema: Value,
    /// The input schema
    input_schema: Value,
    /// The output schema
    output_schema: Value,
}

impl ToolValidator {
    /// Create a new tool validator.
    ///
    /// # Arguments
    /// * `spec_schema` - The schema for validating tool specifications
    /// * `input_schema` - The schema for validating tool inputs
    /// * `output_schema` - The schema for validating tool outputs
    ///
    /// # Returns
    /// Returns a new `ToolValidator` instance.
    pub fn new(spec_schema: Value, input_schema: Value, output_schema: Value) -> Self {
        Self {
            spec_schema,
            input_schema,
            output_schema,
        }
    }

    /// Create a new tool validator with default schemas.
    ///
    /// # Returns
    /// Returns a new `ToolValidator` instance with default schemas.
    pub fn default_schemas() -> Self {
        Self {
            spec_schema: serde_json::json!({
                "type": "object",
                "required": ["name", "description", "input_schema", "output_schema"],
                "properties": {
                    "name": {
                        "type": "string",
                        "minLength": 1
                    },
                    "description": {
                        "type": "string",
                        "minLength": 1
                    },
                    "input_schema": {
                        "type": "object"
                    },
                    "output_schema": {
                        "type": "object"
                    },
                    "examples": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["description", "input", "output"],
                            "properties": {
                                "description": { "type": "string" },
                                "input": {},
                                "output": {}
                            }
                        }
                    }
                }
            }),
            input_schema: serde_json::json!({}), // Empty schema matches anything
            output_schema: serde_json::json!({}), // Empty schema matches anything
        }
    }

    /// Validate a tool specification.
    ///
    /// # Arguments
    /// * `spec` - The tool specification to validate
    ///
    /// # Returns
    /// Returns `Ok(())` if the specification is valid, or an error if it's not.
    pub fn validate_spec(&self, spec: &Value) -> Result<(), ValidationError> {
        spec.validate(&self.spec_schema)
            .map_err(|err| ValidationError::SpecificationError(err.to_string()))
    }

    /// Validate a tool input.
    ///
    /// # Arguments
    /// * `input` - The tool input to validate
    /// * `input_schema` - The specific schema for this input (overrides the default)
    ///
    /// # Returns
    /// Returns `Ok(())` if the input is valid, or an error if it's not.
    pub fn validate_input(
        &self,
        input: &Value,
        input_schema: Option<&Value>,
    ) -> Result<(), ValidationError> {
        let schema = input_schema.unwrap_or(&self.input_schema);
        input
            .validate(schema)
            .map_err(|err| ValidationError::InputError(err.to_string()))
    }

    /// Validate a tool output.
    ///
    /// # Arguments
    /// * `output` - The tool output to validate
    /// * `output_schema` - The specific schema for this output (overrides the default)
    ///
    /// # Returns
    /// Returns `Ok(())` if the output is valid, or an error if it's not.
    pub fn validate_output(
        &self,
        output: &Value,
        output_schema: Option<&Value>,
    ) -> Result<(), ValidationError> {
        let schema = output_schema.unwrap_or(&self.output_schema);
        output
            .validate(schema)
            .map_err(|err| ValidationError::OutputError(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_value() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });

        let value = serde_json::json!({
            "name": "test"
        });

        assert!(value.validate(&schema).is_ok());
    }

    #[test]
    fn test_validate_invalid_value() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });

        let value = serde_json::json!({
            "name": 123  // Wrong type
        });

        assert!(value.validate(&schema).is_err());
    }

    #[test]
    fn test_tool_validator() {
        let validator = ToolValidator::default_schemas();

        // Valid spec
        let valid_spec = serde_json::json!({
            "name": "test-tool",
            "description": "A test tool",
            "input_schema": { "type": "string" },
            "output_schema": { "type": "string" },
            "examples": []
        });

        assert!(validator.validate_spec(&valid_spec).is_ok());

        // Invalid spec (missing description)
        let invalid_spec = serde_json::json!({
            "name": "test-tool",
            "input_schema": { "type": "string" },
            "output_schema": { "type": "string" }
        });

        assert!(validator.validate_spec(&invalid_spec).is_err());

        // Validate input
        let input_schema = serde_json::json!({ "type": "string" });
        let valid_input = serde_json::json!("hello");
        let invalid_input = serde_json::json!(123);

        assert!(validator
            .validate_input(&valid_input, Some(&input_schema))
            .is_ok());
        assert!(validator
            .validate_input(&invalid_input, Some(&input_schema))
            .is_err());

        // Validate output
        let output_schema = serde_json::json!({ "type": "object", "required": ["result"] });
        let valid_output = serde_json::json!({ "result": "success" });
        let invalid_output = serde_json::json!({ "error": "failure" });

        assert!(validator
            .validate_output(&valid_output, Some(&output_schema))
            .is_ok());
        assert!(validator
            .validate_output(&invalid_output, Some(&output_schema))
            .is_err());
    }
}
