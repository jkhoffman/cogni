fn main() {
    // Tell cargo to rerun this build script if the migrations change
    println!("cargo:rerun-if-changed=migrations");
}
