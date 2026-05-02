use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
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

        let primary = t.get("mPrimary")      .and_then(|v| v.as_str()).unwrap_or("#3584e4");
        let on_primary = t.get("mOnPrimary")   .and_then(|v| v.as_str()).unwrap_or("#ffffff");
        let surface = t.get("mSurface")        .and_then(|v| v.as_str()).unwrap_or("#1e1e1e");
        let on_surface = t.get("mOnSurface")   .and_then(|v| v.as_str()).unwrap_or("#ffffff");
        let surface_variant = t.get("mSurfaceVariant")
                                            .and_then(|v| v.as_str()).unwrap_or("#2a2a2a");
        let on_surface_variant = t.get("mOnSurfaceVariant")
                                            .and_then(|v| v.as_str()).unwrap_or("#c0c0c0");
        let outline = t.get("mOutline")        .and_then(|v| v.as_str()).unwrap_or("#555555");
        let error = t.get("mError")            .and_then(|v| v.as_str()).unwrap_or("#e01b24");

        self.gtk_css = format!(
            "window {{\n\
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
             }}\n\
             * {{\n\
             transition: background-color 300ms ease-in-out,\n\
                         color 300ms ease-in-out,\n\
                         border-color 300ms ease-in-out;\n\
             }}\n\
             window, .window, .dialog, .osd, .background {{\n\
             background-color: {surface};\n\
             color: {on_surface};\n\
             }}\n\
             label, .label, .heading, .title-1, .title-2, .title-3, .title-4,\n\
             .caption, .body, .monospace {{\n\
             color: {on_surface};\n\
             }}\n\
             entry, textview, text, .entry {{\n\
             background-color: {surface_variant};\n\
             color: {on_surface};\n\
             }}\n\
             button {{\n\
             background-color: {surface_variant};\n\
             color: {on_surface};\n\
             }}\n\
             button:hover {{\n\
             background-color: {primary};\n\
             color: {on_primary};\n\
             }}\n\
             .toolbar {{\n\
             background-color: {surface_variant};\n\
             color: {on_surface_variant};\n\
             border-radius: 12px;\n\
             }}\n\
             listview, listbox, .list, .boxed-list {{\n\
             background-color: {surface};\n\
             color: {on_surface};\n\
             }}\n\
             row, listboxrow, .row {{\n\
             background-color: transparent;\n\
             color: {on_surface};\n\
             }}\n\
             row:hover, listboxrow:hover {{\n\
             background-color: {surface_variant};\n\
             }}\n\
             .command-overlay {{\n\
             background-color: {surface} !important;\n\
             color: {on_surface} !important;\n\
             }}\n\
             .command-overlay.background {{\n\
             background-color: {surface} !important;\n\
             color: {on_surface} !important;\n\
             }}\n\
             .command-col {{\n\
             border: 2px solid {primary};\n\
             border-radius: 12px;\n\
             padding: 8px;\n\
             background-color: {surface_variant};\n\
             }}\n\
             .command-col label {{\n\
             color: {on_surface};\n\
             }}\n\
             .command-selected {{\n\
             background-color: {primary};\n\
             color: {on_primary};\n\
             }}\n",
            primary = primary,
            on_primary = on_primary,
            surface = surface,
            on_surface = on_surface,
            surface_variant = surface_variant,
            on_surface_variant = on_surface_variant,
            outline = outline,
            error = error,
        );

        // Only set color-scheme as a *hint* to pages that support it.
        // We do NOT override form control colors or add transitions to the page,
        // because that causes unreadable light-on-light (or dark-on-dark)
        // combinations on sites that don't respect color-scheme.
        let scheme = if dark { "dark" } else { "light" };
        self.webkit_css = format!(
            ":root {{ color-scheme: {}; }}\n",
            scheme,
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

    pub fn start_watch(
        tm: Rc<RefCell<ThemeManager>>,
        webview: &webkit6::WebView,
        provider: &gtk4::CssProvider,
    ) {
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

        let provider = provider.clone();
        let webview_weak = webview.downgrade();
        monitor.connect_changed(move |_monitor, child, _other, event_type| {
            match event_type {
                gio::FileMonitorEvent::ChangesDoneHint
                | gio::FileMonitorEvent::Created
                | gio::FileMonitorEvent::Renamed
                | gio::FileMonitorEvent::AttributeChanged => {}
                _ => return,
            }

            if let Some(child_path) = child.path() {
                eprintln!("Noctalia: {:?} on {:?}", event_type, child_path);
                let expected = theme_path.as_ref().map(|p| p.as_path());
                if Some(child_path.as_path()) == expected {
                    eprintln!("Noctalia: theme file changed, reloading...");
                    tm.borrow_mut().reload(theme_path.as_deref());
                    if let Some(wv) = webview_weak.upgrade() {
                        let tm_ref = tm.borrow();
                        tm_ref.apply_gtk_css(&provider);
                        tm_ref.apply_webkit_css(&wv);
                        eprintln!("Noctalia: theme reloaded and applied");
                    }
                }
            }
        });
    }

    fn reload(&mut self, expected_path: Option<&Path>) {
        if let Some(path) = expected_path {
            let stored_path = self.theme_path.as_deref().unwrap_or(Path::new(""));
            if path == stored_path {
                if read_file(path).is_some() {
                    self.load();
                    eprintln!("Noctalia: theme reloaded from {:?}", path);
                } else {
                    eprintln!("Noctalia: theme file {:?} missing, skipping reload", path);
                }
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

pub fn is_dark_preferred() -> bool {
    let style_manager = adw::StyleManager::default();
    match style_manager.color_scheme() {
        adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => true,
        adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => false,
        adw::ColorScheme::Default => {
            // When libadwaita is in Default mode, check if the system prefers dark.
            // Do NOT read GTK_THEME or gtk-application-prefer-dark-theme —
            // libadwaita handles this internally and warns if we touch it.
            style_manager.is_dark()
        }
        _ => false,
    }
}
