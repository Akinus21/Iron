use std::cell::RefCell;
use std::rc::Rc;

use gio::{Notification, prelude::*};
use webkit6::prelude::*;

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

    pub fn attach(view: &webkit6::WebView, mgr: Rc<RefCell<DownloadManager>>) {
        let Some(session) = view.network_session() else { return; };

        session.connect_download_started(move |_sess, dl| {
            let suggested = dl
                .request()
                .and_then(|r| r.uri().map(|u| u.to_string()))
                .and_then(|uri| uri.rsplit_once('/').map(|(_, name)| name.to_string()))
                .unwrap_or_else(|| "download.bin".to_string());

            let dest_dir = dirs::download_dir()
                .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            let safe_name = sanitize_filename(&suggested);
            let path = dest_dir.join(&safe_name);

            let final_path = uniquify(&path);
            let path_str = final_path.to_string_lossy().to_string();

            dl.set_allow_overwrite(true);
            dl.set_destination(&path_str);

            let item = DownloadItem {
                filename: safe_name.clone(),
                path: path_str.clone(),
                done: false,
                failed: false,
                progress: 0.0,
            };
            mgr.borrow_mut().items.push(item);
            let idx = mgr.borrow().items.len().saturating_sub(1);

            let mgr_prog = mgr.clone();
            dl.connect_estimated_progress_notify(move |dl| {
                let p = dl.estimated_progress();
                if let Some(it) = mgr_prog.borrow_mut().items.get_mut(idx) {
                    it.progress = p.clamp(0.0, 1.0);
                }
            });

            let safe_name_fail = safe_name.clone();
            let mgr_fail = mgr.clone();
            dl.connect_failed(move |_dl, _err| {
                if let Some(it) = mgr_fail.borrow_mut().items.get_mut(idx) {
                    it.failed = true;
                }
                eprintln!("Download failed: {}", safe_name_fail);
            });

            let mgr_done = mgr.clone();
            dl.connect_finished(move |_dl| {
                if let Some(it) = mgr_done.borrow_mut().items.get_mut(idx) {
                    it.done = true;
                    it.progress = 1.0;
                }
                notify_download_complete(&safe_name, &path_str);
            });
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