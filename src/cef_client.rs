#![cfg(not(feature = "cef-stub"))]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

static BROWSER_CREATED: AtomicBool = AtomicBool::new(false);

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

type RenderCallback = Rc<RefCell<dyn FnMut(&[u8], i32, i32)>>;

static RENDER_CALLBACK: RefCell<Option<RenderCallback>> = RefCell::new(None);

pub fn set_render_callback(callback: RenderCallback) {
    *RENDER_CALLBACK.borrow_mut() = Some(callback);
}

cef::wrap_client! {
    pub struct IronClient {
        state: SharedClientState,
    }

    impl Client {
        fn life_span_handler(&self) -> Option<cef::LifeSpanHandler> {
            Some(IronLifeSpanHandler::new(self.state.clone()))
        }

        fn load_handler(&self) -> Option<cef::LoadHandler> {
            Some(IronLoadHandler::new(self.state.clone()))
        }

        fn display_handler(&self) -> Option<cef::DisplayHandler> {
            Some(IronDisplayHandler::new(self.state.clone()))
        }

        fn render_handler(&self) -> Option<cef::RenderHandler> {
            Some(IronRenderHandler::new())
        }

        fn focus_handler(&self) -> Option<cef::FocusHandler> {
            Some(IronFocusHandler::new())
        }

        fn download_handler(&self) -> Option<cef::DownloadHandler> {
            Some(IronDownloadHandler::new())
        }
    }
}

cef::wrap_life_span_handler! {
    pub struct IronLifeSpanHandler {
        state: SharedClientState,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, _browser: Option<&mut cef::Browser>) {
            eprintln!("[CEF] Browser created");
            BROWSER_CREATED.store(true, Ordering::SeqCst);
        }

        fn on_before_close(&self, _browser: Option<&mut cef::Browser>) {
            eprintln!("[CEF] Browser closing");
            BROWSER_CREATED.store(false, Ordering::SeqCst);
        }
    }
}

use std::sync::atomic::AtomicUsize;

static PAGE_LOAD_COUNT: AtomicUsize = AtomicUsize::new(0);

cef::wrap_load_handler! {
    pub struct IronLoadHandler {
        state: SharedClientState,
    }

    impl LoadHandler {
        fn on_load_start(
            &self,
            browser: Option<&mut cef::Browser>,
            _frame: Option<&mut cef::Frame>,
            _transition_type: cef::TransitionType,
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
            browser: Option<&mut cef::Browser>,
            frame: Option<&mut cef::Frame>,
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
            _browser: Option<&mut cef::Browser>,
            is_loading: bool,
            can_go_back: bool,
            can_go_forward: bool,
        ) {
            if let Some(cb) = self.state.borrow_mut().on_loading_state_change.as_ref() {
                cb(is_loading, can_go_back, can_go_forward);
            }
        }
    }
}

cef::wrap_display_handler! {
    pub struct IronDisplayHandler {
        state: SharedClientState,
    }

    impl DisplayHandler {
        fn on_title_change(
            &self,
            _browser: Option<&mut cef::Browser>,
            title: Option<&cef::CefString>,
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
            _browser: Option<&mut cef::Browser>,
            _frame: Option<&mut cef::Frame>,
            url: Option<&cef::CefString>,
        ) {
            if let Some(u) = url {
                let url_str = u.to_string();
                if let Some(cb) = self.state.borrow_mut().on_url_change.as_ref() {
                    cb(&url_str);
                }
            }
        }
    }
}

cef::wrap_render_handler! {
    pub struct IronRenderHandler;

    impl RenderHandler {
        fn on_paint(
            &self,
            _browser: Option<&mut cef::Browser>,
            _kind: cef::PaintElementType,
            _dirty_rects: &[cef::Rect],
            buffer: Option<&[u8]>,
            width: i32,
            height: i32,
        ) {
            let Some(buffer) = buffer else { return };
            if let Some(ref mut callback) = *RENDER_CALLBACK.borrow_mut() {
                callback(buffer, width, height);
            }
        }
    }
}

cef::wrap_focus_handler! {
    pub struct IronFocusHandler;

    impl FocusHandler {
        fn on_set_focus(
            &self,
            _browser: Option<&mut cef::Browser>,
            _source: cef::FocusSource,
        ) -> i32 {
            0
        }
    }
}

cef::wrap_download_handler! {
    pub struct IronDownloadHandler;

    impl DownloadHandler {
        fn on_before_download(
            &self,
            _browser: Option<&mut cef::Browser>,
            _download_item: Option<&mut cef::DownloadItem>,
            _suggested_name: Option<&cef::CefString>,
        ) -> cef::BeforeDownloadCallback {
            eprintln!("[CEF] Download requested");
            cef::BeforeDownloadCallback::default()
        }

        fn on_download_updated(
            &self,
            _browser: Option<&mut cef::Browser>,
            download_item: Option<&mut cef::DownloadItem>,
            _callback: Option<&mut cef::DownloadCallback>,
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
}

pub fn is_browser_created() -> bool {
    BROWSER_CREATED.load(Ordering::SeqCst)
}
