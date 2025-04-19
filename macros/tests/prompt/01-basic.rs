use cogni_core::prompt::{MissingPlaceholderError, PromptArgs};
use cogni_macros::prompt;
use serde::Serialize;

fn main() {
    #[derive(Debug, Serialize)]
    struct TemplateArgs {
        name: String,
    }

    impl PromptArgs for TemplateArgs {
        fn validate(&self) -> Result<(), MissingPlaceholderError> {
            if self.name.is_empty() {
                return Err(MissingPlaceholderError {
                    name: "name".to_string(),
                });
            }
            Ok(())
        }
    }

    let template = prompt!("Hello {{name}}!");
    let args = TemplateArgs {
        name: "World".to_string(),
    };
    let result = template.render(&args).unwrap();
    assert_eq!(result, "Hello World!");
}
