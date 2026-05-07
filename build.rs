//! Build script for CEF (Chromium Embedded Framework) integration
//! 
//! This script:
//! 1. Sets up CEF library paths for linking
//! 2. Copies CEF resources to output directory
//!
//! CEF is expected to be downloaded by cef-dll-sys or provided via CEF_DIR env var.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CEF_TRACK");
    println!("cargo:rerun-if-env-changed=CEF_DIR");
    
    let cef_dir = if let Ok(dir) = env::var("CEF_DIR") {
        PathBuf::from(&dir)
    } else {
        // CEF should have been downloaded by cef-dll-sys build script
        // Check common locations
        let candidates = [
            PathBuf::from("/tmp/cef-download"),
            PathBuf::from("/tmp/cef"),
            std::env::temp_dir().join("cef-download"),
            std::env::temp_dir().join("cef"),
        ];
        
        let mut found = None;
        for candidate in &candidates {
            if candidate.exists() && candidate.join("Release").exists() {
                found = Some(candidate.clone());
                break;
            }
            // Check subdirectories
            if let Ok(entries) = fs::read_dir(candidate) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.join("Release").exists() {
                        found = Some(path);
                        break;
                    }
                }
            }
            if found.is_some() { break; }
        }
        
        match found {
            Some(dir) => dir,
            None => {
                eprintln!("CEF not found. Please set CEF_DIR environment variable.");
                eprintln!("CEF is typically downloaded by cef-dll-sys during its build.");
                std::process::exit(1);
            }
        }
    };
    
    setup_cef_paths(&cef_dir);
}

fn setup_cef_paths(cef_path: &Path) {
    println!("cargo:rustc-link-search=native={}/Release", cef_path.display());
    println!("cargo:rustc-link-lib=dylib=cef");
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
    
    // Copy CEF resources to target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_base = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());
    
    let target_dir = target_base
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("target"));
    
    let release_dir = cef_path.join("Release");
    
    // Copy shared libraries
    let copy_file = |src: &Path, dst_dir: &Path| {
        if src.exists() {
            let dst = dst_dir.join(src.file_name().unwrap());
            if !dst.exists() {
                let _ = fs::copy(src, &dst);
                println!("cargo:warning=Copied {:?} to {:?}", src, dst);
            }
        }
    };
    
    // Copy essential CEF files
    copy_file(&release_dir.join("libcef.so"), &target_dir);
    copy_file(&cef_path.join("icudtl.dat"), &target_dir);
    copy_file(&cef_path.join("chrome_100_percent.pak"), &target_dir);
    copy_file(&cef_path.join("chrome_200_percent.pak"), &target_dir);
    copy_file(&cef_path.join("resources.pak"), &target_dir);
    copy_file(&cef_path.join("snapshot_blob.bin"), &target_dir);
    copy_file(&cef_path.join("v8_context_snapshot.bin"), &target_dir);
    
    if let Ok(entries) = fs::read_dir(cef_path.join("locales")) {
        let locales_dir = target_dir.join("locales");
        let _ = fs::create_dir_all(&locales_dir);
        for entry in entries.flatten() {
            copy_file(&entry.path(), &locales_dir);
        }
    }
    
    // Copy chrome-sandbox if present (requires SUID)
    let chrome_sandbox = cef_path.join("chrome-sandbox");
    if chrome_sandbox.exists() {
        let dst = target_dir.join("chrome-sandbox");
        if !dst.exists() {
            let _ = fs::copy(&chrome_sandbox, &dst);
        }
    }
}
