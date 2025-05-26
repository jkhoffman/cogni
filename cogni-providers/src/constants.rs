//! Constants for provider implementations

/// Default Ollama base URL
pub const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Default Ollama model
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3.2";

/// Default Anthropic base URL
pub const ANTHROPIC_DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default Anthropic model
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";

/// Default OpenAI base URL
pub const OPENAI_DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// Default OpenAI model
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-3.5-turbo";

/// Default max tokens if not specified
pub const DEFAULT_MAX_TOKENS: u32 = 4096;
