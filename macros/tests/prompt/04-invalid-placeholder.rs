use cogni_macros::prompt;
use serde::Serialize;

fn main() {
    let template = prompt!("Hello {{name!}}!"); // Should fail: invalid placeholder name
}
