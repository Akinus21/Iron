use gtk4::prelude::*;
use gtk4::{Widget, gdk, glib};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct CefBrowserWrapper {
    pub widget: Widget,
    url: Rc<RefCell<String>>,
    title: Rc<RefCell<String>>,
    is_loading: Rc<RefCell<bool>>,
    can_go_back: Rc<RefCell<bool>>,
    can_go_forward: Rc<RefCell<bool>>,
    window_handle: u64,
}

impl CefBrowserWrapper {
    pub fn new(_parent_window: &gdk::Surface, url: &str, _is_offscreen: bool) -> Result<Self, String> {
        let box_widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_widget.set_hexpand(true);
        box_widget.set_vexpand(true);

        let label = gtk4::Label::new(Some(
            "CEF Browser Integration In Progress\n\n\
             This is a placeholder. Full CEF integration requires:\n\
             1. CEF binary distribution download\n\
             2. X11 window embedding setup\n\
             3. CEF message loop integration\n\n\
             See build instructions in README.md"
        ));
        label.set_halign(gtk4::Align::Center);
        label.set_valign(gtk4::Align::Center);
        box_widget.append(&label);

        let url_str = url.to_string();
        let title_str = format!("Iron - {}", url_str);

        Ok(Self {
            widget: box_widget.upcast(),
            url: Rc::new(RefCell::new(url_str)),
            title: Rc::new(RefCell::new(title_str)),
            is_loading: Rc::new(RefCell::new(false)),
            can_go_back: Rc::new(RefCell::new(false)),
            can_go_forward: Rc::new(RefCell::new(false)),
            window_handle: 0,
        })
    }

    pub fn load_uri(&self, url: &str) {
        *self.url.borrow_mut() = url.to_string();
        *self.title.borrow_mut() = format!("Iron - {}", url);
        *self.is_loading.borrow_mut() = true;

        glib::timeout_add_local_once(std::time::Duration::from_millis(100), {
            let is_loading = self.is_loading.clone();
            move || {
                *is_loading.borrow_mut() = false;
            }
        });
    }

    pub fn uri(&self) -> Option<String> {
        let url = self.url.borrow().clone();
        if url.is_empty() { None } else { Some(url) }
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

    pub fn go_back(&self) -> bool {
        if *self.can_go_back.borrow() { true } else { false }
    }

    pub fn go_forward(&self) -> bool {
        if *self.can_go_forward.borrow() { true } else { false }
    }

    pub fn can_go_back(&self) -> bool {
        *self.can_go_back.borrow()
    }

    pub fn can_go_forward(&self) -> bool {
        *self.can_go_forward.borrow()
    }

    pub fn reload(&self) {
        let current_url = self.url.borrow().clone();
        self.load_uri(&current_url);
    }

    pub fn execute_javascript(&self, js_code: &str) {
        eprintln!("Executing JS: {}", js_code);
    }

    pub fn find(&self, text: &str, forward: bool, case_sensitive: bool) {
        eprintln!("Find: '{}' (forward={}, case={})", text, forward, case_sensitive);
    }

    pub fn stop_finding(&self) {}

    pub fn search_next(&self) {
        eprintln!("search_next called");
    }

    pub fn grab_focus(&self) {
        self.widget.grab_focus();
    }
}

pub struct IronDownloadHandler;

impl IronDownloadHandler {
    pub fn new() -> Self { Self }
}

pub struct IronClientHandler {
    pub on_loading_state_change: Option<Box<dyn Fn(bool, bool, bool)>>,
    pub on_title_change: Option<Box<dyn Fn(&str)>>,
    pub on_address_change: Option<Box<dyn Fn(&str)>>,
}

impl IronClientHandler {
    pub fn new() -> Self {
        Self {
            on_loading_state_change: None,
            on_title_change: None,
            on_address_change: None,
        }
    }
}