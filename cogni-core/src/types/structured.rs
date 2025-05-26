//! Types for structured output functionality

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Trait for types that can be used as structured output from LLMs.
///
/// Implementing this trait allows a type to be used with provider-specific
/// structured output features (like OpenAI's JSON mode or Anthropic's structured output).
pub trait StructuredOutput: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Returns the JSON Schema for this type.
    ///
    /// This schema is used by providers to ensure the model's output
    /// conforms to the expected structure.
    fn schema() -> Value;

    /// Returns example instances of this type.
    ///
    /// Examples can help improve model performance by showing
    /// the expected format and values.
    fn examples() -> Vec<Self> {
        vec![]
    }
}

/// Format specification for structured responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ResponseFormat {
    /// Request a response matching a specific JSON Schema.
    JsonSchema {
        /// The JSON Schema that the response must conform to.
        schema: Value,
        /// Whether to enforce strict schema validation (provider-specific).
        strict: bool,
    },
    /// Request any valid JSON object response.
    #[default]
    JsonObject,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        age: u32,
        active: bool,
    }

    impl StructuredOutput for TestStruct {
        fn schema() -> Value {
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "integer", "minimum": 0 },
                    "active": { "type": "boolean" }
                },
                "required": ["name", "age", "active"]
            })
        }

        fn examples() -> Vec<Self> {
            vec![
                TestStruct {
                    name: "Alice".to_string(),
                    age: 30,
                    active: true,
                },
                TestStruct {
                    name: "Bob".to_string(),
                    age: 25,
                    active: false,
                },
            ]
        }
    }

    #[test]
    fn test_response_format_default() {
        let format = ResponseFormat::default();
        assert_eq!(format, ResponseFormat::JsonObject);
    }

    #[test]
    fn test_response_format_json_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "field": { "type": "string" }
            }
        });

        let format = ResponseFormat::JsonSchema {
            schema: schema.clone(),
            strict: true,
        };

        match &format {
            ResponseFormat::JsonSchema { schema: s, strict } => {
                assert_eq!(s, &schema);
                assert_eq!(*strict, true);
            }
            _ => panic!("Expected JsonSchema variant"),
        }
    }

    #[test]
    fn test_response_format_serialization() {
        let format = ResponseFormat::JsonObject;
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ResponseFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(format, deserialized);

        let format = ResponseFormat::JsonSchema {
            schema: json!({"type": "object"}),
            strict: false,
        };
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ResponseFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(format, deserialized);
    }

    #[test]
    fn test_structured_output_trait() {
        let schema = TestStruct::schema();
        assert!(schema.is_object());
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "integer");
        assert_eq!(schema["properties"]["active"]["type"], "boolean");

        let examples = TestStruct::examples();
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].name, "Alice");
        assert_eq!(examples[1].name, "Bob");
    }

    #[test]
    fn test_structured_output_default_examples() {
        struct MinimalStruct;

        impl Serialize for MinimalStruct {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_unit()
            }
        }

        impl<'de> Deserialize<'de> for MinimalStruct {
            fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(MinimalStruct)
            }
        }

        impl StructuredOutput for MinimalStruct {
            fn schema() -> Value {
                json!({"type": "null"})
            }
        }

        let examples = MinimalStruct::examples();
        assert!(examples.is_empty());
    }

    #[test]
    fn test_response_format_clone() {
        let format = ResponseFormat::JsonSchema {
            schema: json!({"type": "string"}),
            strict: true,
        };
        let cloned = format.clone();
        assert_eq!(format, cloned);
    }

    #[test]
    fn test_response_format_debug() {
        let format = ResponseFormat::JsonObject;
        let debug_str = format!("{:?}", format);
        assert!(debug_str.contains("JsonObject"));

        let format = ResponseFormat::JsonSchema {
            schema: json!({"type": "number"}),
            strict: false,
        };
        let debug_str = format!("{:?}", format);
        assert!(debug_str.contains("JsonSchema"));
        assert!(debug_str.contains("number"));
    }
}
