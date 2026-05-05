use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use webkit6::prelude::*;
use webkit6::{
    CookieAcceptPolicy, CookiePersistentStorage, NetworkSession, WebsiteDataTypes,
};

/// Manages the browser's persistent session state:
/// - isolated data/cache directory under ~/.local/share/iron/
/// - SQLite-backed cookie jar with NO_THIRD_PARTY policy
/// - persistent credential storage (WebKit's native keyring-backed store)
/// - site-data clearing (:clear-site-data / :csd)
/// - incognito mode (ephemeral NetworkSession, no cookies/history)
pub struct SessionManager {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    cookie_file: PathBuf,
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

        let cookie_file = data_dir.join("cookies.sqlite");

        SessionManager {
            data_dir,
            cache_dir,
            cookie_file,
            incognito: false,
        }
    }

    /// Switch to incognito mode. Must be called *before* any `NetworkSession`
    /// is instantiated (i.e. before the first `WebView` is created).
    pub fn set_incognito(&mut self, enabled: bool) {
        self.incognito = enabled;
    }

    /// Build a `NetworkSession` for the current configuration.
    ///
    /// * Normal mode  → persistent `NetworkSession::new(data_dir, cache_dir)`
    /// * Incognito    → ephemeral `NetworkSession::new_ephemeral()`
    pub fn build_network_session(&self) -> NetworkSession {
        if self.incognito {
            return NetworkSession::new_ephemeral();
        }

        // Ensure directories exist
        let _ = std::fs::create_dir_all(&self.data_dir);
        let _ = std::fs::create_dir_all(&self.cache_dir);

        let data = self.data_dir.to_str();
        let cache = self.cache_dir.to_str();

        NetworkSession::new(data, cache)
    }

    /// Wire up cookie persistence, accept-policy, and credential storage.
    /// Call once per `WebView` after its `NetworkSession` exists.
    pub fn configure_session(&self, webview: &webkit6::WebView) {
        let Some(session) = webview.network_session() else {
            eprintln!("SessionManager: WebView has no NetworkSession");
            return;
        };

        if self.incognito {
            // Ephemeral session: nothing to persist, credentials disabled
            session.set_persistent_credential_storage_enabled(false);
            return;
        }

        // ---- Cookies ----
        if let Some(cm) = session.cookie_manager() {
            if let Some(path) = self.cookie_file.to_str() {
                cm.set_persistent_storage(path, CookiePersistentStorage::Sqlite);
            }
            // Reject third-party cookies outright (privacy by default)
            cm.set_accept_policy(CookieAcceptPolicy::OnlyFromMainDocumentDomain);
        }

        // ---- Credentials ----
        // Enable WebKit's native persistent credential storage.
        // On BlueAK/Noctalia this stores HTTP-auth secrets in the user's
        // default Secret Service keyring (gnome-keyring / KDE Wallet / keepassxc-secret-service).
        session.set_persistent_credential_storage_enabled(true);
    }

    /// Clear all site data (cookies, local storage, disk cache, IndexedDB,
    /// service workers, HSTS cache, DOM cache, ITP data) for the current session.
    /// This is what `:clear-site-data` / `:csd` invokes.
    pub fn clear_all_site_data(&self, webview: &webkit6::WebView) {
        let Some(session) = webview.network_session() else {
            eprintln!("SessionManager: no NetworkSession to clear");
            return;
        };

        let Some(wdm) = session.website_data_manager() else {
            eprintln!("SessionManager: no WebsiteDataManager to clear");
            return;
        };

        // Wipe everything back to Unix epoch (i.e. all time).
        let types = WebsiteDataTypes::ALL;
        let timespan = glib::TimeSpan::from_seconds(0);

        wdm.clear(
            types,
            timespan,
            None::<&gio::Cancellable>,
            |result| match result {
                Ok(()) => eprintln!("All site data cleared successfully"),
                Err(e) => eprintln!("Failed to clear site data: {}", e),
            },
        );
    }

    /// Clear cookies only (useful for "log out everywhere" feel).
    pub fn clear_cookies(&self, webview: &webkit6::WebView) {
        let Some(session) = webview.network_session() else { return };
        let Some(wdm) = session.website_data_manager() else { return };

        wdm.clear(
            WebsiteDataTypes::COOKIES,
            glib::TimeSpan::from_seconds(0),
            None::<&gio::Cancellable>,
            |result| match result {
                Ok(()) => eprintln!("Cookies cleared"),
                Err(e) => eprintln!("Failed to clear cookies: {}", e),
            },
        );
    }

    /// Convenience: path to the cookie database (for debugging / inspection).
    pub fn cookie_path(&self) -> &std::path::Path {
        &self.cookie_file
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
