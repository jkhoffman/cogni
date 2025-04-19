use cogni_macros::prompt;

fn main() {
    let template = prompt!("Hello {{name!}}!"); // Should fail: invalid placeholder name
}
