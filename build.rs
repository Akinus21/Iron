fn main() {
    println!("cargo:rerun-if-env-changed=CEF_PATH");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");

    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");

    if let Ok(cef_path) = std::env::var("CEF_PATH") {
        println!("cargo:rustc-link-search=native={}", cef_path);
        println!("cargo:rustc-link-lib=dylib=cef");
    }

    let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
    if !out_dir.is_empty() {
        let lib_dir = std::path::Path::new(&out_dir).join("out");
        if lib_dir.exists() {
            if let Some(lib_dir_str) = lib_dir.to_str() {
                println!("cargo:rustc-link-search=native={}", lib_dir_str);
            }
        }
    }
}