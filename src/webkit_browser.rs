use glib::object::Cast;
use gtk4::prelude::*;
use gtk4::{Widget, EventControllerKey};
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::WebView;
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
    ) -> Result<Self, String> {
        let web_view = WebView::new();

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

        eprintln!("[WebKit] Starting load of: {}", url_str);
        wrapper.load_uri(&url_str);
        wrapper.widget.grab_focus();

        Ok(wrapper)
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