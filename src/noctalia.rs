use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::prelude::*;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserStyleLevel, UserStyleSheet};

pub struct ThemeManager {
    gtk_css: String,
    webkit_css: String,
    theme_path: Option<PathBuf>,
}

impl ThemeManager {
    pub fn new() -> Self {
        ThemeManager {
            gtk_css: String::new(),
            webkit_css: String::new(),
            theme_path: None,
        }
    }

    pub fn load(&mut self) {
        let theme_path = find_active_theme();
        self.theme_path = theme_path;

        let content = match self.theme_path.as_ref().and_then(|p| read_file(p)) {
            Some(c) => c,
            None => return,
        };

        let tokens = match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Noctalia: JSON parse error: {}", e);
                return;
            }
        };

        let dark = is_dark_preferred();
        let variant = if dark { "dark" } else { "light" };

        let t = match tokens.get(variant) {
            Some(v) => v,
            None => return,
        };

        let primary = t.get("mPrimary").and_then(|v| v.as_str()).unwrap_or("");
        let on_primary = t.get("mOnPrimary").and_then(|v| v.as_str()).unwrap_or("");
        let surface = t.get("mSurface").and_then(|v| v.as_str()).unwrap_or("");
        let on_surface = t.get("mOnSurface").and_then(|v| v.as_str()).unwrap_or("");
        let surface_variant = t.get("mSurfaceVariant").and_then(|v| v.as_str()).unwrap_or("");
        let on_surface_variant = t.get("mOnSurfaceVariant").and_then(|v| v.as_str()).unwrap_or("");
        let error = t.get("mError").and_then(|v| v.as_str()).unwrap_or("");

        self.gtk_css = format!(
            ":root {{\n\
             --accent-color: {primary};\n\
             --accent-bg-color: {primary};\n\
             --accent-fg-color: {on_primary};\n\
             --window-bg-color: {surface};\n\
             --window-fg-color: {on_surface};\n\
             --view-bg-color: {surface};\n\
             --view-fg-color: {on_surface};\n\
             --headerbar-bg-color: {surface_variant};\n\
             --headerbar-fg-color: {on_surface_variant};\n\
             --card-bg-color: {surface_variant};\n\
             --card-fg-color: {on_surface_variant};\n\
             --sidebar-bg-color: {surface_variant};\n\
             --popover-bg-color: {surface_variant};\n\
             --popover-fg-color: {on_surface_variant};\n\
             --error-color: {error};\n\
             --destructive-color: {error};\n\
             }}\n",
            primary = primary,
            on_primary = on_primary,
            surface = surface,
            on_surface = on_surface,
            surface_variant = surface_variant,
            on_surface_variant = on_surface_variant,
            error = error,
        );

        self.webkit_css = format!(
            "@media screen {{\n\
             body, .content, main {{\n\
             background-color: {surface} !important;\n\
             color: {on_surface} !important;\n\
             }}\n\
             code, pre, textarea, input, select {{\n\
             background-color: {surface_variant} !important;\n\
             }}\n\
             }}\n",
            surface = surface,
            on_surface = on_surface,
            surface_variant = surface_variant,
        );
    }

    pub fn gtk_css(&self) -> &str {
        &self.gtk_css
    }

    pub fn webkit_css(&self) -> &str {
        &self.webkit_css
    }

    pub fn apply_gtk_css(&self, provider: &gtk4::CssProvider) {
        if !self.gtk_css.is_empty() {
            provider.load_from_string(&self.gtk_css);
        }
    }

    pub fn apply_webkit_css(&self, webview: &webkit6::WebView) {
        if self.webkit_css.is_empty() {
            return;
        }
        if let Some(cm) = webview.user_content_manager() {
            cm.remove_all_style_sheets();
            let stylesheet = UserStyleSheet::new(
                &self.webkit_css,
                UserContentInjectedFrames::AllFrames,
                UserStyleLevel::User,
                &[],
                &[],
            );
            cm.add_style_sheet(&stylesheet);
        }
    }

    pub fn start_watch(tm: Rc<RefCell<ThemeManager>>, webview: &webkit6::WebView) {
        let theme_path = {
            let tm_ref = tm.borrow();
            tm_ref.theme_path.clone()
        };

        let watch_dir = match theme_path.as_ref() {
            Some(path) => path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| path.clone()),
            None => return,
        };

        if !watch_dir.is_dir() {
            return;
        }

        let file = gio::File::for_path(&watch_dir);
        let Ok(monitor) = file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) else {
            eprintln!("Noctalia: failed to create directory monitor");
            return;
        };

        let webview_weak = webview.downgrade();
        monitor.connect_changed(move |_monitor, child, _other, event_type| {
            match event_type {
                gio::FileMonitorEvent::ChangesDoneHint => {}
                _ => return,
            }

            if let Some(child) = child {
                if let Some(child_path) = child.path() {
                    let expected = theme_path.as_ref().map(|p| p.as_path());
                    if Some(child_path.as_path()) == expected {
                        tm.borrow_mut().reload(theme_path.as_deref());
                        if let Some(wv) = webview_weak.upgrade() {
                            let tm_ref = tm.borrow();
                            tm_ref.apply_webkit_css(&wv);
                        }
                    }
                }
            }
        });
    }

    fn reload(&mut self, expected_path: Option<&Path>) {
        if let Some(path) = expected_path {
            if path == self.theme_path.as_deref().unwrap_or(Path::new("")) && read_file(path).is_some() {
                self.load();
            }
        }
    }
}

fn find_active_theme() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    let schemes_dir = config_dir.join("noctalia").join("colorschemes");

    let entries = std::fs::read_dir(&schemes_dir).ok()?;
    for entry in entries.flatten() {
        let theme_dir = entry.path();
        if theme_dir.is_dir() {
            let theme_name = theme_dir.file_name()?.to_str()?;
            let json_path = theme_dir.join(format!("{}.json", theme_name));
            if json_path.exists() {
                return Some(json_path);
            }
        }
    }

    None
}

fn read_file(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) if !content.trim().is_empty() => Some(content),
        Ok(_) => None,
        Err(e) => {
            eprintln!("Noctalia: cannot read {:?}: {}", path, e);
            None
        }
    }
}

fn is_dark_preferred() -> bool {
    std::env::var("GTK_THEME")
        .map(|t| t.to_lowercase().contains("dark"))
        .unwrap_or(false)
}
