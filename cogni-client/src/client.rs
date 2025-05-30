//! High-level client implementation

use crate::{RequestBuilder, StatefulClient};
use cogni_context::ContextManager;
use cogni_core::{
    Content, Error, Message, Metadata, Model, Parameters, Provider, Request, Response, Role,
    StreamEvent, StructuredOutput,
};
use cogni_state::StateStore;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use std::pin::Pin;
use std::sync::Arc;

/// High-level client for LLM interactions
///
/// This client provides a simplified interface for common operations while
/// still allowing full control when needed.
///
/// # Examples
///
/// ```no_run
/// use cogni_client::Client;
/// use cogni_providers::OpenAI;
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAI::with_api_key("your-api-key".to_string())?;
/// let client = Client::new(provider);
///
/// // Simple chat
/// let response = client.chat("Hello, how are you?").await?;
/// println!("{}", response);
///
/// // Streaming chat
/// let mut stream = client.stream_chat("Tell me a story").await?;
/// while let Some(chunk) = stream.next().await {
///     print!("{}", chunk?);
/// }
/// # Ok(())
/// # }
/// ```
pub struct Client<P: Provider> {
    pub(crate) provider: P,
    pub(crate) default_model: Model,
    pub(crate) default_parameters: Parameters,
}

impl<P: Provider> Client<P> {
    /// Create a new client with a provider
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            default_model: Model::default(),
            default_parameters: Parameters::default(),
        }
    }

    /// Set the default model for requests
    pub fn with_model(mut self, model: impl Into<Model>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set default parameters for requests
    pub fn with_parameters(mut self, parameters: Parameters) -> Self {
        self.default_parameters = parameters;
        self
    }

    /// Create a stateful client with conversation persistence
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cogni_client::Client;
    /// # use cogni_providers::OpenAI;
    /// # use cogni_state::MemoryStore;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider = OpenAI::with_api_key("key".to_string())?;
    /// let client = Client::new(provider);
    /// let store = Arc::new(MemoryStore::new());
    /// let mut stateful = client.with_state(store);
    ///
    /// // Start a new conversation
    /// stateful.new_conversation().await?;
    /// let response = stateful.chat("Hello!").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_state(self, store: Arc<dyn StateStore>) -> StatefulClient<P> {
        StatefulClient::new(self, store)
    }

    /// Simple chat interface
    ///
    /// This method accepts either a single message or a vector of messages.
    pub async fn chat(&self, messages: impl Into<MessageInput>) -> Result<String, Error> {
        let request = Request {
            messages: messages.into().into_messages(),
            model: self.default_model.clone(),
            parameters: self.default_parameters.clone(),
            tools: Vec::new(),
            response_format: None,
        };

        let response = self.provider.request(request).await?;
        Ok(response.content)
    }

    /// Streaming chat interface
    ///
    /// Returns a stream of content chunks that can be processed as they arrive.
    pub async fn stream_chat(
        &self,
        messages: impl Into<MessageInput>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, Error>> + Send + 'static>>, Error>
    where
        P::Stream: 'static,
    {
        let request = Request {
            messages: messages.into().into_messages(),
            model: self.default_model.clone(),
            parameters: self.default_parameters.clone(),
            tools: Vec::new(),
            response_format: None,
        };

        let stream = self.provider.stream(request).await?;

        // Use take_while to stop the stream when Done is received
        let content_stream = stream
            .take_while(|event| futures::future::ready(!matches!(event, Ok(StreamEvent::Done))))
            .filter_map(|event| async move {
                match event {
                    Ok(StreamEvent::Content(delta)) => Some(Ok(delta.text)),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(content_stream))
    }

    /// Create a request builder for more complex scenarios
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cogni_client::Client;
    /// # use cogni_providers::OpenAI;
    /// # use cogni_core::Role;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider = OpenAI::with_api_key("key".to_string())?;
    /// # let client = Client::new(provider);
    /// let response = client
    ///     .request()
    ///     .system("You are a helpful assistant")
    ///     .user("What is the weather like?")
    ///     .temperature(0.7)
    ///     .max_tokens(100)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn request(&self) -> ConnectedRequestBuilder<'_, P> {
        ConnectedRequestBuilder {
            client: self,
            builder: RequestBuilder::new()
                .model(self.default_model.clone())
                .parameters(self.default_parameters.clone()),
            context_manager: None,
        }
    }

    /// Chat with structured output
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cogni_client::Client;
    /// # use cogni_providers::OpenAI;
    /// # use cogni_core::StructuredOutput;
    /// # use serde::{Deserialize, Serialize};
    /// # use serde_json::json;
    /// #
    /// # #[derive(Debug, Serialize, Deserialize)]
    /// # struct WeatherReport {
    /// #     temperature: f32,
    /// #     conditions: String,
    /// # }
    /// #
    /// # impl StructuredOutput for WeatherReport {
    /// #     fn schema() -> serde_json::Value {
    /// #         json!({
    /// #             "type": "object",
    /// #             "properties": {
    /// #                 "temperature": { "type": "number" },
    /// #                 "conditions": { "type": "string" }
    /// #             },
    /// #             "required": ["temperature", "conditions"]
    /// #         })
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider = OpenAI::with_api_key("key".to_string())?;
    /// # let client = Client::new(provider);
    /// let weather: WeatherReport = client
    ///     .chat_structured("What's the weather like in San Francisco?")
    ///     .await?;
    /// println!("Temperature: {}°F", weather.temperature);
    /// println!("Conditions: {}", weather.conditions);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat_structured<T, M>(&self, messages: M) -> Result<T, Error>
    where
        T: StructuredOutput + for<'de> Deserialize<'de>,
        M: Into<MessageInput>,
    {
        let response = self
            .request()
            .with_structured_output::<T>()
            .messages(messages.into().into_messages())
            .send()
            .await?;

        response.parse_structured()
    }

    /// Get a reference to the underlying provider
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Execute a pre-built request
    pub async fn execute(&self, request: Request) -> Result<Response, Error> {
        self.provider.request(request).await
    }

    /// Execute a pre-built request with streaming
    pub async fn execute_stream(&self, request: Request) -> Result<P::Stream, Error> {
        self.provider.stream(request).await
    }
}

/// Helper enum for accepting different message inputs
#[doc(hidden)]
pub enum MessageInput {
    Single(String),
    Multiple(Vec<Message>),
}

impl From<&str> for MessageInput {
    fn from(s: &str) -> Self {
        MessageInput::Single(s.to_string())
    }
}

impl From<String> for MessageInput {
    fn from(s: String) -> Self {
        MessageInput::Single(s)
    }
}

impl From<Vec<Message>> for MessageInput {
    fn from(messages: Vec<Message>) -> Self {
        MessageInput::Multiple(messages)
    }
}

impl MessageInput {
    pub(crate) fn into_messages(self) -> Vec<Message> {
        match self {
            MessageInput::Single(text) => vec![Message {
                role: Role::User,
                content: Content::Text(text),
                metadata: Metadata::default(),
            }],
            MessageInput::Multiple(messages) => messages,
        }
    }
}

/// Request builder connected to a client
pub struct ConnectedRequestBuilder<'a, P: Provider> {
    client: &'a Client<P>,
    builder: RequestBuilder,
    context_manager: Option<Arc<ContextManager>>,
}

