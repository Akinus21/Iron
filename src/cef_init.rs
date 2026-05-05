//! CEF (Chromium Embedded Framework) initialization
//! 
//! This module handles CEF lifecycle management.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Global CEF initialization state
static CEF_INITIALIZED: AtomicBool = AtomicBool::new(false);
static CEF_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

/// CEF configuration
#[derive(Debug, Clone)]
pub struct CefConfig {
    pub track: String,  // "stable" or "nightly"
    pub cache_path: PathBuf,
    pub log_level: String,
    pub enable_window_sleep: bool,
}

impl Default for CefConfig {
    fn default() -> Self {
        Self {
            track: "stable".to_string(),
            cache_path: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("iron")
                .join("cef"),
            log_level: "info".to_string(),
            enable_window_sleep: true,
        }
    }
}

/// Initialize CEF before creating browsers
pub fn initialize_cef(config: &CefConfig) -> Result<(), String> {
    if CEF_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    // Ensure cache directory exists
    let _ = std::fs::create_dir_all(&config.cache_path);
    
    // Log initialization
    eprintln!("[CEF] Initializing (track={}, cache={:?})", 
              config.track, config.cache_path);
    
    // Note: Actual CEF initialization requires FFI calls to CefInitialize
    // This is a placeholder until full CEF FFI is implemented
    
    CEF_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
    CEF_INITIALIZED.store(true, Ordering::SeqCst);
    
    Ok(())
}

/// Shutdown CEF when application closes
pub fn shutdown_cef() {
    if !CEF_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }
    
    let count = CEF_INIT_COUNT.fetch_sub(1, Ordering::SeqCst);
    if count == 1 {
        eprintln!("[CEF] Shutting down");
        // Note: Actual CEF shutdown requires FFI call to CefShutdown
        CEF_INITIALIZED.store(false, Ordering::SeqCst);
    }
}

/// Check if CEF is initialized
pub fn is_cef_initialized() -> bool {
    CEF_INITIALIZED.load(Ordering::SeqCst)
}

/// Get CEF command-line flags for resource conservation
pub fn get_cef_flags() -> Vec<String> {
    vec![
        "--disable-gpu".to_string(),
        "--disable-gpu-compositing".to_string(),
        "--disable-extensions".to_string(),
        "--disable-background-networking".to_string(),
        "--disable-background-timer-throttling".to_string(),
        "--disable-backgrounding-occluded-windows".to_string(),
        "--disable-renderer-backgrounding".to_string(),
        "--disable-dev-shm-usage".to_string(),
        "--max-old-space-size=4096".to_string(),
        "--enable-features=AutomaticTabDiscarding".to_string(),
        "--disable-component-update".to_string(),
        "--disable-default-apps".to_string(),
    ]
}
