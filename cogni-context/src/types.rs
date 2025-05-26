use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub model_name: String,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            context_window: 4096,    // Conservative default
            max_output_tokens: 1024, // Common default
            model_name: String::from("unknown"),
        }
    }
}

impl ModelLimits {
    pub fn for_model(model: &str) -> Option<Self> {
        // Define known model limits
        let limits = match model {
            // OpenAI models
            "gpt-4" | "gpt-4-0613" => Self {
                context_window: 8192,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "gpt-4-32k" | "gpt-4-32k-0613" => Self {
                context_window: 32768,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "gpt-4-turbo"
            | "gpt-4-1106-preview"
            | "gpt-4-0125-preview"
            | "gpt-4-turbo-preview"
            | "gpt-4-turbo-2024-04-09" => Self {
                context_window: 128000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "gpt-4o" | "gpt-4o-2024-05-13" => Self {
                context_window: 128000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "gpt-4o-mini" | "gpt-4o-mini-2024-07-18" => Self {
                context_window: 128000,
                max_output_tokens: 16384,
                model_name: model.to_string(),
            },
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0613" => Self {
                context_window: 4096,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "gpt-3.5-turbo-16k" | "gpt-3.5-turbo-16k-0613" => Self {
                context_window: 16384,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },

            // Anthropic models
            "claude-3-opus" | "claude-3-opus-20240229" => Self {
                context_window: 200000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "claude-3-sonnet" | "claude-3-sonnet-20240229" => Self {
                context_window: 200000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "claude-3-haiku" | "claude-3-haiku-20240307" => Self {
                context_window: 200000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "claude-2.1" => Self {
                context_window: 200000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },
            "claude-2.0" => Self {
                context_window: 100000,
                max_output_tokens: 4096,
                model_name: model.to_string(),
            },

            _ => return None,
        };

        Some(limits)
    }

    pub fn available_tokens(&self, reserve_output: usize) -> usize {
        self.context_window
            .saturating_sub(reserve_output.min(self.max_output_tokens))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_limits() {
        let limits = ModelLimits::for_model("gpt-4").unwrap();
        assert_eq!(limits.context_window, 8192);
        assert_eq!(limits.max_output_tokens, 4096);

        let limits = ModelLimits::for_model("claude-3-opus").unwrap();
        assert_eq!(limits.context_window, 200000);

        assert!(ModelLimits::for_model("unknown-model").is_none());
    }

    #[test]
    fn test_available_tokens() {
        let limits = ModelLimits::for_model("gpt-4").unwrap();
        assert_eq!(limits.available_tokens(1000), 7192);
        assert_eq!(limits.available_tokens(10000), 4096); // Capped at max_output_tokens
    }
}
