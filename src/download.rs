use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gio::{Notification, prelude::*};
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, LevelBar, Orientation};
use webkit6::prelude::*;

use crate::parallel_download;

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

    pub fn attach(
        view: &webkit6::WebView,
        mgr: Rc<RefCell<DownloadManager>>,
        overlay: &gtk4::Overlay,
        tokio_rt: Arc<tokio::runtime::Runtime>,
    ) {
        let Some(session) = view.network_session() else { return; };
        let overlay = overlay.clone();

        session.connect_download_started(move |_sess, dl| {
            let url = match dl.request().and_then(|r| r.uri()).map(|u| u.to_string()) {
                Some(u) => u,
                None => return,
            };

            dl.cancel();

            let suggested = url
                .rsplit('/')
                .next()
                .and_then(|s| s.split('?').next())
                .unwrap_or("download.bin")
                .to_string();

            let dest_dir = dirs::download_dir()
                .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            let safe_name = sanitize_filename(&suggested);
            let final_path = uniquify(&dest_dir.join(&safe_name));
            let path_str = final_path.to_string_lossy().to_string();

            let idx = {
                let mut m = mgr.borrow_mut();
                m.items.push(DownloadItem {
                    filename: safe_name.clone(),
                    path: path_str.clone(),
                    done: false,
                    failed: false,
                    progress: 0.0,
                });
                m.items.len() - 1
            };

            let progress_widget = build_progress_widget(&safe_name, &path_str);
            overlay.add_overlay(&progress_widget);

            let ua = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".to_string();
            let (mut rx, handle) = parallel_download::start(url, final_path.clone(), ua, tokio_rt.handle().clone());

            let mgr_poll = mgr.clone();
            let progress_widget_poll = progress_widget.clone();
            let safe_name_poll = safe_name.clone();
            let path_str_poll = path_str.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                if rx.has_changed().unwrap_or(false) {
                    let prog = *rx.borrow_and_update();
                    let fraction = prog.fraction();

                    if let Some(item) = mgr_poll.borrow_mut().items.get_mut(idx) {
                        item.progress = fraction;
                    }

                    update_progress_widget(&progress_widget_poll, fraction);

                    if fraction >= 1.0 {
                        if let Some(item) = mgr_poll.borrow_mut().items.get_mut(idx) {
                            item.done = true;
                        }
                        notify_download_complete(&safe_name_poll, &path_str_poll);
                        let pw = progress_widget_poll.clone();
                        glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                            pw.unparent();
                        });
                        return glib::ControlFlow::Break;
                    }
                }

                if handle.is_finished() {
                    if let Some(item) = mgr_poll.borrow_mut().items.get_mut(idx) {
                        if !item.done {
                            item.failed = true;
                            eprintln!("Download failed: {}", safe_name_poll);
                            update_progress_widget_error(&progress_widget_poll);
                            let pw = progress_widget_poll.clone();
                            glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
                                pw.unparent();
                            });
                        }
                    }
                    return glib::ControlFlow::Break;
                }

                glib::ControlFlow::Continue
            });
        });
    }

    pub fn recent(&self, limit: usize) -> Vec<&DownloadItem> {
        self.items.iter().rev().filter(|i| i.done && !i.failed).take(limit).collect()
    }
}

fn build_progress_widget(filename: &str, path: &str) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 4);
    container.add_css_class("toolbar");
    container.set_margin_bottom(16);
    container.set_margin_end(16);
    container.set_halign(Align::End);
    container.set_valign(Align::End);
    container.set_size_request(280, -1);

    let name_lbl = Label::new(Some(filename));
    name_lbl.add_css_class("caption");
    name_lbl.set_halign(Align::Start);
    name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    container.append(&name_lbl);

    let bar = LevelBar::new();
    bar.set_min_value(0.0);
    bar.set_max_value(1.0);
    bar.set_value(0.0);
    bar.set_name("iron-dl-bar");
    container.append(&bar);

    let pct_lbl = Label::new(Some("0%"));
    pct_lbl.add_css_class("caption");
    pct_lbl.set_name("iron-dl-pct");
    pct_lbl.set_halign(Align::End);
    container.append(&pct_lbl);

    let open_btn = Button::with_label("Show in Files");
    open_btn.add_css_class("flat");
    open_btn.set_name("iron-dl-open");
    let path_owned = path.to_string();
    open_btn.connect_clicked(move |_| {
        open_folder(&path_owned);
    });
    open_btn.set_visible(false);
    container.append(&open_btn);

    container
}

fn update_progress_widget(widget: &GtkBox, fraction: f64) {
    let mut child = widget.first_child();
    while let Some(c) = child {
        if c.widget_name() == "iron-dl-bar" {
            if let Some(bar) = c.downcast_ref::<LevelBar>() {
                bar.set_value(fraction);
            }
        }
        if c.widget_name() == "iron-dl-pct" {
            if let Some(lbl) = c.downcast_ref::<Label>() {
                lbl.set_text(&format!("{:.0}%", fraction * 100.0));
            }
        }
        if fraction >= 1.0 && c.widget_name() == "iron-dl-open" {
            c.set_visible(true);
        }
        child = c.next_sibling();
    }
}

fn update_progress_widget_error(widget: &GtkBox) {
    let mut child = widget.first_child();
    while let Some(c) = child {
        if c.widget_name() == "iron-dl-pct" {
            if let Some(lbl) = c.downcast_ref::<Label>() {
                lbl.set_text("Failed");
            }
        }
        child = c.next_sibling();
    }
}

pub fn open_folder(path: &str) {
    let folder = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
}

fn notify_download_complete(filename: &str, path: &str) {
    let app = gio::Application::default();
    let Some(ref app) = app else { return; };

    let notif = Notification::new(&format!("Download complete: {}", filename));
    notif.set_body(Some(&format!("Saved to {}", path)));
    notif.set_priority(gio::NotificationPriority::Normal);

    let folder = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let target = glib::Variant::from(folder.as_str());
    notif.add_button_with_target_value("Open folder", "app.open-folder", Some(&target));
    app.send_notification(Some("iron-download"), &notif);
}

fn sanitize_filename(name: &str) -> String {
    name.chars().map(|c| match c { '/' | '\\' | '\0' => '_', _ => c }).collect()
}

fn uniquify(path: &std::path::Path) -> PathBuf {
    if !path.exists() { return path.to_path_buf(); }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("download");
    let ext = path.extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s)).unwrap_or_default();
    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    for n in 1..=9999 {
        let candidate = parent.join(format!("{} ({}){}", stem, n, ext));
        if !candidate.exists() { return candidate; }
    }
    path.to_path_buf()
}