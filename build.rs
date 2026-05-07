//! Build script for CEF resources
//!
//! Copies CEF runtime resources to the output directory.
//! CEF library linking is handled automatically by cef-dll-sys.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CEF_DIR");

    // Get CEF_DIR from environment (set by cef-dll-sys) or look for it
    let cef_dir = if let Ok(dir) = env::var("CEF_DIR") {
        PathBuf::from(dir)
    } else {
        // Search in common locations used by cef-dll-sys
        let search_paths = [
            std::env::temp_dir().join("cef"),
            std::env::temp_dir().join("cef-download"),
            PathBuf::from("/tmp/cef"),
        ];

        for path in &search_paths {
            if path.join("Release").exists() || path.join("libcef.so").exists() {
                break;
            }
        }

        // If not found, exit successfully - cef-dll-sys will handle things
        return;
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_base = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or_else(|| PathBuf::from("target"));

    let release_dir = cef_dir.join("Release");

    let copy_file = |src: &std::path::Path, dst: &std::path::Path| {
        if src.exists() {
            let dst_path = dst.join(src.file_name().unwrap());
            if !dst_path.exists() {
                let _ = fs::copy(src, &dst_path);
            }
        }
    };

    if release_dir.join("libcef.so").exists() {
        copy_file(&release_dir.join("libcef.so"), &target_base);
    }
    copy_file(&cef_dir.join("icudtl.dat"), &target_base);
    copy_file(&cef_dir.join("chrome_100_percent.pak"), &target_base);
    copy_file(&cef_dir.join("chrome_200_percent.pak"), &target_base);
    copy_file(&cef_dir.join("resources.pak"), &target_base);
    copy_file(&cef_dir.join("snapshot_blob.bin"), &target_base);
    copy_file(&cef_dir.join("v8_context_snapshot.bin"), &target_base);

    let locales_src = cef_dir.join("locales");
    if locales_src.exists() {
        let locales_dst = target_base.join("locales");
        let _ = fs::create_dir_all(&locales_dst);
        if let Ok(entries) = fs::read_dir(&locales_src) {
            for entry in entries.flatten() {
                copy_file(&entry.path(), &locales_dst);
            }
        }
    }
}
