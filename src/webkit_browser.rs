use glib::object::Cast;
use gtk4::prelude::*;
use gtk4::{Widget, EventControllerKey};
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::{WebView, UserContentManager, UserStyleSheet, UserContentInjectedFrames, UserStyleLevel, NetworkSession};
use webkit6::prelude::WebViewExt;

#[derive(Clone)]
pub struct WebKitBrowserWrapper {
    pub widget: Widget,
    web_view: WebView,
    url: Rc<RefCell<String>>,
    title: Rc<RefCell<String>>,
    is_loading: Rc<RefCell<bool>>,
}

impl WebKitBrowserWrapper {
    pub fn new(
        _parent_window: Option<&gtk4::gdk::Surface>,
        url: &str,
        _is_offscreen: bool,
        network_session: Option<&NetworkSession>,
        color_scheme: Option<&str>,
    ) -> Result<Self, String> {
        let web_view = match network_session {
            Some(ns) => WebView::builder()
                .network_session(ns)
                .build(),
            None => WebView::new(),
        };

        let url_str = url.to_string();
        let title_str = format!("Iron - {}", url_str);

        let widget = web_view.clone().upcast::<Widget>();
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        widget.set_size_request(640, 480);

        let wrapper = Self {
            widget,
            web_view,
            url: Rc::new(RefCell::new(url_str.clone())),
            title: Rc::new(RefCell::new(title_str.clone())),
            is_loading: Rc::new(RefCell::new(true)),
        };

        // Apply colour scheme *before* loading the URL so the first
        // page render already sees the right scheme.
        if let Some(scheme) = color_scheme {
            wrapper.set_color_scheme(scheme);
        }

        wrapper.load_uri(&url_str);
        wrapper.widget.grab_focus();

        Ok(wrapper)
    }

    /// Returns the `NetworkSession` associated with this WebView.
    pub fn network_session(&self) -> Option<NetworkSession> {
        self.web_view.network_session()
    }

    pub fn load_uri(&self, url: &str) {
        *self.url.borrow_mut() = url.to_string();
        *self.title.borrow_mut() = format!("Iron - {}", url);
        self.web_view.load_uri(url);
    }

    pub fn reload(&self) {
        self.web_view.reload();
    }

    pub fn go_back(&self) {
        self.web_view.go_back();
    }

    pub fn go_forward(&self) {
        self.web_view.go_forward();
    }

    pub fn get_url(&self) -> String {
        self.url.borrow().clone()
    }

    pub fn get_title(&self) -> String {
        self.title.borrow().clone()
    }

    pub fn is_loading(&self) -> bool {
        *self.is_loading.borrow()
    }

    pub fn can_go_back(&self) -> bool {
        true
    }

    pub fn can_go_forward(&self) -> bool {
        true
    }

    /// Set the preferred color scheme for web pages (light or dark).
    /// Injects a comprehensive UserStyleSheet that:
    /// 1. Sets `color-scheme` on `:root` so `prefers-color-scheme` media
    ///    queries evaluate correctly (e.g. GitHub, Google, DuckDuckGo).
    /// 2. Forces a fallback background/text colour on `<html>` and `<body>`
    ///    for pages that do not implement dark mode at all.
    /// 3. Inherits the forced colours down to common block-level elements
    ///    so the fallback is not broken by element-specific overrides.
    /// 4. Preserves images, videos and iframes from colour inversion.
    pub fn set_color_scheme(&self, scheme: &str) {
        let scheme_str = match scheme {
            "dark" => "dark",
            _ => "light",
        };
        let (bg, fg) = if scheme_str == "dark" {
            ("#1a1a1a", "#e6e6e6")
        } else {
            ("#ffffff", "#000000")
        };
        let link = if scheme_str == "dark" { "#80bfff" } else { "#0000ee" };
        let vlink = if scheme_str == "dark" { "#c58af9" } else { "#551a8b" };

        let css = format!(
            "/* === Iron forced colour scheme === */\n\
            :root {{\n\
                color-scheme: {};\n\
            }}\n\n\
            /* Inform pages that prefer dark/light via media query */\n\
            @media (prefers-color-scheme: {}) {{\n\
                :root {{\n\
                    color-scheme: {};\n\
                }}\n\
            }}\n\n\
            /* Fallback for pages without proper dark-mode support */\n\
            html, body {{\n\
                background-color: {} !important;\n\
                color: {} !important;\n\
            }}\n\
            /* Ensure common text containers inherit the fallback */\n\
            div, span, p, li, td, th, label, h1, h2, h3, h4, h5, h6,\n\
            article, section, aside, header, footer, main, nav,\n\
            blockquote, pre, code, figure, figcaption {{\n\
                background-color: transparent !important;\n\
                color: inherit !important;\n\
            }}\n\
            /* Preserve link colours */\n\
            a:link {{ color: {link} !important; }}\n\
            a:visited {{ color: {vlink} !important; }}\n\
            a:active {{ color: {link} !important; }}\n\
            /* Do NOT invert images, video or iframes */\n\
            img, picture, video, svg, iframe, canvas, embed, object {{\n\
                filter: none !important;\n\
            }}\n",
            scheme_str, scheme_str, scheme_str, bg, fg
        );

        if let Some(ucm) = self.web_view.user_content_manager() {
            ucm.remove_all_style_sheets();
            let stylesheet = UserStyleSheet::new(
                &css,
                UserContentInjectedFrames::AllFrames,
                UserStyleLevel::User,
                &[], // allow_list
                &[], // block_list
            );
            ucm.add_style_sheet(&stylesheet);
        }
    }

    pub fn execute_js(&self, script: &str) {
        let _ = self.web_view.evaluate_javascript(script, None, None, None::<&gio::Cancellable>, |_| {});
    }

    pub fn find(&self, _text: &str, _forward: bool, _case_sensitive: bool) {
        eprintln!("[WebKit] Find not yet implemented");
    }

    pub fn stop_finding(&self) {
        eprintln!("[WebKit] Stop finding not yet implemented");
    }

    pub fn search_next(&self) {
        eprintln!("[WebKit] Search next not yet implemented");
    }

    pub fn grab_focus(&self) {
        self.widget.grab_focus();
    }
}