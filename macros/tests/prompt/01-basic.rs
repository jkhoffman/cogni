use cogni_macros::prompt;

fn main() {
    let template = prompt!("Hello {{name}}!");
    let args = TemplateArgs {
        name: "World".to_string(),
    };
    let result = template.render(&args).unwrap();
    assert_eq!(result, "Hello World!");
}
