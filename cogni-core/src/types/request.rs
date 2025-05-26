//! Request types for LLM interactions

use crate::types::message::Message;
use crate::types::structured::ResponseFormat;
use crate::types::tool::Tool;
use thiserror::Error;

/// A model identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model(pub String);

impl Model {
    /// Create a new model identifier
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for Model {
    fn default() -> Self {
        Self("gpt-4".to_string())
    }
}

impl From<&str> for Model {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Model {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parameters for controlling LLM generation
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parameters {
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature for randomness (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Top-p nucleus sampling
    pub top_p: Option<f32>,
    /// Number of completions to generate
    pub n: Option<u32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f32>,
    /// Random seed for deterministic generation
    pub seed: Option<u64>,
}

impl Parameters {
    /// Create a new parameters builder
    pub fn builder() -> ParametersBuilder {
        ParametersBuilder::default()
    }
}

/// Builder for Parameters
#[derive(Default)]
pub struct ParametersBuilder {
    params: Parameters,
}

impl ParametersBuilder {
    /// Set maximum tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.params.max_tokens = Some(tokens);
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.params.temperature = Some(temp);
        self
    }

    /// Set top-p
    pub fn top_p(mut self, p: f32) -> Self {
        self.params.top_p = Some(p);
        self
    }

    /// Set stop sequences
    pub fn stop(mut self, sequences: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.params.stop = Some(sequences.into_iter().map(Into::into).collect());
        self
    }

    /// Build the parameters
    pub fn build(self) -> Parameters {
        self.params
    }
}

/// A request to an LLM
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The conversation messages
    pub messages: Vec<Message>,
    /// The model to use
    pub model: Model,
    /// Generation parameters
    pub parameters: Parameters,
    /// Available tools/functions
    pub tools: Vec<Tool>,
    /// Response format specification
    pub response_format: Option<ResponseFormat>,
}

impl Request {
    /// Create a new request builder
    pub fn builder() -> RequestBuilder {
        RequestBuilder::default()
    }

    /// Create a simple request with just messages
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            model: Model::default(),
            parameters: Parameters::default(),
            tools: Vec::new(),
            response_format: None,
        }
    }

    /// Check if the request has tools available
    pub fn has_tools(&self) -> bool {
        !self.tools.is_empty()
    }
}

/// Builder for Request
#[derive(Default)]
pub struct RequestBuilder {
    messages: Vec<Message>,
    model: Option<Model>,
    parameters: Parameters,
    tools: Vec<Tool>,
    response_format: Option<ResponseFormat>,
}

impl RequestBuilder {
    /// Add a message
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<Model>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set parameters
    pub fn parameters(mut self, params: Parameters) -> Self {
        self.parameters = params;
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.parameters.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.parameters.max_tokens = Some(tokens);
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the response format
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Build the request
    pub fn build(self) -> Request {
        Request {
            messages: self.messages,
            model: self.model.unwrap_or_default(),
            parameters: self.parameters,
            tools: self.tools,
            response_format: self.response_format,
        }
    }

    /// Try to build the request, returning an error if validation fails
    pub fn try_build(self) -> Result<Request, BuildError> {
        if self.messages.is_empty() {
            return Err(BuildError::NoMessages);
        }

        Ok(Request {
            messages: self.messages,
            model: self.model.unwrap_or_default(),
            parameters: self.parameters,
            tools: self.tools,
            response_format: self.response_format,
        })
    }
}

/// Errors that can occur when building a request
#[derive(Debug, Error)]
pub enum BuildError {
    /// Request must contain at least one message
    #[error("Request must contain at least one message")]
    NoMessages,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::message::Message;
    use crate::types::tool::{Function, Tool};
    use serde_json::json;

    #[test]
    fn test_model_creation() {
        let model = Model::new("gpt-4");
        assert_eq!(model.0, "gpt-4");

        let model = Model::from("claude-3");
        assert_eq!(model.0, "claude-3");

        let model = Model::from("llama".to_string());
        assert_eq!(model.0, "llama");

        let model: Model = "custom-model".into();
        assert_eq!(model.0, "custom-model");
    }

    #[test]
    fn test_model_default() {
        let model = Model::default();
        assert_eq!(model.0, "gpt-4");
    }

    #[test]
    fn test_model_display() {
        let model = Model("test-model".to_string());
        assert_eq!(model.to_string(), "test-model");
    }

    #[test]
    fn test_parameters_builder() {
        let params = Parameters::builder()
            .max_tokens(100)
            .temperature(0.7)
            .top_p(0.9)
            .stop(vec!["\\n", "STOP"])
            .build();

        assert_eq!(params.max_tokens, Some(100));
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.top_p, Some(0.9));
        assert_eq!(
            params.stop,
            Some(vec!["\\n".to_string(), "STOP".to_string()])
        );
        assert_eq!(params.n, None);
        assert_eq!(params.presence_penalty, None);
        assert_eq!(params.frequency_penalty, None);
        assert_eq!(params.seed, None);
    }

