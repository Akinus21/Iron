fn main() {
    println!("cargo:rerun-if-env-changed=CEF_PATH");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");
}