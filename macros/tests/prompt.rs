#[test]
fn test_prompt_macro() {
    let t = trybuild::TestCases::new();
    t.pass("tests/prompt/01-basic.rs");
    t.pass("tests/prompt/02-multiple-placeholders.rs");
    t.compile_fail("tests/prompt/03-empty-template.rs");
    t.compile_fail("tests/prompt/04-invalid-placeholder.rs");
}