    #[test]
    fn test_parameters_default() {
        let params = Parameters::default();
        assert!(params.max_tokens.is_none());
        assert!(params.temperature.is_none());
        assert!(params.top_p.is_none());
        assert!(params.stop.is_none());
    }

    #[test]
    fn test_request_new() {
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello"),
        ];
        let request = Request::new(messages.clone());

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.model.0, "gpt-4");
        assert_eq!(request.parameters, Parameters::default());
        assert!(request.tools.is_empty());
        assert!(request.response_format.is_none());
    }

    #[test]
    fn test_request_has_tools() {
        let request = Request::new(vec![Message::user("test")]);
        assert!(!request.has_tools());

        let mut request_with_tools = request.clone();
        request_with_tools.tools.push(Tool {
            name: "test".to_string(),
            description: "test tool".to_string(),
            function: Function {
                parameters: json!({}),
                returns: None,
            },
        });
        assert!(request_with_tools.has_tools());
    }

    #[test]
    fn test_request_builder_basic() {
        let request = Request::builder()
            .message(Message::system("System message"))
            .message(Message::user("User message"))
            .model("gpt-3.5-turbo")
            .temperature(0.5)
            .max_tokens(1000)
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.model.0, "gpt-3.5-turbo");
        assert_eq!(request.parameters.temperature, Some(0.5));
        assert_eq!(request.parameters.max_tokens, Some(1000));
    }

    #[test]
    fn test_request_builder_with_messages() {
        let messages = vec![
            Message::user("First"),
            Message::assistant("Response"),
            Message::user("Second"),
        ];

        let request = Request::builder().messages(messages.clone()).build();

        assert_eq!(request.messages.len(), 3);
    }

    #[test]
    fn test_request_builder_with_parameters() {
        let params = Parameters::builder()
            .temperature(0.8)
            .max_tokens(500)
            .build();

        let request = Request::builder()
            .message(Message::user("test"))
            .parameters(params.clone())
            .temperature(0.9) // This should override the params temperature
            .build();

        assert_eq!(request.parameters.temperature, Some(0.9));
        assert_eq!(request.parameters.max_tokens, Some(500));
    }

    #[test]
    fn test_request_builder_with_tools() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: "Calculates math".to_string(),
            function: Function {
                parameters: json!({"type": "object"}),
                returns: Some("number".to_string()),
            },
        };

        let request = Request::builder()
            .message(Message::user("Calculate 2+2"))
            .tool(tool.clone())
            .build();

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].name, "calculator");
    }

    #[test]
    fn test_request_builder_with_response_format() {
        let format = ResponseFormat::JsonObject;

        let request = Request::builder()
            .message(Message::user("test"))
            .response_format(format.clone())
            .build();

        assert_eq!(request.response_format, Some(ResponseFormat::JsonObject));
    }

    #[test]
    fn test_request_builder_try_build_success() {
        let result = Request::builder()
            .message(Message::user("test"))
            .try_build();

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_request_builder_try_build_no_messages() {
        let result = Request::builder().try_build();

        assert!(result.is_err());
        match result {
            Err(BuildError::NoMessages) => {}
            _ => panic!("Expected NoMessages error"),
        }
    }

    #[test]
    fn test_build_error_display() {
        let error = BuildError::NoMessages;
        assert_eq!(
            error.to_string(),
            "Request must contain at least one message"
        );
    }

    #[test]
    fn test_request_builder_default() {
        let builder = RequestBuilder::default();
        let request = builder.message(Message::user("test")).build();

        assert_eq!(request.model.0, "gpt-4");
    }

    #[test]
    fn test_model_equality() {
        let model1 = Model("gpt-4".to_string());
        let model2 = Model("gpt-4".to_string());
        let model3 = Model("claude".to_string());

        assert_eq!(model1, model2);
        assert_ne!(model1, model3);
    }

    #[test]
    fn test_parameters_equality() {
        let params1 = Parameters::builder().temperature(0.7).build();
        let params2 = Parameters::builder().temperature(0.7).build();
        let params3 = Parameters::builder().temperature(0.8).build();

        assert_eq!(params1, params2);
        assert_ne!(params1, params3);
    }

    #[test]
    fn test_request_clone() {
        let request = Request::builder()
            .message(Message::user("test"))
            .model("test-model")
            .temperature(0.5)
            .build();

        let cloned = request.clone();
        assert_eq!(request, cloned);
    }
}
