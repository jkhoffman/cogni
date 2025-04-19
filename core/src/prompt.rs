use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A parsed template with placeholders that can be rendered with a context.
/// This type is typically created via the `prompt!` macro which provides compile-time validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    /// The raw template string with {{placeholder}} syntax
    template: String,
    /// Set of required placeholder names
    placeholders: Vec<String>,
}

/// Trait implemented by generated argument structs from the prompt! macro
pub trait PromptArgs: Serialize {
    /// Validates that all required placeholders are present
    fn validate(&self) -> Result<(), MissingPlaceholderError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("missing required placeholder: {0}")]
    MissingPlaceholder(#[from] MissingPlaceholderError),
    #[error("failed to render template: {0}")]
    RenderError(#[from] handlebars::RenderError),
}

#[derive(Debug, thiserror::Error)]
#[error("missing required placeholder: {name}")]
pub struct MissingPlaceholderError {
    name: String,
}

impl PromptTemplate {
    /// Creates a new template from a string, extracting placeholders.
    /// This is typically called by the prompt! macro, not directly.
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let placeholders = Self::extract_placeholders(&template);
        Self {
            template,
            placeholders,
        }
    }

    /// Renders the template with the given context.
    /// The context must implement PromptArgs which ensures all placeholders are present.
    pub fn render<T: PromptArgs>(&self, args: &T) -> Result<String, PromptError> {
        args.validate()?;

        let reg = handlebars::Handlebars::new();
        Ok(reg.render_template(&self.template, args)?)
    }

    /// Returns the set of placeholder names in this template
    pub fn placeholders(&self) -> &[String] {
        &self.placeholders
    }

    fn extract_placeholders(template: &str) -> Vec<String> {
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        re.captures_iter(template)
            .map(|cap| cap[1].trim().to_string())
            .collect()
    }
}

impl fmt::Display for PromptTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_placeholders() {
        let template = "Hello {{name}}, you are {{age}} years old!";
        let prompt = PromptTemplate::new(template);
        assert_eq!(prompt.placeholders(), &["name", "age"]);
    }

    #[test]
    fn test_render_template() {
        let template = "Hello {{name}}, you are {{age}} years old!";
        let prompt = PromptTemplate::new(template);

        #[derive(Serialize)]
        struct Args {
            name: String,
            age: u32,
        }

        impl PromptArgs for Args {
            fn validate(&self) -> Result<(), MissingPlaceholderError> {
                Ok(())
            }
        }

        let args = Args {
            name: "Alice".to_string(),
            age: 30,
        };

        let result = prompt.render(&args).unwrap();
        assert_eq!(result, "Hello Alice, you are 30 years old!");
    }
}
