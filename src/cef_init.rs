use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cef::{CefArgs, Settings, App};

static CEF_INITIALIZED: AtomicBool = AtomicBool::new(false);
static CEF_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct CefConfig {
    pub track: String,
    pub cache_path: PathBuf,
    pub log_level: String,
    pub enable_window_sleep: bool,
    pub windowless_rendering: bool,
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
            windowless_rendering: true,
        }
    }
}

pub fn initialize_cef(config: &CefConfig) -> Result<(), String> {
    if CEF_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    let _ = std::fs::create_dir_all(&config.cache_path);

    eprintln!("[CEF] Initializing (track={}, cache={:?}, osr={})",
              config.track, config.cache_path, config.windowless_rendering);

    let args: Vec<String> = std::env::args().collect();
    let mut cef_args = CefArgs::new();
    for arg in &args {
        cef_args.push(arg);
    }
    cef_args.push("--disable-gpu");
    cef_args.push("--disable-gpu-compositing");
    cef_args.push("--disable-extensions");
    cef_args.push("--disable-background-networking");
    cef_args.push("--disable-background-timer-throttling");
    cef_args.push("--disable-backgrounding-occluded-windows");
    cef_args.push("--disable-renderer-backgrounding");
    cef_args.push("--disable-dev-shm-usage");
    cef_args.push("--enable-features=AutomaticTabDiscarding");
    cef_args.push("--disable-component-update");
    cef_args.push("--disable-default-apps");

    if config.windowless_rendering {
        cef_args.push("--off-screen-rendering-enabled");
        cef_args.push("--enable-gpu");
    }

    let mut settings = Settings::default();
    settings.windowless_rendering_enabled = if config.windowless_rendering { 1 } else { 0 };
    settings.external_message_pump = 1;
    settings.multi_threaded_message_loop = 0;

    let cache_path_str = config.cache_path.to_string_lossy();
    settings.cache_path = Some(cef::CefString::from(cache_path_str.as_ref()));

    let result = cef::initialize(
        Some(cef_args.as_main_args()),
        Some(&settings),
        None::<App>,
        std::ptr::null_mut(),
    );

    if result != 0 {
        return Err("CEF subprocess handler".to_string());
    }

    CEF_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
    CEF_INITIALIZED.store(true, Ordering::SeqCst);

    eprintln!("[CEF] Initialized successfully");
    Ok(())
}

pub fn shutdown_cef() {
    if !CEF_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    let count = CEF_INIT_COUNT.fetch_sub(1, Ordering::SeqCst);
    if count == 1 {
        eprintln!("[CEF] Shutting down");
        cef::shutdown();
        CEF_INITIALIZED.store(false, Ordering::SeqCst);
    }
}

pub fn is_cef_initialized() -> bool {
    CEF_INITIALIZED.load(Ordering::SeqCst)
}

pub fn do_message_loop_work() {
    if CEF_INITIALIZED.load(Ordering::SeqCst) {
        cef::do_message_loop_work();
    }
}

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