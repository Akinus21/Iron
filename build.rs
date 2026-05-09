fn main() {
    println!("cargo:rerun-if-env-changed=CEF_PATH");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");

    if let Ok(cef_path) = std::env::var("CEF_PATH") {
        println!("cargo:rustc-link-search=native={}", cef_path);
        println!("cargo:rustc-link-lib=dylib=cef");
    }
}