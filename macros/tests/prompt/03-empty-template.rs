use cogni_macros::prompt;

fn main() {
    let template = prompt!("Hello world!"); // Should fail: no placeholders
}
