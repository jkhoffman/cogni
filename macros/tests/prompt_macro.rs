use cogni_core::PromptTemplate;
use cogni_core::prompt::PromptArgs;
use cogni_macros::prompt;
use std::collections::HashMap;
use trybuild::TestCases;

#[test]
fn test_basic_prompt() {
    let template = prompt!("Hello {{name}}, you are {{age}} years old!");
    let result = template.to_string();
    assert_eq!(result, "Hello {{name}}, you are {{age}} years old!");
}

#[test]
fn test_prompt_macro() {
    let t = TestCases::new();
    t.pass("tests/prompt/01-basic.rs");
    t.pass("tests/prompt/02-multiple-placeholders.rs");
    t.compile_fail("tests/prompt/03-empty-template.rs");
    t.compile_fail("tests/prompt/04-invalid-placeholder.rs");
}
