//! Request types for LLM interactions

use crate::types::message::Message;
use crate::types::tool::Tool;

/// A model identifier
#[derive(Debug, Clone, PartialEq)]
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
        }
    }
}

/// Builder for Request
#[derive(Default)]
pub struct RequestBuilder {
    messages: Vec<Message>,
    model: Option<Model>,
    parameters: Parameters,
    tools: Vec<Tool>,
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
    
    /// Build the request
    pub fn build(self) -> Request {
        Request {
            messages: self.messages,
            model: self.model.unwrap_or_default(),
            parameters: self.parameters,
            tools: self.tools,
        }
    }
}