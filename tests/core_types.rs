//! Tests for core types

use cogni_core::*;

#[test]
fn test_message_creation() {
    let msg = Message::user("Hello, world!");
    assert_eq!(msg.role, Role::User);
    assert!(matches!(msg.content, Content::Text(_)));
}

#[test]
fn test_request_builder() {
    let request = Request::builder()
        .message(Message::system("You are helpful"))
        .message(Message::user("Hello"))
        .model("gpt-4")
        .temperature(0.8)
        .max_tokens(100)
        .build();

    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.model.0, "gpt-4");
    assert_eq!(request.parameters.temperature, Some(0.8));
    assert_eq!(request.parameters.max_tokens, Some(100));
}

#[test]
fn test_tool_creation() {
    let tool = Tool {
        name: "get_weather".to_string(),
        description: "Get the weather for a location".to_string(),
        function: Function {
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state"
                    }
                },
                "required": ["location"]
            }),
            returns: Some("Weather information".to_string()),
        },
    };

    assert_eq!(tool.name, "get_weather");
    assert!(tool.function.parameters.is_object());
}
