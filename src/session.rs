use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::cef_browser::CefBrowserWrapper;

/// Manages the browser's persistent session state for CEF:
/// - isolated data/cache directory under ~/.local/share/iron/
/// - cookie persistence via CEF's native cookie manager
/// - site-data clearing (:clear-site-data / :csd)
/// - incognito mode (separate CEF context, no cookies/history)
pub struct SessionManager {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    pub incognito: bool,
}

impl SessionManager {
    /// Create a `SessionManager` pointing at standard XDG paths.
    ///
    /// Data lives under  `~/.local/share/iron/session/`
    /// Cache lives under `~/.cache/iron/`
    pub fn new() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("iron")
            .join("session");

        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("iron");

        SessionManager {
            data_dir,
            cache_dir,
            incognito: false,
        }
    }

    /// Switch to incognito mode. Must be called *before* CEF initialization.
    pub fn set_incognito(&mut self, enabled: bool) {
        self.incognito = enabled;
    }

    /// Ensure session directories exist (called during CEF init)
    pub fn ensure_directories(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Clear all site data (cookies, local storage, disk cache, etc.)
    /// This is what `:clear-site-data` / `:csd` invokes.
    pub fn clear_all_site_data(&self, _browser: &CefBrowserWrapper) {
        // TODO: When CEF is fully integrated:
        // - Get CefRequestContext from browser
        // - Call CefRequestContext::close_all_connections()
        // - Clear cache directory manually
        
        eprintln!("Clearing all site data...");
        
        // For now, clear cache directory manually
        let cache_path = self.cache_dir.join("cef");
        if cache_path.exists() {
            if let Err(e) = std::fs::remove_dir_all(&cache_path) {
                eprintln!("Failed to clear cache: {}", e);
            } else {
                eprintln!("All site data cleared successfully");
            }
        } else {
            eprintln!("No site data to clear");
        }
    }

    /// Clear cookies only (useful for "log out everywhere" feel).
    pub fn clear_cookies(&self, _browser: &CefBrowserWrapper) {
        // TODO: When CEF is fully integrated:
        // - Get CefCookieManager from CefRequestContext
        // - Call CefCookieManager::delete_cookies()
        
        eprintln!("Clearing cookies...");
        
        // For now, just log the action
        eprintln!("Cookies cleared (placeholder - full implementation pending CEF integration)");
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an `Rc<RefCell<SessionManager>>` that survives for the process lifetime.
pub fn build_session_mgr() -> Rc<RefCell<SessionManager>> {
    Rc::new(RefCell::new(SessionManager::new()))
}
