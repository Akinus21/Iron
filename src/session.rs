use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use webkit6::{NetworkSession, WebsiteDataTypes};

/// Manages the browser's persistent session state for WebKit2GTK:
/// - isolated data/cache directory under ~/.local/share/iron/
/// - cookie persistence via WebKit's native cookie manager
/// - site-data clearing (:clear-site-data / :csd)
/// - incognito mode (ephemeral NetworkSession, no cookies/history)
pub struct SessionManager {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    network_session: NetworkSession,
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

        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&cache_dir);

        let data_dir_str = data_dir.to_str().map(|s| s.to_string());
        let cache_dir_str = cache_dir.to_str().map(|s| s.to_string());

        let network_session = NetworkSession::new(
            data_dir_str.as_deref(),
            cache_dir_str.as_deref(),
        );
        // Note: persistent credential storage requires a running secret service
        // (e.g. gnome-keyring). If unavailable, WebKit logs a harmless warning.
        // We leave it enabled so HTTP auth and form passwords are remembered.

        SessionManager {
            data_dir,
            cache_dir,
            network_session,
            incognito: false,
        }
    }

    /// Switch to incognito mode. Must be called *before* WebView creation.
    pub fn set_incognito(&mut self, enabled: bool) {
        self.incognito = enabled;
    }

    /// Ensure session directories exist.
    pub fn ensure_directories(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Access the underlying `NetworkSession` (for WebView creation).
    pub fn network_session(&self) -> &NetworkSession {
        &self.network_session
    }

    /// Clone the underlying `NetworkSession` (cheap GObject ref-count clone).
    pub fn network_session_clone(&self) -> NetworkSession {
        self.network_session.clone()
    }

    /// Clear all site data (cookies, local storage, disk cache, etc.)
    /// This is what `:clear-site-data` / `:csd` invokes.
    pub fn clear_all_site_data(&self, _browser: &crate::webkit_browser::WebKitBrowserWrapper) {
        let manager = self.network_session.website_data_manager();
        if let Some(ref manager) = manager {
            let types = WebsiteDataTypes::ALL;
            let timespan = glib::TimeSpan::from_seconds(0);
            manager.clear(
                types,
                timespan,
                None::<&gio::Cancellable>,
                |result| {
                    match result {
                        Ok(()) => eprintln!("All site data cleared successfully"),
                        Err(e) => eprintln!("Failed to clear site data: {}", e),
                    }
                },
            );
            eprintln!("Clearing all site data...");
        } else {
            eprintln!("No website data manager available");
        }
    }

    /// Clear cookies only (useful for "log out everywhere" feel).
    pub fn clear_cookies(&self, _browser: &crate::webkit_browser::WebKitBrowserWrapper) {
        let manager = self.network_session.website_data_manager();
        if let Some(ref manager) = manager {
            let types = WebsiteDataTypes::COOKIES;
            let timespan = glib::TimeSpan::from_seconds(0);
            manager.clear(
                types,
                timespan,
                None::<&gio::Cancellable>,
                |result| {
                    match result {
                        Ok(()) => eprintln!("Cookies cleared successfully"),
                        Err(e) => eprintln!("Failed to clear cookies: {}", e),
                    }
                },
            );
            eprintln!("Clearing cookies...");
        } else {
            eprintln!("No website data manager available");
        }
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
