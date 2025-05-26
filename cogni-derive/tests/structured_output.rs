use cogni_core::StructuredOutput;
use cogni_derive::StructuredOutput;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct ComplexStruct {
    id: u64,
    tags: Vec<String>,
    active: bool,
    score: f64,
    metadata: Option<String>,
}

#[test]
fn test_simple_struct_schema() {
    let schema = Person::schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"].is_object());
    assert_eq!(schema["properties"]["name"]["type"], "string");
    assert_eq!(schema["properties"]["age"]["type"], "integer");
    assert_eq!(schema["properties"]["age"]["minimum"], 0);
    assert_eq!(schema["properties"]["email"]["type"], "string");
    assert_eq!(schema["additionalProperties"], false);

    // Required fields should not include optional fields
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 2);
    assert!(required.contains(&json!("name")));
    assert!(required.contains(&json!("age")));
    assert!(!required.contains(&json!("email")));
}

#[test]
fn test_complex_struct_schema() {
    let schema = ComplexStruct::schema();

    assert_eq!(schema["type"], "object");

    // Check properties
    assert_eq!(schema["properties"]["id"]["type"], "integer");
    assert_eq!(schema["properties"]["id"]["minimum"], 0);
    assert_eq!(schema["properties"]["tags"]["type"], "array");
    assert_eq!(schema["properties"]["tags"]["items"]["type"], "string");
    assert_eq!(schema["properties"]["active"]["type"], "boolean");
    assert_eq!(schema["properties"]["score"]["type"], "number");
    assert_eq!(schema["properties"]["metadata"]["type"], "string");

    // Check required fields
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 4);
    assert!(required.contains(&json!("id")));
    assert!(required.contains(&json!("tags")));
    assert!(required.contains(&json!("active")));
    assert!(required.contains(&json!("score")));
    assert!(!required.contains(&json!("metadata")));
}

#[test]
fn test_nested_option() {
    #[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
    struct WithNestedOption {
        required: String,
        optional: Option<Vec<String>>,
    }

    let schema = WithNestedOption::schema();

    // Optional field should still have the inner type schema
    assert_eq!(schema["properties"]["optional"]["type"], "array");
    assert_eq!(schema["properties"]["optional"]["items"]["type"], "string");

    // But it shouldn't be in required
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert!(required.contains(&json!("required")));
}
