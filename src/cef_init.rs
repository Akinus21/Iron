use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cef::{App, CefString, CommandLine, ImplApp, MainArgs, Settings, WrapApp};
use cef::rc::Rc;

static CEF_INITIALIZED: AtomicBool = AtomicBool::new(false);
static CEF_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

cef::wrap_app! {
    pub struct IronApp {}
    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            if let Some(cmd) = command_line {
                cmd.append_switch(Some(&CefString::from("no-sandbox")));
                cmd.append_switch(Some(&CefString::from("disable-gpu")));
                cmd.append_switch(Some(&CefString::from("disable-gpu-compositing")));
                cmd.append_switch(Some(&CefString::from("disable-dev-shm-usage")));
                cmd.append_switch(Some(&CefString::from("no-zygote")));
                cmd.append_switch(Some(&CefString::from("in-process-gpu")));
                cmd.append_switch(Some(&CefString::from("renderer-process-limit=1")));
                cmd.append_switch(Some(&CefString::from("disable-site-isolation-trials")));
                cmd.append_switch(Some(&CefString::from("disable-features=IsolateOrigins,site-per-process,SitePerProcess")));
            }
        }
    }
}

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

fn find_cef_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("IRON_CEF_RUNTIME_DIR") {
        let path = PathBuf::from(&dir);
        if path.join("libcef.so").exists() {
            eprintln!("[CEF] Found CEF dir via IRON_CEF_RUNTIME_DIR: {}", dir);
            return Some(path);
        }
    }

    let exe = std::fs::read_link("/proc/self/exe")
        .or_else(|_| std::env::current_exe())
        .ok()?;
    let exe_dir = exe.parent()?;

    if exe_dir.join("libcef.so").exists() {
        eprintln!("[CEF] Found CEF dir next to exe: {}", exe_dir.display());
        return Some(exe_dir.to_path_buf());
    }

    if let Some(prefix) = exe_dir.parent() {
        let homebrew_cef = prefix.join("libexec").join("cef-runtime");
        if homebrew_cef.join("libcef.so").exists() {
            eprintln!("[CEF] Found CEF dir via Homebrew layout: {}", homebrew_cef.display());
            return Some(homebrew_cef);
        }
    }

    if exe_dir.join("lib").join("libcef.so").exists() {
        return Some(exe_dir.join("lib"));
    }

    if let Ok(ld_path) = std::env::var("LD_LIBRARY_PATH") {
        for dir in ld_path.split(':') {
            let path = PathBuf::from(dir);
            if path.join("libcef.so").exists() {
                eprintln!("[CEF] Found CEF dir via LD_LIBRARY_PATH: {}", path.display());
                return Some(path);
            }
        }
    }

    None
}

fn build_raw_main_args() -> (Vec<CString>, Vec<*mut std::os::raw::c_char>, MainArgs) {
    let mut args: Vec<String> = std::env::args().collect();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            if args.is_empty() {
                args.push(exe_str.to_string());
            } else {
                args[0] = exe_str.to_string();
            }
        }
    }

    let mut owned: Vec<CString> = Vec::with_capacity(args.len());
    for arg in &args {
        owned.push(CString::new(arg.as_str()).unwrap_or_else(|_| CString::new("").unwrap()));
    }
    let mut c_args: Vec<*mut std::os::raw::c_char> =
        owned.iter_mut().map(|c| c.as_ptr() as *mut std::os::raw::c_char).collect();
    c_args.push(std::ptr::null_mut());

    let mut main_args = MainArgs::default();
    main_args.argc = owned.len() as i32;
    main_args.argv = c_args.as_mut_ptr();
    (owned, c_args, main_args)
}

fn build_main_args() -> (Vec<CString>, Vec<*mut std::os::raw::c_char>, MainArgs) {
    let mut args: Vec<String> = std::env::args().collect();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            if args.is_empty() {
                args.push(exe_str.to_string());
            } else {
                args[0] = exe_str.to_string();
            }
        }
    }

    for flag in [
        "--single-process",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-gpu-compositing",
        "--disable-vulkan",
        "--disable-features=Vulkan",
        "--disable-dev-shm-usage",
    ] {
        if !args.iter().any(|arg| arg == flag) {
            args.push(flag.to_string());
        }
    }

    let mut owned: Vec<CString> = Vec::with_capacity(args.len());
    for arg in &args {
        owned.push(CString::new(arg.as_str()).unwrap_or_else(|_| CString::new("").unwrap()));
    }
    let mut c_args: Vec<*mut std::os::raw::c_char> =
        owned.iter_mut().map(|c| c.as_ptr() as *mut std::os::raw::c_char).collect();
    c_args.push(std::ptr::null_mut());

    let mut main_args = MainArgs::default();
    main_args.argc = owned.len() as i32;
    main_args.argv = c_args.as_mut_ptr();
    (owned, c_args, main_args)
}

pub fn execute_subprocess() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    eprintln!("[CEF] execute_subprocess called with args: {:?}", args);

    let exit_code = cef::execute_process(None, None, std::ptr::null_mut());
    eprintln!("[CEF] execute_process returned: {}", exit_code);

    if exit_code >= 0 {
        Some(exit_code)
    } else {
        None
    }
}

pub fn initialize_cef(config: &CefConfig) -> Result<(), String> {
    if CEF_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    let _ = std::fs::create_dir_all(&config.cache_path);

    let raw_args: Vec<String> = std::env::args().collect();
    eprintln!("[CEF] Raw args: {:?}", raw_args);

    let mut args: Vec<String> = raw_args.into_iter()
        .filter(|arg| !arg.starts_with("--type="))
        .collect();

    for flag in &[
        "--no-sandbox",
        "--disable-gpu",
        "--disable-gpu-compositing",
        "--disable-dev-shm-usage",
    ] {
        if !args.iter().any(|a| a == flag) {
            args.push(flag.to_string());
        }
    }

    eprintln!(
        "[CEF] Initializing (track={}, cache={:?}, osr={})",
        config.track, config.cache_path, config.windowless_rendering
    );
    eprintln!("[CEF] Filtered args: {:?}", args);

    let mut owned: Vec<CString> = Vec::with_capacity(args.len());
    for arg in &args {
        owned.push(CString::new(arg.as_str()).unwrap_or_else(|_| CString::new("").unwrap()));
    }
    let mut c_args: Vec<*mut std::os::raw::c_char> =
        owned.iter_mut().map(|c| c.as_ptr() as *mut std::os::raw::c_char).collect();
    c_args.push(std::ptr::null_mut());
    let mut main_args = MainArgs::default();
    main_args.argc = owned.len() as i32;
    main_args.argv = c_args.as_mut_ptr();

    let mut settings = Settings::default();
    settings.no_sandbox = 1;
    settings.windowless_rendering_enabled = 0;
    settings.external_message_pump = 1;
    settings.multi_threaded_message_loop = 1;
    settings.multi_threaded_message_loop = 0;
    settings.persist_session_cookies = 0;

let cache_path_str = config.cache_path.to_string_lossy();
    settings.cache_path = CefString::from(cache_path_str.as_ref());

    if let Some(cef_dir) = find_cef_dir() {
        settings.resources_dir_path = CefString::from(cef_dir.to_string_lossy().as_ref());
        eprintln!("[CEF] Resources dir: {}", cef_dir.display());
    }

    let mut app = IronApp::new();
    eprintln!("[CEF] Calling cef::initialize");
    let result = cef::initialize(
        Some(&main_args),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    );

    if result == 0 {
        return Err(format!("CEF initialization failed (result={})", result));
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
