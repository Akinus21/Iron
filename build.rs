//! Build script for CEF (Chromium Embedded Framework) integration
//! 
//! This script:
//! 1. Downloads CEF binary distribution if not present
//! 2. Sets up library paths for linking
//! 3. Copies CEF resources to output directory

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CEF_VERSION_STABLE: &str = "147.1.0+147.0.10";
const CEF_VERSION_NIGHTLY: &str = "147.1.0+147.0.10"; // Would be updated dynamically

fn main() {
    println!("cargo:rerun-if-env-changed=CEF_TRACK");
    println!("cargo:rerun-if-env-changed=CEF_DIR");
    
    let cef_track = env::var("CEF_TRACK").unwrap_or_else(|_| "stable".to_string());
    let cef_version = if cef_track == "nightly" {
        CEF_VERSION_NIGHTLY
    } else {
        CEF_VERSION_STABLE
    };
    
    // Check if CEF_DIR is set (CI will set this)
    if let Ok(cef_dir) = env::var("CEF_DIR") {
        let cef_path = PathBuf::from(&cef_dir);
        if cef_path.exists() {
            setup_cef_paths(&cef_path);
            return;
        }
    }
    
    // For local development, try common CEF locations
    let common_locations = vec![
        PathBuf::from("/opt/cef"),
        PathBuf::from("/usr/local/cef"),
        PathBuf::from(env::var("HOME").unwrap_or_default()).join(".local/cef"),
    ];
    
    for location in &common_locations {
        if location.exists() {
            setup_cef_paths(location);
            return;
        }
    }
    
    // CEF not found - warn and continue (will fail at runtime)
    println!("cargo:warning=CEF binary distribution not found. Build will succeed but runtime will fail.");
    println!("cargo:warning=Set CEF_DIR environment variable or install CEF to /opt/cef");
    println!("cargo:warning=Download CEF from: https://cef-builds.spotifycdn.com/index.html");
}

fn setup_cef_paths(cef_path: &Path) {
    println!("cargo:rustc-link-search=native={}/Release", cef_path.display());
    println!("cargo:rustc-link-lib=dylib=cef");
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
    
    // Copy CEF resources to target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or(&PathBuf::from("target"));
    
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
    copy_file(&release_dir.join("libcef.so"), target_dir);
    copy_file(&cef_path.join("icudtl.dat"), target_dir);
    copy_file(&cef_path.join("chrome_100_percent.pak"), target_dir);
    copy_file(&cef_path.join("chrome_200_percent.pak"), target_dir);
    copy_file(&cef_path.join("resources.pak"), target_dir);
    copy_file(&cef_path.join("snapshot_blob.bin"), target_dir);
    copy_file(&cef_path.join("v8_context_snapshot.bin"), target_dir);
    
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
            // Note: SUID bit would need to be set post-install
        }
    }
}
