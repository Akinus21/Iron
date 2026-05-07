use gtk4::prelude::*;
use gtk4::{Widget, gdk, glib};
use std::cell::RefCell;
use std::rc::Rc;

use crate::cef_client::{self, IronClient, SharedClientState};

#[derive(Clone)]
pub struct CefBrowserWrapper {
    pub widget: Widget,
    browser: Rc<RefCell<Option<cef::Browser>>>,
    client_state: SharedClientState,
    url: Rc<RefCell<String>>,
    title: Rc<RefCell<String>>,
    is_loading: Rc<RefCell<bool>>,
    can_go_back: Rc<RefCell<bool>>,
    can_go_forward: Rc<RefCell<bool>>,
}

impl CefBrowserWrapper {
    pub fn new(
        parent_window: Option<&gdk::Surface>,
        url: &str,
        is_offscreen: bool,
    ) -> Result<Self, String> {
        let client_state = cef_client::create_shared_state();

        let container = gtk4::Picture::new();
        container.set_hexpand(true);
        container.set_vexpand(true);

        let url_str = url.to_string();
        let title_str = format!("Iron - {}", url_str);

        let wrapper = Self {
            widget: container.upcast(),
            browser: Rc::new(RefCell::new(None)),
            client_state: client_state.clone(),
            url: Rc::new(RefCell::new(url_str.clone())),
            title: Rc::new(RefCell::new(title_str)),
            is_loading: Rc::new(RefCell::new(true)),
            can_go_back: Rc::new(RefCell::new(false)),
            can_go_forward: Rc::new(RefCell::new(false)),
        };

        let is_loading_clone = wrapper.is_loading.clone();
        let can_go_back_clone = wrapper.can_go_back.clone();
        let can_go_forward_clone = wrapper.can_go_forward.clone();
        let title_clone = wrapper.title.clone();
        let url_clone = wrapper.url.clone();

        client_state.borrow_mut().on_loading_state_change = Some(Box::new(move |loading, back, forward| {
            *is_loading_clone.borrow_mut() = loading;
            *can_go_back_clone.borrow_mut() = back;
            *can_go_forward_clone.borrow_mut() = forward;
        }));

        let title_clone2 = wrapper.title.clone();
        client_state.borrow_mut().on_title_change = Some(Box::new(move |t: &str| {
            *title_clone2.borrow_mut() = t.to_string();
        }));

        let url_clone2 = wrapper.url.clone();
        client_state.borrow_mut().on_url_change = Some(Box::new(move |u: &str| {
            *url_clone2.borrow_mut() = u.to_string();
        }));

        if !crate::cef_init::is_cef_initialized() {
            eprintln!("[CEF] CEF not initialized, showing placeholder");
            return Ok(wrapper);
        }

        let mut client = IronClient::new(client_state.clone());

        let mut window_info = cef::WindowInfo::default();

        if is_offscreen || parent_window.is_none() {
            window_info.windowless_rendering_enabled = 1;
            window_info.set_as_windowless(0);
        } else if let Some(surface) = parent_window {
            let win_id = get_window_handle(surface);
            window_info.set_as_child(win_id, cef::Rect::default());
        }

        let mut browser_settings = cef::BrowserSettings::default();
        browser_settings.windowless_rendering_enabled = if is_offscreen || parent_window.is_none() { 1 } else { 0 };

        let cef_url = cef::CefString::from(url_str.as_str());

        let result = cef::browser_host_create_browser(
            Some(&window_info),
            Some(&mut client),
            Some(&cef_url),
            Some(&browser_settings),
            None,
            None,
        );

        if result != 1 {
            eprintln!("[CEF] Failed to create browser (result={})", result);
            return Err("Failed to create CEF browser".to_string());
        }

        eprintln!("[CEF] Browser creation initiated for {}", url_str);
        Ok(wrapper)
    }

    pub fn load_uri(&self, url: &str) {
        *self.url.borrow_mut() = url.to_string();
        *self.title.borrow_mut() = format!("Iron - {}", url);
        *self.is_loading.borrow_mut() = true;

        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(frame) = browser.main_frame() {
                let cef_url = cef::CefString::from(url);
                frame.load_url(Some(&cef_url));
            }
        }
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
        if let Some(browser) = self.browser.borrow().as_ref() {
            if browser.can_go_back() != 0 {
                browser.go_back();
                return true;
            }
        }
        false
    }

    pub fn go_forward(&self) -> bool {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if browser.can_go_forward() != 0 {
                browser.go_forward();
                return true;
            }
        }
        false
    }

    pub fn can_go_back(&self) -> bool {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.can_go_back() != 0
        } else {
            *self.can_go_back.borrow()
        }
    }

    pub fn can_go_forward(&self) -> bool {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.can_go_forward() != 0
        } else {
            *self.can_go_forward.borrow()
        }
    }

    pub fn reload(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.reload();
        } else {
            let current_url = self.url.borrow().clone();
            self.load_uri(&current_url);
        }
    }

    pub fn execute_javascript(&self, js_code: &str) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(frame) = browser.main_frame() {
                let code = cef::CefString::from(js_code);
                let url = cef::CefString::from("https://iron-browser.internal");
                frame.execute_java_script(Some(&code), Some(&url), 0);
            }
        } else {
            eprintln!("[CEF] Cannot execute JS: no browser (placeholder mode)");
        }
    }

    pub fn find(&self, text: &str, forward: bool, case_sensitive: bool) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(host) = browser.host() {
                let search_text = cef::CefString::from(text);
                host.find(Some(&search_text), forward as i32, case_sensitive as i32, 0);
            }
        }
    }

    pub fn stop_finding(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(host) = browser.host() {
                host.stop_finding(1);
            }
        }
    }

    pub fn search_next(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(host) = browser.host() {
                let empty = cef::CefString::from("");
                host.find(Some(&empty), 1, 0, 1);
            }
        }
    }

    pub fn grab_focus(&self) {
        self.widget.grab_focus();
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(host) = browser.host() {
                host.set_focus(1);
            }
        }
    }

    pub fn get_browser(&self) -> Option<cef::Browser> {
        self.browser.borrow().as_ref().cloned()
    }

    pub fn set_browser(&self, browser: cef::Browser) {
        *self.browser.borrow_mut() = Some(browser);
    }
}

#[cfg(target_os = "linux")]
fn get_window_handle(surface: &gdk::Surface) -> u64 {
    // On X11: return the X11 window ID for native embedding
    // On Wayland: return 0 to force OSR (off-screen rendering)
    // OSR works everywhere and avoids X11/Wayland display issues
    let _ = surface;
    0
}

#[cfg(not(target_os = "linux"))]
fn get_window_handle(_surface: &gdk::Surface) -> u64 {
    0
}