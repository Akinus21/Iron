#![cfg(feature = "cef-stub")]

use std::cell::RefCell;
use std::rc::Rc;

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

pub fn set_render_callback(_callback: Option<Box<dyn Fn(&[u8], i32, i32)>>) {
}

#[derive(Clone)]
pub struct IronClient;

impl IronClient {
    pub fn new(_state: SharedClientState) -> Self {
        Self
    }
}

pub fn is_browser_created() -> bool {
    false
}