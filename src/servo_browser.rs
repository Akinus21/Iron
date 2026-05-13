use gtk4::prelude::*;
use gtk4::{Widget, EventControllerKey, EventControllerMotion, GestureClick, EventControllerScroll};
use gtk4::EventControllerFocus;
use std::cell::RefCell;
use std::rc::Rc;
use servo_gtk::WebView;

#[derive(Clone)]
pub struct ServoBrowserWrapper {
    pub widget: Widget,
    web_view: WebView,
    url: Rc<RefCell<String>>,
    title: Rc<RefCell<String>>,
    is_loading: Rc<RefCell<bool>>,
}

impl ServoBrowserWrapper {
    pub fn new(
        _parent_window: Option<&gtk4::gdk::Surface>,
        url: &str,
        _is_offscreen: bool,
    ) -> Result<Self, String> {
        let web_view = WebView::new();

        let url_str = url.to_string();
        let title_str = format!("Iron - {}", url_str);

        let wrapper = Self {
            widget: web_view.clone().upcast(),
            web_view,
            url: Rc::new(RefCell::new(url_str.clone())),
            title: Rc::new(RefCell::new(title_str)),
            is_loading: Rc::new(RefCell::new(true)),
        };

        wrapper.widget.grab_focus();

        Ok(wrapper)
    }

    pub fn load_uri(&self, url: &str) {
        *self.url.borrow_mut() = url.to_string();
        *self.title.borrow_mut() = format!("Iron - {}", url);
        self.web_view.load_url(url);
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

    pub fn execute_js(&self, _script: &str) {
        eprintln!("[Servo] JS execution not yet implemented");
    }

    pub fn find(&self, _text: &str, _forward: bool, _case_sensitive: bool) {
        eprintln!("[Servo] Find not yet implemented");
    }

    pub fn stop_finding(&self) {
        eprintln!("[Servo] Stop finding not yet implemented");
    }

    pub fn search_next(&self) {
        eprintln!("[Servo] Search next not yet implemented");
    }

    pub fn grab_focus(&self) {
        self.widget.grab_focus();
    }
}