impl<P: Provider> ConnectedRequestBuilder<'_, P> {
    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.builder = self.builder.system(content);
        self
    }

    /// Add a user message
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.builder = self.builder.user(content);
        self
    }

    /// Add an assistant message
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.builder = self.builder.assistant(content);
        self
    }

    /// Add a message with a specific role
    pub fn message(mut self, role: Role, content: impl Into<Content>) -> Self {
        self.builder = self.builder.message(role, content);
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<Model>) -> Self {
        self.builder = self.builder.model(model);
        self
    }

    /// Set the temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.builder = self.builder.temperature(temperature);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.builder = self.builder.max_tokens(max_tokens);
        self
    }

    /// Set parameters
    pub fn parameters(mut self, parameters: Parameters) -> Self {
        self.builder = self.builder.parameters(parameters);
        self
    }

    /// Add multiple messages
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.builder = self.builder.messages(messages);
        self
    }

    /// Add one or more tools
    pub fn tools(mut self, tools: impl IntoIterator<Item = cogni_core::Tool>) -> Self {
        self.builder = self.builder.tools(tools);
        self
    }

    /// Set the response format
    pub fn response_format(mut self, format: cogni_core::ResponseFormat) -> Self {
        self.builder = self.builder.response_format(format);
        self
    }

    /// Request structured output of a specific type
    pub fn with_structured_output<T: cogni_core::StructuredOutput>(mut self) -> Self {
        self.builder = self.builder.with_structured_output::<T>();
        self
    }

    /// Request JSON object output
    pub fn json_mode(mut self) -> Self {
        self.builder = self.builder.json_mode();
        self
    }

    /// Set a context manager for automatic message pruning
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cogni_client::Client;
    /// # use cogni_providers::OpenAI;
    /// # use cogni_context::{ContextManager, TiktokenCounter};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider = OpenAI::with_api_key("key".to_string())?;
    /// # let client = Client::new(provider);
    /// let counter = Arc::new(TiktokenCounter::for_model("gpt-4")?);
    /// let context_manager = Arc::new(ContextManager::new(counter));
    ///
    /// let response = client
    ///     .request()
    ///     .with_context_manager(context_manager)
    ///     .user("Long conversation...")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_context_manager(mut self, manager: Arc<ContextManager>) -> Self {
        self.context_manager = Some(manager);
        self
    }

    /// Build the request
    pub fn build(self) -> Request {
        self.builder.build()
    }

    /// Send the request
    pub async fn send(self) -> Result<Response, Error> {
        let mut request = self.builder.build();

        // Apply context management if configured
        if let Some(manager) = &self.context_manager {
            request.messages = manager.fit_messages(request.messages).await?;
        }

        self.client.execute(request).await
    }

    /// Send the request and get a stream
    pub async fn stream(self) -> Result<P::Stream, Error> {
        let mut request = self.builder.build();

        // Apply context management if configured
        if let Some(manager) = &self.context_manager {
            request.messages = manager.fit_messages(request.messages).await?;
        }

        self.client.execute_stream(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::{ContentDelta, ResponseMetadata, StreamEvent};
    use futures::stream;

    // Mock provider for testing
    struct MockProvider;

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        type Stream = Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>;

        async fn request(&self, _request: Request) -> Result<Response, Error> {
            Ok(Response {
                content: "Hello from mock provider".to_string(),
                tool_calls: vec![],
                metadata: ResponseMetadata::default(),
            })
        }

        async fn stream(&self, _request: Request) -> Result<Self::Stream, Error> {
            let events = vec![
                Ok(StreamEvent::Content(ContentDelta {
                    text: "Hello ".to_string(),
                })),
                Ok(StreamEvent::Content(ContentDelta {
                    text: "world".to_string(),
                })),
                Ok(StreamEvent::Done),
            ];
            Ok(Box::pin(stream::iter(events)))
        }
    }

    #[tokio::test]
    async fn test_simple_chat() {
        let client = Client::new(MockProvider);
        let response = client.chat("Hello").await.unwrap();
        assert_eq!(response, "Hello from mock provider");
    }

    #[tokio::test]
    async fn test_streaming_chat() {
        let client = Client::new(MockProvider);
        let mut stream = client.stream_chat("Hello").await.unwrap();

        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk.unwrap());
        }

        assert_eq!(result, "Hello world");
    }

    #[tokio::test]
    async fn test_request_builder() {
        let client = Client::new(MockProvider);
        let response = client
            .request()
            .system("You are a helpful assistant")
            .user("Hello")
            .temperature(0.7)
            .send()
            .await
            .unwrap();

        assert_eq!(response.content, "Hello from mock provider");
    }
}
