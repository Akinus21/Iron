//! Build script for CEF (Chromium Embedded Framework) integration
//!
//! CEF is downloaded by cef-dll-sys during its build. We find and link it.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CEF_TRACK");
    println!("cargo:rerun-if-env-changed=CEF_DIR");

    let cef_dir = if let Ok(dir) = env::var("CEF_DIR") {
        PathBuf::from(&dir)
    } else {
        // CEF is downloaded by cef-dll-sys into its build output
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let target_dir = out_dir
            .parent().unwrap()
            .parent().unwrap()
            .parent().unwrap();

        let mut found = None;

        // Search cef-dll-sys build output directories
        let build_dir = target_dir.join("build");
        if let Ok(entries) = fs::read_dir(&build_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("cef-dll-sys-") {
                    let out_path = entry.path().join("out");
                    // cef-dll-sys extracts to out/ or out/cef_binary_.../
                    if out_path.join("Release").exists() {
                        found = Some(out_path);
                        break;
                    }
                    // Check one level deeper
                    if let Ok(subdirs) = fs::read_dir(&out_path) {
                        for sub in subdirs.flatten() {
                            let sub_path = sub.path();
                            if sub_path.join("Release").exists() {
                                found = Some(sub_path);
                                break;
                            }
                        }
                    }
                    if found.is_some() { break; }
                }
            }
        }

        match found {
            Some(dir) => {
                eprintln!("Found CEF at: {}", dir.display());
                dir
            }
            None => {
                eprintln!("CEF not found. Searched in: {}/build/cef-dll-sys-*/out/", target_dir.display());
                eprintln!("Please set CEF_DIR to the CEF binary directory.");
                std::process::exit(1);
            }
        }
    };

    setup_cef_paths(&cef_dir);
}

fn setup_cef_paths(cef_path: &Path) {
    let release_dir = cef_path.join("Release");
    
    if release_dir.exists() {
        println!("cargo:rustc-link-search=native={}", release_dir.display());
    } else {
        println!("cargo:rustc-link-search=native={}", cef_path.display());
    }
    
    println!("cargo:rustc-link-lib=dylib=cef");

    // Copy CEF resources to target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_base = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());

    let target_dir = target_base
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("target"));

    let copy_file = |src: &Path, dst_dir: &Path| {
        if src.exists() {
            let dst = dst_dir.join(src.file_name().unwrap());
            if !dst.exists() {
                let _ = fs::copy(src, &dst);
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
}
