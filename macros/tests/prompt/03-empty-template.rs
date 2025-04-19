use cogni_macros::prompt;
use serde::Serialize;

fn main() {
    let template = prompt!("Hello world!"); // Should fail: no placeholders
}
