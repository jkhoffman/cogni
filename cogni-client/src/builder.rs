//! Request builder for fluent API

use cogni_core::{
    Content, Message, Metadata, Model, Parameters, Request, ResponseFormat, Role, StructuredOutput,
    Tool,
};

/// Builder for constructing requests with a fluent API
///
/// # Examples
///
/// ```
/// use cogni_client::RequestBuilder;
/// use cogni_core::Role;
///
/// let request = RequestBuilder::new()
///     .system("You are a helpful assistant")
///     .user("What is the weather like?")
///     .temperature(0.7)
///     .max_tokens(100)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct RequestBuilder {
    messages: Vec<Message>,
    model: Option<Model>,
    parameters: Parameters,
    tools: Vec<Tool>,
    response_format: Option<ResponseFormat>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a system message
    pub fn system(self, content: impl Into<String>) -> Self {
        self.message(Role::System, content.into())
    }

    /// Add a user message
    pub fn user(self, content: impl Into<String>) -> Self {
        self.message(Role::User, content.into())
    }

    /// Add an assistant message
    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.message(Role::Assistant, content.into())
    }

    /// Add a message with a specific role and content
    pub fn message(mut self, role: Role, content: impl Into<Content>) -> Self {
        self.messages.push(Message {
            role,
            content: content.into(),
            metadata: Metadata::default(),
        });
        self
    }

    /// Add a message with full control
    pub fn with_message(mut self, message: Message) -> Self {
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

    /// Set the temperature (0.0 to 2.0)
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.parameters.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.parameters.max_tokens = Some(max_tokens);
        self
    }

    /// Set the top_p parameter
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.parameters.top_p = Some(top_p);
        self
    }

    /// Set the frequency penalty
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.parameters.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.parameters.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences
    pub fn stop(mut self, stop: impl Into<Vec<String>>) -> Self {
        self.parameters.stop = Some(stop.into());
        self
    }

    /// Set custom parameters
    pub fn parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// Add one or more tools
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set the response format
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Request structured output of a specific type
    pub fn with_structured_output<T: StructuredOutput>(mut self) -> Self {
        self.response_format = Some(ResponseFormat::JsonSchema {
            schema: T::schema(),
            strict: true,
        });
        self
    }

    /// Request JSON object output
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::JsonObject);
        self
    }

    /// Build the request
    ///
    /// # Panics
    ///
    /// Panics if no messages have been added to the request.
    pub fn build(self) -> Request {
        if self.messages.is_empty() {
            panic!("Request must contain at least one message");
        }

        Request {
            messages: self.messages,
            model: self.model.unwrap_or_default(),
            parameters: self.parameters,
            tools: self.tools,
            response_format: self.response_format,
        }
    }

    /// Try to build the request, returning an error if validation fails
    pub fn try_build(self) -> Result<Request, BuilderError> {
        if self.messages.is_empty() {
            return Err(BuilderError::NoMessages);
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
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Request must contain at least one message")]
    NoMessages,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = RequestBuilder::new()
            .system("You are a helpful assistant")
            .user("Hello")
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, Role::System);
        assert_eq!(request.messages[1].role, Role::User);
    }

    #[test]
    fn test_builder_with_parameters() {
        let request = RequestBuilder::new()
            .user("Hello")
            .temperature(0.7)
            .max_tokens(100)
            .top_p(0.9)
            .build();

        assert_eq!(request.parameters.temperature, Some(0.7));
        assert_eq!(request.parameters.max_tokens, Some(100));
        assert_eq!(request.parameters.top_p, Some(0.9));
    }

    #[test]
    fn test_builder_with_model() {
        let request = RequestBuilder::new().user("Hello").model("gpt-4").build();

        assert_eq!(request.model.to_string(), "gpt-4");
    }

    #[test]
    #[should_panic(expected = "Request must contain at least one message")]
    fn test_builder_no_messages_panics() {
        RequestBuilder::new().build();
    }

    #[test]
    fn test_try_build_no_messages() {
        let result = RequestBuilder::new().try_build();
        assert!(matches!(result, Err(BuilderError::NoMessages)));
    }
}
