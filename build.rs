//! Build script for Iron
//!
//! Passes through CEF library configuration from cef-dll-sys

fn main() {
    // Tell cargo to rerun if these env vars change
    println!("cargo:rerun-if-env-changed=CEF_DIR");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");

    // Let cef-dll-sys handle CEF download and linking
    // Our build script just ensures resources are available
}