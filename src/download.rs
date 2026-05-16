use std::cell::RefCell;
use std::rc::Rc;

use gio::{Notification, prelude::*};
use webkit6::{Download, NetworkSession};

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

    /// Attach to the `NetworkSession` to receive download-started signals.
    pub fn attach(
        session: &NetworkSession,
        mgr: Rc<RefCell<DownloadManager>>,
        browser: &crate::webkit_browser::WebKitBrowserWrapper,
    ) {
        let mgr_clone = mgr.clone();
        let browser_clone = browser.clone();
        session.connect_download_started(move |_session, dl| {
            browser_clone.go_back();
            DownloadManager::handle_download(dl, mgr_clone.clone());
        });
        eprintln!("Download handler attached");
    }

    fn handle_download(dl: &Download, mgr: Rc<RefCell<DownloadManager>>) {
        eprintln!("[Download] handle_download called for {}", dl.request().and_then(|r| r.uri()).map(|u| u.to_string()).unwrap_or_default());
        dl.set_allow_overwrite(true);

        let mgr_decide = mgr.clone();
        dl.connect_decide_destination(move |dl, suggested| {
            eprintln!("[Download] decide_destination: suggested={}", suggested);
            let filename = sanitize_filename(suggested);
            let downloads = dirs::download_dir()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let dest = downloads.join(&filename);
            let dest = uniquify(&dest);
            let dest_str = dest.to_string_lossy().to_string();
            dl.set_destination(&dest_str);
            eprintln!("[Download] destination set to: {}", dest_str);
            
            let item = DownloadItem {
                filename: filename.clone(),
                path: dest_str,
                done: false,
                failed: false,
                progress: 0.0,
            };
            mgr_decide.borrow_mut().items.push(item);
            true
        });

        let mgr_progress = mgr.clone();
        dl.connect_estimated_progress_notify(move |dl| {
            let progress = dl.estimated_progress();
            eprintln!("[Download] progress: {:.2}%", progress * 100.0);
            if let Some(item) = mgr_progress.borrow_mut().items.last_mut() {
                item.progress = progress;
            }
        });

        let mgr_finished = mgr.clone();
        dl.connect_finished(move |dl| {
            eprintln!("[Download] finished");
            if let Some(item) = mgr_finished.borrow_mut().items.last_mut() {
                item.done = true;
                item.progress = 1.0;
                let path = item.path.clone();
                let filename = item.filename.clone();
                notify_download_complete(&filename, &path);
            }
            let _ = dl;
        });

        let mgr_failed = mgr.clone();
        dl.connect_failed(move |_dl, error| {
            eprintln!("[Download] failed: {}", error);
            if let Some(item) = mgr_failed.borrow_mut().items.last_mut() {
                item.failed = true;
            }
        });
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