//! Tool validation utilities

use crate::error::{Result, ToolError};
use serde_json::{Value, Map};

/// Trait for validating tool arguments
pub trait ToolValidator {
    /// Validate arguments against a schema
    fn validate(&self, args: &Value, schema: &Value) -> Result<()>;
}

/// Default JSON Schema validator
pub struct JsonSchemaValidator;

impl JsonSchemaValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate a value against a JSON schema (simplified implementation)
    #[allow(clippy::only_used_in_recursion)]
    fn validate_value(&self, value: &Value, schema: &Value, path: &str) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        
        if let Some(schema_obj) = schema.as_object() {
            // Check type
            if let Some(expected_type) = schema_obj.get("type").and_then(|t| t.as_str()) {
                match (expected_type, value) {
                    ("string", Value::String(_)) => {},
                    ("number", Value::Number(_)) => {},
                    ("integer", Value::Number(n)) if n.is_i64() || n.is_u64() => {},
                    ("boolean", Value::Bool(_)) => {},
                    ("array", Value::Array(_)) => {},
                    ("object", Value::Object(_)) => {},
                    ("null", Value::Null) => {},
                    _ => {
                        errors.push(format!(
                            "{}: Expected type '{}', got '{}'",
                            path, expected_type, value_type_name(value)
                        ));
                    }
                }
            }
            
            // Check required properties for objects
            if value.is_object() {
                let obj = value.as_object().unwrap();
                
                // Check required fields
                if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
                    for req in required {
                        if let Some(field_name) = req.as_str() {
                            if !obj.contains_key(field_name) {
                                errors.push(format!("{}: Missing required field '{}'", path, field_name));
                            }
                        }
                    }
                }
                
                // Validate properties
                if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                    for (key, value) in obj {
                        if let Some(prop_schema) = properties.get(key) {
                            let prop_path = if path.is_empty() {
                                key.clone()
                            } else {
                                format!("{}.{}", path, key)
                            };
                            errors.extend(self.validate_value(value, prop_schema, &prop_path)?);
                        } else if schema_obj.get("additionalProperties") == Some(&Value::Bool(false)) {
                            errors.push(format!("{}: Unexpected property '{}'", path, key));
                        }
                    }
                }
            }
            
            // Check array items
            if let (Some(items_schema), Some(array)) = (schema_obj.get("items"), value.as_array()) {
                for (i, item) in array.iter().enumerate() {
                    let item_path = format!("{}[{}]", path, i);
                    errors.extend(self.validate_value(item, items_schema, &item_path)?);
                }
            }
            
            // Check minimum/maximum for numbers
            if let Some(n) = value.as_f64() {
                if let Some(min) = schema_obj.get("minimum").and_then(|v| v.as_f64()) {
                    if n < min {
                        errors.push(format!("{}: Value {} is less than minimum {}", path, n, min));
                    }
                }
                if let Some(max) = schema_obj.get("maximum").and_then(|v| v.as_f64()) {
                    if n > max {
                        errors.push(format!("{}: Value {} is greater than maximum {}", path, n, max));
                    }
                }
            }
            
            // Check string constraints
            if let Some(s) = value.as_str() {
                if let Some(min_length) = schema_obj.get("minLength").and_then(|v| v.as_u64()) {
                    if s.len() < min_length as usize {
                        errors.push(format!("{}: String length {} is less than minLength {}", path, s.len(), min_length));
                    }
                }
                if let Some(max_length) = schema_obj.get("maxLength").and_then(|v| v.as_u64()) {
                    if s.len() > max_length as usize {
                        errors.push(format!("{}: String length {} is greater than maxLength {}", path, s.len(), max_length));
                    }
                }
                if let Some(pattern) = schema_obj.get("pattern").and_then(|v| v.as_str()) {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(s) {
                            errors.push(format!("{}: String '{}' does not match pattern '{}'", path, s, pattern));
                        }
                    }
                }
            }
            
            // Check enum values
            if let Some(enum_values) = schema_obj.get("enum").and_then(|e| e.as_array()) {
                if !enum_values.contains(value) {
                    errors.push(format!("{}: Value must be one of {:?}", path, enum_values));
                }
            }
        }
        
        Ok(errors)
    }
}

impl Default for JsonSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolValidator for JsonSchemaValidator {
    fn validate(&self, args: &Value, schema: &Value) -> Result<()> {
        let errors = self.validate_value(args, schema, "")?;
        
        if !errors.is_empty() {
            return Err(ToolError::ValidationFailed {
                tool: "unknown".to_string(),
                errors,
            });
        }
        
        Ok(())
    }
}

/// Get the type name of a JSON value
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Validate tool arguments using a schema
pub fn validate_args(args: &Value, schema: &Value) -> Result<()> {
    let validator = JsonSchemaValidator::new();
    validator.validate(args, schema)
}

/// Helper to create a simple parameter schema
pub fn param_schema() -> ParameterSchemaBuilder {
    ParameterSchemaBuilder::new()
}

/// Builder for creating parameter schemas
#[derive(Default)]
pub struct ParameterSchemaBuilder {
    properties: Map<String, Value>,
    required: Vec<String>,
    additional_properties: Option<bool>,
}

impl ParameterSchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a string parameter
    pub fn string(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), serde_json::json!({
            "type": "string",
            "description": description.into()
        }));
        self
    }
    
    /// Add a required string parameter
    pub fn string_required(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), serde_json::json!({
            "type": "string",
            "description": description.into()
        }));
        self.required.push(name);
        self
    }
    
    /// Add a number parameter
    pub fn number(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), serde_json::json!({
            "type": "number",
            "description": description.into()
        }));
        self
    }
    
    /// Add a boolean parameter
    pub fn boolean(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), serde_json::json!({
            "type": "boolean",
            "description": description.into()
        }));
        self
    }
    
    /// Add an array parameter
    pub fn array(mut self, name: impl Into<String>, item_type: &str, description: impl Into<String>) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), serde_json::json!({
            "type": "array",
            "items": { "type": item_type },
            "description": description.into()
        }));
        self
    }
    
    /// Set whether additional properties are allowed
    pub fn additional_properties(mut self, allowed: bool) -> Self {
        self.additional_properties = Some(allowed);
        self
    }
    
    /// Build the schema
    pub fn build(self) -> Value {
        let mut schema = serde_json::json!({
            "type": "object",
            "properties": self.properties
        });
        
        if !self.required.is_empty() {
            schema["required"] = Value::Array(
                self.required.into_iter().map(Value::String).collect()
            );
        }
        
        if let Some(additional) = self.additional_properties {
            schema["additionalProperties"] = Value::Bool(additional);
        }
        
        schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_validation() {
        let validator = JsonSchemaValidator::new();
        
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number", "minimum": 0 }
            },
            "required": ["name"]
        });
        
        // Valid input
        let valid = json!({ "name": "Alice", "age": 30 });
        assert!(validator.validate(&valid, &schema).is_ok());
        
        // Missing required field
        let invalid = json!({ "age": 30 });
        assert!(validator.validate(&invalid, &schema).is_err());
        
        // Wrong type
        let invalid = json!({ "name": "Alice", "age": "thirty" });
        assert!(validator.validate(&invalid, &schema).is_err());
    }
    
    #[test]
    fn test_schema_builder() {
        let schema = param_schema()
            .string_required("query", "The search query")
            .number("limit", "Maximum results")
            .boolean("detailed", "Include detailed results")
            .build();
        
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"], json!(["query"]));
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["properties"]["detailed"].is_object());
    }
}