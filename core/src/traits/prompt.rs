//! Prompt handling for the Cogni framework.
//!
//! This module provides the core traits and types for handling prompts
//! in a type-safe and flexible way. It includes support for template
//! rendering with placeholder validation.

use handlebars::Handlebars;
use regex::Regex;
use serde::Serialize;
use std::fmt;
use thiserror::Error;

/// Error type for prompt template operations.
#[derive(Debug, Error)]
pub enum PromptError {
    /// A required placeholder is missing
    #[error(transparent)]
    MissingPlaceholder(#[from] MissingPlaceholderError),
    /// Failed to render the template
    #[error("failed to render template: {0}")]
    RenderError(#[from] handlebars::RenderError),
}

/// Error type for missing placeholders in a prompt template.
#[derive(Debug, Error)]
#[error("missing required placeholder: {name}")]
pub struct MissingPlaceholderError {
    /// The name of the missing placeholder
    pub name: String,
}

/// A trait for types that can be used as arguments to a prompt template.
///
/// This trait must be implemented by any type that will be used to render
/// a prompt template. It ensures that all required placeholders have values
/// before rendering.
///
/// # Examples
///
/// ```rust
/// use cogni_core::traits::prompt::{PromptArgs, MissingPlaceholderError};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyArgs {
///     name: String,
///     age: u32,
/// }
///
/// impl PromptArgs for MyArgs {
///     fn validate(&self) -> Result<(), MissingPlaceholderError> {
///         if self.name.is_empty() {
///             return Err(MissingPlaceholderError {
///                 name: "name".to_string(),
///             });
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait PromptArgs: Serialize {
    /// Validates that all required placeholders are present.
    ///
    /// This method should check that all required fields have valid values
    /// before the template is rendered.
    ///
    /// # Returns
    /// - `Ok(())` if all required placeholders are present
    /// - `Err(MissingPlaceholderError)` if any required placeholder is missing
    fn validate(&self) -> Result<(), MissingPlaceholderError>;
}

/// A parsed template with placeholders that can be rendered with a context.
///
/// This type represents a prompt template that can be rendered with a set
/// of arguments. It supports the Handlebars template syntax with {{placeholder}}
/// notation.
///
/// This type is typically created via the `prompt!` macro which provides
/// compile-time validation of placeholders.
///
/// # Examples
///
/// ```rust
/// use cogni_core::traits::prompt::{PromptTemplate, PromptArgs};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyArgs {
///     name: String,
/// }
///
/// impl PromptArgs for MyArgs {
///     fn validate(&self) -> Result<(), cogni_core::traits::prompt::MissingPlaceholderError> {
///         if self.name.is_empty() {
///             return Err(cogni_core::traits::prompt::MissingPlaceholderError {
///                 name: "name".to_string(),
///             });
///         }
///         Ok(())
///     }
/// }
///
/// let template = PromptTemplate::new("Hello {{name}}!");
/// let args = MyArgs {
///     name: "Alice".to_string(),
/// };
/// let result = template.render(&args).unwrap();
/// assert_eq!(result, "Hello Alice!");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    /// The raw template string with {{placeholder}} syntax
    template: String,
    /// Set of required placeholder names
    placeholders: Vec<String>,
}

impl PromptTemplate {
    /// Creates a new template from a string, extracting placeholders.
    ///
    /// This is typically called by the `prompt!` macro, not directly.
    /// The macro provides compile-time validation of placeholders.
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let placeholders = Self::extract_placeholders(&template);
        Self {
            template,
            placeholders,
        }
    }

    /// Renders the template with the given context.
    ///
    /// The context must implement `PromptArgs` which ensures all placeholders
    /// are present before rendering.
    ///
    /// # Arguments
    /// * `args` - The arguments to use for rendering
    ///
    /// # Returns
    /// - `Ok(String)` containing the rendered template
    /// - `Err(PromptError)` if validation fails or rendering fails
    pub fn render<T: PromptArgs>(&self, args: &T) -> Result<String, PromptError> {
        args.validate()?;

        let reg = Handlebars::new();
        Ok(reg.render_template(&self.template, args)?)
    }

    /// Returns the set of placeholder names in this template.
    ///
    /// This can be used to inspect what placeholders are required
    /// by the template.
    pub fn placeholders(&self) -> &[String] {
        &self.placeholders
    }

    /// Extracts placeholder names from a template string.
    fn extract_placeholders(template: &str) -> Vec<String> {
        let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
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
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestArgs {
        name: String,
        age: String,
    }

    impl PromptArgs for TestArgs {
        fn validate(&self) -> Result<(), MissingPlaceholderError> {
            if self.name.is_empty() {
                return Err(MissingPlaceholderError {
                    name: "name".to_string(),
                });
            }
            if self.age.is_empty() {
                return Err(MissingPlaceholderError {
                    name: "age".to_string(),
                });
            }
            Ok(())
        }
    }

    #[test]
    fn test_template_rendering() {
        let template = PromptTemplate::new("Hello {{name}}, you are {{age}} years old!");
        let args = TestArgs {
            name: "Alice".to_string(),
            age: "30".to_string(),
        };
        let result = template.render(&args).unwrap();
        assert_eq!(result, "Hello Alice, you are 30 years old!");
    }

    #[test]
    fn test_missing_placeholder() {
        let template = PromptTemplate::new("Hello {{name}}, you are {{age}} years old!");
        let args = TestArgs {
            name: "".to_string(),
            age: "30".to_string(),
        };
        let result = template.render(&args);
        assert!(matches!(
            result.unwrap_err(),
            PromptError::MissingPlaceholder(_)
        ));
    }

    #[test]
    fn test_extract_placeholders() {
        let template = PromptTemplate::new("Hello {{name}}, you are {{age}} years old!");
        assert_eq!(template.placeholders(), &["name", "age"]);
    }
}
