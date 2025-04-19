use cogni_core::prompt::{MissingPlaceholderError, PromptArgs};
use cogni_macros::prompt;
use serde::Serialize;

fn main() {
    #[derive(Debug, Serialize)]
    struct TemplateArgs {
        name: String,
        age: String,
        city: String,
    }

    impl PromptArgs for TemplateArgs {
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
            if self.city.is_empty() {
                return Err(MissingPlaceholderError {
                    name: "city".to_string(),
                });
            }
            Ok(())
        }
    }

    let template = prompt!("Hello {{name}}, you are {{age}} years old and live in {{city}}!");
    let args = TemplateArgs {
        name: "Alice".to_string(),
        age: "30".to_string(),
        city: "London".to_string(),
    };
    let result = template.render(&args).unwrap();
    assert_eq!(
        result,
        "Hello Alice, you are 30 years old and live in London!"
    );
}
