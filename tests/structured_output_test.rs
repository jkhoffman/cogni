//! Integration tests for structured output functionality

use cogni::prelude::*;
use cogni::{providers::OpenAI, ResponseFormat, StructuredOutput};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestStructure {
    name: String,
    value: i32,
    active: bool,
}

impl StructuredOutput for TestStructure {
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "value": { "type": "integer" },
                "active": { "type": "boolean" }
            },
            "required": ["name", "value", "active"],
            "additionalProperties": false
        })
    }
}

#[tokio::test]
async fn test_structured_output_with_openai() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI structured output test - OPENAI_API_KEY not set");
            return Ok(());
        }
    };

    let provider = OpenAI::with_api_key(api_key)?;
    let client = Client::new(provider);

    // Test with structured output
    let result: TestStructure = client
        .request()
        .system("You are a JSON generator. Generate the exact JSON structure requested.")
        .user("Generate a TestStructure with name='test', value=42, active=true")
        .with_structured_output::<TestStructure>()
        .model("gpt-4o-mini")
        .send()
        .await
        .expect("Request should succeed")
        .parse_structured()
        .expect("Should parse structured response");

    assert_eq!(result.name, "test");
    assert_eq!(result.value, 42);
    assert!(result.active);

    Ok(())
}

#[tokio::test]
async fn test_json_mode_with_openai() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Skipping OpenAI JSON mode test - OPENAI_API_KEY not set");
            return Ok(());
        }
    };

    let provider = OpenAI::with_api_key(api_key)?;
    let client = Client::new(provider);

    // Test with JSON mode
    let response = client
        .request()
        .system("You are a helpful assistant that always responds in JSON format.")
        .user("Create a JSON object with a 'status' field set to 'ok'")
        .json_mode()
        .model("gpt-3.5-turbo")
        .send()
        .await
        .expect("Request should succeed");

    let json_value = response.parse_json().expect("Should parse as JSON");
    assert_eq!(json_value["status"], "ok");

    Ok(())
}

#[tokio::test]
async fn test_response_format_in_request() -> Result<(), Box<dyn std::error::Error>> {
    // Test that response_format is properly set in request
    let request = RequestBuilder::new()
        .user("test")
        .response_format(ResponseFormat::JsonObject)
        .build();

    assert_eq!(request.response_format, Some(ResponseFormat::JsonObject));

    // Test with JSON schema
    let schema = json!({
        "type": "object",
        "properties": {
            "test": { "type": "string" }
        }
    });

    let request = RequestBuilder::new()
        .user("test")
        .response_format(ResponseFormat::JsonSchema {
            schema: schema.clone(),
            strict: true,
        })
        .build();

    match request.response_format {
        Some(ResponseFormat::JsonSchema { schema: s, strict }) => {
            assert_eq!(s, schema);
            assert!(strict);
        }
        _ => panic!("Expected JsonSchema response format"),
    }

    Ok(())
}

#[tokio::test]
async fn test_parse_structured_response() -> Result<(), Box<dyn std::error::Error>> {
    // Test parsing a valid JSON response
    let response = Response {
        content: r#"{"name": "test", "value": 123, "active": false}"#.to_string(),
        tool_calls: vec![],
        metadata: Default::default(),
    };

    let result: TestStructure = response
        .parse_structured()
        .expect("Should parse valid JSON");

    assert_eq!(result.name, "test");
    assert_eq!(result.value, 123);
    assert!(!result.active);

    // Test parsing invalid JSON
    let response = Response {
        content: "not json".to_string(),
        tool_calls: vec![],
        metadata: Default::default(),
    };

    let result: Result<TestStructure, _> = response.parse_structured();
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_parse_json_response() -> Result<(), Box<dyn std::error::Error>> {
    // Test parsing valid JSON
    let response = Response {
        content: r#"{"key": "value", "number": 42}"#.to_string(),
        tool_calls: vec![],
        metadata: Default::default(),
    };

    let json = response.parse_json().expect("Should parse as JSON");
    assert_eq!(json["key"], "value");
    assert_eq!(json["number"], 42);

    // Test parsing invalid JSON
    let response = Response {
        content: "not json".to_string(),
        tool_calls: vec![],
        metadata: Default::default(),
    };

    assert!(response.parse_json().is_err());

    Ok(())
}
