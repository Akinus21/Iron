#![cfg(not(feature = "cef-stub"))]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::AtomicUsize;

use cef::{
    Browser, Frame, CefString,
    PaintElementType, Rect, FocusSource, TransitionType,
    BeforeDownloadCallback, DownloadCallback, DownloadItem,
    ImplClient, ImplLifeSpanHandler, ImplLoadHandler, ImplDisplayHandler,
    ImplRenderHandler, ImplFocusHandler, ImplDownloadHandler,
    LifeSpanHandler, LoadHandler, DisplayHandler, RenderHandler, FocusHandler, DownloadHandler,
    Client, App,
};

static BROWSER_CREATED: AtomicBool = AtomicBool::new(false);
static PAGE_LOAD_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct IronClientState {
    pub on_title_change: Option<Box<dyn Fn(&str)>>,
    pub on_url_change: Option<Box<dyn Fn(&str)>>,
    pub on_loading_state_change: Option<Box<dyn Fn(bool, bool, bool)>>,
    pub on_page_load_end: Option<Box<dyn Fn()>>,
}

pub type SharedClientState = Rc<RefCell<IronClientState>>;

pub fn create_shared_state() -> SharedClientState {
    Rc::new(RefCell::new(IronClientState {
        on_title_change: None,
        on_url_change: None,
        on_loading_state_change: None,
        on_page_load_end: None,
    }))
}

type RenderCallback = Box<dyn FnMut(&[u8], i32, i32) + Send + Sync>;

static RENDER_CALLBACK: std::sync::Mutex<Option<RenderCallback>> = std::sync::Mutex::new(None);

pub fn set_render_callback(callback: Option<Box<dyn FnMut(&[u8], i32, i32) + Send + Sync>>) {
    *RENDER_CALLBACK.lock().unwrap() = callback;
}

pub struct IronLifeSpanHandler {
    state: SharedClientState,
}

impl IronLifeSpanHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

pub struct IronLoadHandler {
    state: SharedClientState,
}

impl IronLoadHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

pub struct IronDisplayHandler {
    state: SharedClientState,
}

impl IronDisplayHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

pub struct IronRenderHandler;

impl IronRenderHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IronRenderHandler {
    fn default() -> Self {
        Self
    }
}

pub struct IronFocusHandler;

impl IronFocusHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IronFocusHandler {
    fn default() -> Self {
        Self
    }
}

pub struct IronDownloadHandler;

impl IronDownloadHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IronDownloadHandler {
    fn default() -> Self {
        Self
    }
}

pub struct IronClient {
    state: SharedClientState,
}

impl IronClient {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

impl ImplLifeSpanHandler for IronLifeSpanHandler {
    fn on_after_created(&self, _browser: Option<&mut Browser>) {
        eprintln!("[CEF] Browser created");
        BROWSER_CREATED.store(true, Ordering::SeqCst);
    }

    fn on_before_close(&self, _browser: Option<&mut Browser>) {
        eprintln!("[CEF] Browser closing");
        BROWSER_CREATED.store(false, Ordering::SeqCst);
    }
}

impl ImplLoadHandler for IronLoadHandler {
    fn on_load_start(
        &self,
        browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _transition_type: TransitionType,
    ) {
        if let Some(browser) = browser {
            if let Some(frame) = browser.main_frame() {
                if let Some(url) = frame.url() {
                    let url_str = url.to_string();
                    eprintln!("[CEF] Page load started: {}", url_str);
                }
            }
        }
    }

    fn on_load_end(
        &self,
        browser: Option<&mut Browser>,
        frame: Option<&mut Frame>,
        _http_status_code: i32,
    ) {
        if let Some(cb) = self.state.borrow_mut().on_page_load_end.as_ref() {
            cb();
        }

        if let Some(browser) = browser {
            if let Some(frame) = frame {
                if frame.is_main() != 0 {
                    if let Some(url) = frame.url() {
                        let url_str = url.to_string();
                        let title = browser.get_title().map(|t| t.to_string()).unwrap_or_default();
                        eprintln!("[CEF] Page loaded: {} - {}", url_str, title);
                    }
                }
            }
        }
    }

    fn on_loading_state_change(
        &self,
        _browser: Option<&mut Browser>,
        is_loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    ) {
        if let Some(cb) = self.state.borrow_mut().on_loading_state_change.as_ref() {
            cb(is_loading, can_go_back, can_go_forward);
        }
    }
}

impl ImplDisplayHandler for IronDisplayHandler {
    fn on_title_change(
        &self,
        _browser: Option<&mut Browser>,
        title: Option<&CefString>,
    ) {
        if let Some(t) = title {
            let title_str = t.to_string();
            if let Some(cb) = self.state.borrow_mut().on_title_change.as_ref() {
                cb(&title_str);
            }
        }
    }

    fn on_address_change(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        url: Option<&CefString>,
    ) {
        if let Some(u) = url {
            let url_str = u.to_string();
            if let Some(cb) = self.state.borrow_mut().on_url_change.as_ref() {
                cb(&url_str);
            }
        }
    }
}

impl ImplRenderHandler for IronRenderHandler {
    fn on_paint(
        &self,
        _browser: Option<&mut Browser>,
        _kind: PaintElementType,
        _dirty_rects: &[Rect],
        buffer: Option<&[u8]>,
        width: i32,
        height: i32,
    ) {
        let Some(buffer) = buffer else { return };
        if let Ok(mut guard) = RENDER_CALLBACK.lock() {
            if let Some(ref mut callback) = *guard {
                callback(buffer, width, height);
            }
        }
    }
}

impl ImplFocusHandler for IronFocusHandler {
    fn on_set_focus(
        &self,
        _browser: Option<&mut Browser>,
        _source: FocusSource,
    ) -> i32 {
        0
    }
}

impl ImplDownloadHandler for IronDownloadHandler {
    fn on_before_download(
        &self,
        _browser: Option<&mut Browser>,
        _download_item: Option<&mut DownloadItem>,
        _suggested_name: Option<&CefString>,
    ) -> BeforeDownloadCallback {
        eprintln!("[CEF] Download requested");
        BeforeDownloadCallback::default()
    }

    fn on_download_updated(
        &self,
        _browser: Option<&mut Browser>,
        download_item: Option<&mut DownloadItem>,
        _callback: Option<&mut DownloadCallback>,
    ) {
        if let Some(item) = download_item {
            let is_done = item.is_done() != 0;
            let percent = item.percent_complete();
            let speed = item.current_speed();
            let url = item.url().map(|s| s.to_string()).unwrap_or_default();

            if is_done {
                eprintln!("[CEF] Download complete: {}", url);
            } else {
                eprintln!("[CEF] Download progress: {}% at {} bytes/sec", percent, speed);
            }
        }
    }
}

impl ImplClient for IronClient {
    fn life_span_handler(&self) -> Option<LifeSpanHandler> {
        None
    }

    fn load_handler(&self) -> Option<LoadHandler> {
        None
    }

    fn display_handler(&self) -> Option<DisplayHandler> {
        None
    }

    fn render_handler(&self) -> Option<RenderHandler> {
        None
    }

    fn focus_handler(&self) -> Option<FocusHandler> {
        None
    }

    fn download_handler(&self) -> Option<DownloadHandler> {
        None
    }
}

pub fn is_browser_created() -> bool {
    BROWSER_CREATED.load(Ordering::SeqCst)
}