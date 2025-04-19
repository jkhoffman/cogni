use cogni_macros::prompt;

fn main() {
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
