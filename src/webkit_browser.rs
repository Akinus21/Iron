use glib::object::Cast;
use gtk4::prelude::*;
use gtk4::{Widget, EventControllerKey};
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::{WebView, UserContentManager, UserStyleSheet, UserContentInjectedFrames, UserStyleLevel, NetworkSession, Settings, UserScript, UserScriptInjectionTime, WebViewExt};
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

    pub fn set_color_scheme(&self, scheme: &str) {
        let scheme_str = match scheme {
            "dark" => "dark",
            _ => "light",
        };

        // CSS signal – inject :root { color-scheme } at User level so
        // prefers-color-scheme media queries evaluate to the chosen scheme.
        let signal_css = format!(":root {{ color-scheme: {}; }}\n", scheme_str);

        // 3. Gentle CSS fallback – injected at *Author* level (same cascade
        //    priority as the page’s own CSS).  This means responsive sites
        //    with more-specific selectors naturally override our fallback.
        //    Non-responsive sites that never set explicit body colours get
        //    a dark background + light text from these base rules.
        let (bg, fg) = if scheme_str == "dark" {
            ("#1a1a1a", "#e6e6e6")
        } else {
            ("#ffffff", "#1a1a1a")
        };
        let fallback_css = format!(
            "html {{ background-color: {}; }}\n\
            body {{ color: {}; }}\n",
            bg, fg
        );

        if let Some(ucm) = self.web_view.user_content_manager() {
            ucm.remove_all_style_sheets();

            // Signal sheet (highest priority, cannot be overridden).
            let signal_sheet = UserStyleSheet::new(
                &signal_css,
                UserContentInjectedFrames::AllFrames,
                UserStyleLevel::User,
                &[],
                &[],
            );
            ucm.add_style_sheet(&signal_sheet);

            // Fallback sheet (author priority, responsive sites override us).
            let fallback_sheet = UserStyleSheet::new(
                &fallback_css,
                UserContentInjectedFrames::AllFrames,
                UserStyleLevel::Author,
                &[],
                &[],
            );
            ucm.add_style_sheet(&fallback_sheet);
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

    /// Inject the AkSprayPaint-style recoloring JS via UserContentManager.
    /// The script runs at document end so it sees the full DOM.
    pub fn apply_recolor(&self, script: &str) {
        if let Some(ucm) = self.web_view.user_content_manager() {
            // Remove previous recolor scripts to avoid stacking
            ucm.remove_all_scripts();

            let user_script = UserScript::new(
                script,
                UserContentInjectedFrames::AllFrames,
                UserScriptInjectionTime::AtDocumentEnd,
                &[], // allow_list
                &[], // block_list
            );
            ucm.add_script(&user_script);
        }
    }

    /// Disable recoloring by removing all injected scripts and calling
    /// the JS deactivation hook.
    pub fn disable_recolor(&self) {
        if let Some(ucm) = self.web_view.user_content_manager() {
            ucm.remove_all_scripts();
        }
        self.execute_js("if(window.__iron_recolor_deactivate) window.__iron_recolor_deactivate();");
    }
}