use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::{self};
use gtk4::prelude::{FileMonitorExt, FileExt};

/// Convert a hex colour to a GTK-compatible rgba() or 6-char hex string.
/// Handles 6-char ("#RRGGBB"), 8-char QML ("#AARRGGBB"), and 8-char CSS ("#RRGGBBAA") formats.
/// QML stores colors as #AARRGGBB (alpha-first), so 8-char hex is always treated as QML format.
/// Returns 8-digit CSS hex (#RRGGBBAA) for GTK CSS compatibility.
fn hex_to_rgba(hex: &str, alpha: f64) -> String {
    let t = hex.trim().trim_start_matches('#');
    let (r, g, b) = if t.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&t[0..2], 16), u8::from_str_radix(&t[2..4], 16), u8::from_str_radix(&t[4..6], 16)) {
            (r, g, b)
        } else {
            return hex.to_string();
        }
    } else if t.len() == 8 {
        // QML color format: #AARRGGBB (alpha comes first)
        if let (Ok(_a), Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&t[0..2], 16),
            u8::from_str_radix(&t[2..4], 16),
            u8::from_str_radix(&t[4..6], 16),
            u8::from_str_radix(&t[6..8], 16),
        ) {
            (r, g, b)
        } else {
            return hex.to_string();
        }
    } else {
        return hex.to_string();
    };
    
    let a = (alpha * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
}

/// Sanitize a hex color from Noctalia for GTK CSS.
/// Converts QML 8-char #AARRGGBB to rgba(), passes through 6-char #RRGGBB as-is.
fn css_color(hex: &str) -> String {
    let t = hex.trim().trim_start_matches('#');
    if t.len() == 8 {
        hex_to_rgba(hex, 1.0)
    } else {
        hex.trim().to_string()
    }
}

pub struct ThemeManager {
    gtk_css: String,
    webkit_css: String,
    theme_path: Option<PathBuf>,
    _monitor: Option<gio::FileMonitor>,
}

impl ThemeManager {
    pub fn new() -> Self {
        ThemeManager {
            gtk_css: String::new(),
            webkit_css: String::new(),
            theme_path: None,
            _monitor: None,
        }
    }

    pub fn load(&mut self) {
        let theme_path = find_active_theme();
        self.theme_path = theme_path;

        let content = match self.theme_path.as_ref().and_then(|p| read_file(p)) {
            Some(c) => c,
            None => return,
        };

        eprintln!("Noctalia: load() content.len={} path={:?}", content.len(), self.theme_path);

        let tokens = match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Noctalia: JSON parse error: {}", e);
                return;
            }
        };

        let dark = is_dark_preferred();

        let primary = css_color(tokens.get("mPrimary").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#3584e4"));
        let on_primary = css_color(tokens.get("mOnPrimary").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#ffffff"));
        let surface = css_color(tokens.get("mSurface").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#1e1e1e"));
        let on_surface = css_color(tokens.get("mOnSurface").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#ffffff"));
        let surface_variant = css_color(tokens.get("mSurfaceVariant").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#2a2a2a"));
        let on_surface_variant = css_color(tokens.get("mOnSurfaceVariant").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#c0c0c0"));
        let error_color = css_color(tokens.get("mError").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#e01b24"));

        let surface_raw = tokens.get("mSurface").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("#1e1e1e");
        let surface_rgba = hex_to_rgba(surface_raw, 0.88);

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
             --error-color: {error_color};\n\
             --destructive-color: {error_color};\n\
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
             background-color: {surface_rgba} !important;\n\
             color: {on_surface} !important;\n\
             }}\n\
             .command-overlay.background {{\n\
             background-color: {surface_rgba} !important;\n\
             color: {on_surface} !important;\n\
             }}\n\
              .command-col {{\n\
              border: 2px solid {primary};\n\
              border-radius: 12px;\n\
              padding: 8px;\n\
              background-color: {surface_rgba};\n\
             }}\n\
             .command-col label {{\n\
             color: {on_surface};\n\
             }}\n\
              .command-col listbox {{\n\
              background-color: {surface_rgba};\n\
              color: {on_surface};\n\
              }}\n\
             .command-col listbox row {{\n\
             background-color: transparent;\n\
             }}\n\
             .command-selected {{\n\
             background-color: {primary};\n\
             color: {on_primary};\n\
             }}\n",
            primary = primary,
            on_primary = on_primary,
            surface = surface,
            on_surface = on_surface,
            surface_rgba = surface_rgba,
            surface_variant = surface_variant,
            on_surface_variant = on_surface_variant,
            error_color = error_color,
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
            provider.load_from_data(&self.gtk_css);
        }
    }

    pub fn apply_webkit_css(&self, webview: &crate::webkit_browser::WebKitBrowserWrapper) {
        if self.webkit_css.is_empty() {
            return;
        }
        let css = &self.webkit_css;
        let escaped = css.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
        let js = format!(
            "(function(){{ var s = document.createElement('style'); s.textContent = `{}`; document.head.appendChild(s); }})()",
            escaped
        );
        webview.execute_js(&js);
    }

    pub fn start_watch(
        tm: Rc<RefCell<ThemeManager>>,
        webview: &crate::webkit_browser::WebKitBrowserWrapper,
        provider: &gtk4::CssProvider,
    ) {
        let config_dir = match dirs::config_dir() {
            Some(d) => d,
            None => {
                eprintln!("Noctalia: no config dir found");
                return;
            }
        };
        // Noctalia Shell stores active colors in colors.json in ~/.config/noctalia/
        let noctalia_dir = config_dir.join("noctalia");

        eprintln!("Noctalia: watching dir={:?} for colors.json", noctalia_dir);
        if !noctalia_dir.exists() {
            eprintln!("Noctalia: noctalia config dir does not exist");
            return;
        }

        let file = gio::File::for_path(&noctalia_dir);
        let Ok(monitor) = file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) else {
            eprintln!("Noctalia: failed to create directory monitor");
            return;
        };

        let _provider = provider.clone();
        let tm_clone = tm.clone();
        let wv_clone = webview.clone();
        let is_colors_json = |f: &gio::File| -> bool {
            f.path().as_ref()
                .and_then(|p| p.file_name().map(|n| n == "colors.json"))
                .unwrap_or(false)
        };
        monitor.connect_changed(move |_monitor, child, other, event_type| {
            let child_match = is_colors_json(child);
            let other_match = other.map(|o| is_colors_json(o)).unwrap_or(false);
            if child_match || other_match {
                eprintln!("Noctalia: colors.json changed (event={:?})!", event_type);
                tm.borrow_mut().load();
                tm.borrow().apply_gtk_css(&_provider);
                tm.borrow().apply_webkit_css(&wv_clone);
            }
        });

        tm_clone.borrow_mut()._monitor = Some(monitor);
    }
}

fn find_active_theme() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    // Noctalia Shell writes active colors to colors.json in its config dir
    // This is NOT the same as user color schemes in colorschemes/
    let colors_file = config_dir.join("noctalia").join("colors.json");
    if colors_file.exists() {
        return Some(colors_file);
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
