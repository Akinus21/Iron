//! Build script for Iron

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CEF_DIR");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");

    let cef_dir = std::env::var("CEF_DIR")
        .map(PathBuf::from)
        .expect("CEF_DIR must be set — point it at your extracted CEF binary distribution");

    let release_dir = cef_dir.join("Release");
    let wrapper_dir = cef_dir.join("build").join("libcef_dll_wrapper");

    println!("cargo:rustc-link-search=native={}", release_dir.display());
    println!("cargo:rustc-link-search=native={}", wrapper_dir.display());
    println!("cargo:rustc-link-lib=dylib=cef");
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();

    for entry in ["Release", "Resources"] {
        let src = cef_dir.join(entry);
        if src.exists() {
            let dst = target_dir.join(entry);
            std::fs::create_dir_all(&dst).ok();
            copy_dir_all(&src, &dst);
        }
    }
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&dst_path).ok();
            copy_dir_all(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), dst_path).ok();
        }
    }
}