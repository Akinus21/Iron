use std::cell::RefCell;
use std::rc::Rc;

use gio::{Notification, prelude::*};

use crate::cef_browser::CefBrowserWrapper;

pub struct DownloadItem {
    pub filename: String,
    pub path: String,
    pub done: bool,
    pub failed: bool,
    pub progress: f64,
}

pub struct DownloadManager {
    pub items: Vec<DownloadItem>,
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager { items: Vec::new() }
    }

    /// Attach download handler to CEF browser
    /// 
    /// Note: Full CEF download implementation requires CefDownloadHandler
    /// This is a placeholder for now
    pub fn attach(_browser: &CefBrowserWrapper, _mgr: Rc<RefCell<DownloadManager>>) {
        // TODO: When CEF is fully integrated:
        // - Implement CefDownloadHandler trait
        // - Set handler via CefClient
        // - Handle OnBeforeDownload, OnDownloadUpdated, OnDownloadStateChanged
        
        eprintln!("Download handler attached (placeholder - full implementation pending CEF integration)");
    }

    pub fn recent(&self, limit: usize) -> Vec<&DownloadItem> {
        self.items
            .iter()
            .rev()
            .filter(|i| i.done && !i.failed)
            .take(limit)
            .collect()
    }
}

fn notify_download_complete(filename: &str, path: &str) {
    let app = gio::Application::default();
    let Some(ref app) = app else {
        return;
    };

    let notif = Notification::new(&format!("Download complete: {}", filename));
    notif.set_body(Some(&format!("Saved to {}", path)));
    notif.set_priority(gio::NotificationPriority::Normal);

    let folder = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let target = glib::Variant::from(folder.as_str());
    notif.add_button_with_target_value(
        "Open folder",
        "app.open-folder",
        Some(&target),
    );

    app.send_notification(Some("iron-download"), &notif);
}

pub fn open_folder(path: &str) {
    let folder = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let _ = std::process::Command::new("xdg-open")
        .arg(&folder)
        .spawn();
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            _ => c,
        })
        .collect()
}

fn uniquify(path: &std::path::Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(std::path::Path::new("."));

    for n in 1..=9999 {
        let candidate = parent.join(format!("{} ({}){}", stem, n, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}
