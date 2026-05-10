use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cef::{App, CefString, CommandLine, ImplApp, ImplCommandLine, MainArgs, Settings};
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
                cmd.append_switch(Some(&CefString::from("in-process-gpu")));
                cmd.append_switch(Some(&CefString::from("disable-dev-shm-usage")));
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
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("libcef.so").exists() {
                return Some(parent.to_path_buf());
            }
            if parent.join("lib").join("libcef.so").exists() {
                return Some(parent.join("lib"));
            }
            if let Some(grandparent) = parent.parent() {
                if grandparent.join("lib").join("libcef.so").exists() {
                    return Some(grandparent.join("lib"));
                }
            }
        }
    }
    if let Ok(ld_path) = std::env::var("LD_LIBRARY_PATH") {
        for dir in ld_path.split(':') {
            let path = PathBuf::from(dir);
            if path.join("libcef.so").exists() {
                return Some(path);
            }
        }
    }
    for dir in &[
        PathBuf::from("/usr/local/lib"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/lib"),
        PathBuf::from("/usr/lib"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu"),
    ] {
        if dir.join("libcef.so").exists() {
            return Some(dir.clone());
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

    let mut main_args = MainArgs::default();
    main_args.argc = c_args.len() as i32;
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

    let mut owned: Vec<CString> = Vec::with_capacity(args.len());
    for arg in &args {
        owned.push(CString::new(arg.as_str()).unwrap_or_else(|_| CString::new("").unwrap()));
    }
    let mut c_args: Vec<*mut std::os::raw::c_char> =
        owned.iter_mut().map(|c| c.as_ptr() as *mut std::os::raw::c_char).collect();

    let mut main_args = MainArgs::default();
    main_args.argc = c_args.len() as i32;
    main_args.argv = c_args.as_mut_ptr();
    (owned, c_args, main_args)
}

pub fn execute_subprocess() -> Option<i32> {
    let is_cef_subprocess = std::env::args().any(|arg| arg.starts_with("--type="));
    let (_owned, _c_args, main_args) = build_raw_main_args();
    let mut app = IronApp::new();
    let exit_code = cef::execute_process(Some(&main_args), Some(&mut app), std::ptr::null_mut());
    if exit_code >= 0 {
        return Some(exit_code);
    }
    if is_cef_subprocess {
        eprintln!("[CEF] Subprocess was not handled by execute_process; exiting to avoid GTK arg parsing");
        return Some(0);
    }
    None
}

pub fn initialize_cef(config: &CefConfig) -> Result<(), String> {
    if CEF_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    let _ = std::fs::create_dir_all(&config.cache_path);

    eprintln!(
        "[CEF] Initializing (track={}, cache={:?}, osr={})",
        config.track, config.cache_path, config.windowless_rendering
    );

    let (_owned, _c_args, main_args) = build_main_args();

    let mut settings = Settings::default();
    settings.no_sandbox = 1;
    settings.windowless_rendering_enabled = if config.windowless_rendering { 1 } else { 0 };
    settings.external_message_pump = 1;
    settings.multi_threaded_message_loop = 0;

    let cache_path_str = config.cache_path.to_string_lossy();
    settings.cache_path = CefString::from(cache_path_str.as_ref());

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            settings.browser_subprocess_path = CefString::from(exe_str);
        }
    }

    let mut resources_set = false;
    let mut locales_set = false;

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let share_iron = exe_dir
                .parent()
                .map(|p| p.join("share").join("iron"))
                .unwrap_or_else(PathBuf::new);
            if share_iron.exists() {
                if let Some(res_str) = share_iron.to_str() {
                    settings.resources_dir_path = CefString::from(res_str);
                    eprintln!("[CEF] Resources dir: {}", res_str);
                    resources_set = true;
                }
                let locales = share_iron.join("locales");
                if locales.exists() {
                    if let Some(loc_str) = locales.to_str() {
                        settings.locales_dir_path = CefString::from(loc_str);
                        eprintln!("[CEF] Locales dir: {}", loc_str);
                        locales_set = true;
                    }
                }
            }
            if !resources_set {
                let res_dir = exe_dir.join("res");
                if res_dir.exists() {
                    if let Some(res_str) = res_dir.to_str() {
                        settings.resources_dir_path = CefString::from(res_str);
                        eprintln!("[CEF] Resources dir: {}", res_str);
                    }
                }
            }
            if !locales_set {
                let locales_dir = exe_dir.join("locales");
                if locales_dir.exists() {
                    if let Some(loc_str) = locales_dir.to_str() {
                        settings.locales_dir_path = CefString::from(loc_str);
                        eprintln!("[CEF] Locales dir: {}", loc_str);
                    }
                }
            }
        }
    }

    if let Some(cef_dir) = find_cef_dir() {
        if !locales_set {
            let cef_locales = cef_dir.join("locales");
            if cef_locales.exists() {
                if let Some(loc_str) = cef_locales.to_str() {
                    settings.locales_dir_path = CefString::from(loc_str);
                    eprintln!("[CEF] Locales dir: {}", loc_str);
                }
            }
        }
    }

    let mut app = IronApp::new();
    let result = cef::initialize(
        Some(&main_args),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    );

    if result != 0 {
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
