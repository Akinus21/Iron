#![cfg(not(feature = "cef-stub"))]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::AtomicUsize;

use cef::{
    Browser, Frame, CefString, BeforeDownloadCallback, DownloadItem, DownloadItemCallback,
    PaintElementType, Rect, FocusSource, TransitionType,
    ImplClient, ImplLifeSpanHandler, ImplLoadHandler, ImplDisplayHandler,
    ImplRenderHandler, ImplFocusHandler, ImplDownloadHandler,
    LifeSpanHandler, LoadHandler, DisplayHandler, RenderHandler, FocusHandler, DownloadHandler,
    Client,
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

#[derive(Clone)]
pub struct IronLifeSpanHandler {
    state: SharedClientState,
}

impl IronLifeSpanHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

#[derive(Clone)]
pub struct IronLoadHandler {
    state: SharedClientState,
}

impl IronLoadHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

#[derive(Clone)]
pub struct IronDisplayHandler {
    state: SharedClientState,
}

impl IronDisplayHandler {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
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

#[derive(Clone)]
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

#[derive(Clone)]
pub struct IronClient {
    state: SharedClientState,
}

impl IronClient {
    pub fn new(state: SharedClientState) -> Self {
        Self { state }
    }
}

impl ImplLifeSpanHandler for IronLifeSpanHandler {
    fn get_raw(&self) -> *mut _cef_life_span_handler_t {
        unimplemented!()
    }

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
    fn get_raw(&self) -> *mut _cef_load_handler_t {
        unimplemented!()
    }

    fn on_load_start(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _transition_type: TransitionType,
    ) {
        eprintln!("[CEF] Page load started");
    }

    fn on_load_end(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _http_status_code: i32,
    ) {
        if let Some(cb) = self.state.borrow_mut().on_page_load_end.as_ref() {
            cb();
        }
        eprintln!("[CEF] Page loaded");
    }

    fn on_loading_state_change(
        &self,
        _browser: Option<&mut Browser>,
        is_loading: i32,
        can_go_back: i32,
        can_go_forward: i32,
    ) {
        if let Some(cb) = self.state.borrow_mut().on_loading_state_change.as_ref() {
            cb(is_loading != 0, can_go_back != 0, can_go_forward != 0);
        }
    }
}

impl ImplDisplayHandler for IronDisplayHandler {
    fn get_raw(&self) -> *mut _cef_display_handler_t {
        unimplemented!()
    }

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
    fn get_raw(&self) -> *mut _cef_render_handler_t {
        unimplemented!()
    }

    fn on_paint(
        &self,
        _browser: Option<&mut Browser>,
        _kind: PaintElementType,
        _dirty_rects: Option<&[Rect]>,
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
    fn get_raw(&self) -> *mut _cef_focus_handler_t {
        unimplemented!()
    }

    fn on_set_focus(
        &self,
        _browser: Option<&mut Browser>,
        _source: FocusSource,
    ) -> i32 {
        0
    }
}

impl ImplDownloadHandler for IronDownloadHandler {
    fn get_raw(&self) -> *mut _cef_download_handler_t {
        unimplemented!()
    }

    fn on_before_download(
        &self,
        _browser: Option<&mut Browser>,
        _download_item: Option<&mut DownloadItem>,
        _suggested_name: Option<&CefString>,
        _callback: Option<&mut BeforeDownloadCallback>,
    ) -> i32 {
        eprintln!("[CEF] Download requested");
        0
    }

    fn on_download_updated(
        &self,
        _browser: Option<&mut Browser>,
        _download_item: Option<&mut DownloadItem>,
        _callback: Option<&mut DownloadItemCallback>,
    ) {
        eprintln!("[CEF] Download updated");
    }
}

impl ImplClient for IronClient {
    fn get_raw(&self) -> *mut _cef_client_t {
        unimplemented!()
    }

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