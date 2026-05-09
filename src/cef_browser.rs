use gtk4::prelude::*;
use gtk4::{Widget, gdk, glib, EventControllerKey, EventControllerMotion, GestureClick, EventControllerScroll};
use gtk4::gdk_pixbuf::Pixbuf;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "cef-stub"))]
use crate::cef_client::{self, IronClient, SharedClientState};
#[cfg(feature = "cef-stub")]
use crate::cef_client_stub::{self, IronClient, SharedClientState};
use cef::{ImplBrowser, ImplBrowserHost, ImplFrame};

#[derive(Clone)]
pub struct CefBrowserWrapper {
    pub widget: Widget,
    pub client_state: SharedClientState,
    browser: Rc<RefCell<Option<cef::Browser>>>,
    url: Rc<RefCell<String>>,
    title: Rc<RefCell<String>>,
    is_loading: Rc<RefCell<bool>>,
    can_go_back: Rc<RefCell<bool>>,
    can_go_forward: Rc<RefCell<bool>>,
    has_focus: Rc<RefCell<bool>>,
    paint_buffer: Rc<RefCell<Option<Vec<u8>>>>,
    buffer_width: Rc<RefCell<i32>>,
    buffer_height: Rc<RefCell<i32>>,
}

impl CefBrowserWrapper {
    pub fn new(
        parent_window: Option<&gdk::Surface>,
        url: &str,
        is_offscreen: bool,
    ) -> Result<Self, String> {
        #[cfg(not(feature = "cef-stub"))]
        let client_state = cef_client::create_shared_state();
        #[cfg(feature = "cef-stub")]
        let client_state = cef_client_stub::create_shared_state();

        let picture = gtk4::Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let url_str = url.to_string();
        let title_str = format!("Iron - {}", url_str);

        let wrapper = Self {
            widget: picture.upcast(),
            browser: Rc::new(RefCell::new(None)),
            client_state: client_state.clone(),
            url: Rc::new(RefCell::new(url_str.clone())),
            title: Rc::new(RefCell::new(title_str)),
            is_loading: Rc::new(RefCell::new(true)),
            can_go_back: Rc::new(RefCell::new(false)),
            can_go_forward: Rc::new(RefCell::new(false)),
            has_focus: Rc::new(RefCell::new(false)),
            paint_buffer: Rc::new(RefCell::new(None)),
            buffer_width: Rc::new(RefCell::new(0)),
            buffer_height: Rc::new(RefCell::new(0)),
        };

        let is_loading_clone = wrapper.is_loading.clone();
        let can_go_back_clone = wrapper.can_go_back.clone();
        let can_go_forward_clone = wrapper.can_go_forward.clone();

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

        let picture = wrapper.widget.downcast_ref::<gtk4::Picture>().unwrap().clone();

        #[cfg(not(feature = "cef-stub"))]
        {
            let _paint_buffer = wrapper.paint_buffer.clone();
            let buffer_width = wrapper.buffer_width.clone();
            let buffer_height = wrapper.buffer_height.clone();

            struct RenderData {
                buffer: Option<Vec<u8>>,
                width: i32,
                height: i32,
            }

            let render_data = Arc::new(Mutex::new(RenderData { buffer: None, width: 0, height: 0 }));
            let render_data_cb = render_data.clone();
            let render_data_poll = render_data.clone();

            let render_callback: Arc<Mutex<Box<dyn FnMut(&[u8], i32, i32) + Send + Sync>>> =
                Arc::new(Mutex::new(Box::new(move |buffer: &[u8], width: i32, height: i32| {
                    let mut d = render_data_cb.lock().unwrap();
                    d.buffer = Some(buffer.to_vec());
                    d.width = width;
                    d.height = height;
                })));

            let rc = render_callback.clone();
            crate::cef_client::set_render_callback(Some(Box::new(move |buffer: &[u8], width: i32, height: i32| {
                if let Ok(mut guard) = rc.lock() {
                    guard(buffer, width, height);
                }
            })));
            let pw = buffer_width.clone();
            let ph = buffer_height.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                let mut d = render_data_poll.lock().unwrap();
                if let Some(rgba) = d.buffer.take() {
                    let w = *pw.borrow();
                    let h = *ph.borrow();
                    if w > 0 && h > 0 {
                        let bytes = glib::Bytes::from(&rgba);
                        let pixbuf = Pixbuf::from_bytes(
                            &bytes,
                            gtk4::gdk_pixbuf::Colorspace::Rgb,
                            true,
                            8,
                            w,
                            h,
                            w * 4,
                        );
                        let texture = gdk::Texture::for_pixbuf(&pixbuf);
                        picture.set_paintable(Some(&texture));
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        #[cfg(not(feature = "cef-stub"))]
        let mut _client = IronClient::new(client_state.clone());

        let window_info = cef::WindowInfo::default();
        let window_info = if is_offscreen || parent_window.is_none() {
            let mut wi = window_info;
            wi.windowless_rendering_enabled = 1;
            wi.set_as_windowless(0)
        } else if let Some(surface) = parent_window {
            let win_id = get_window_handle(surface);
            window_info.set_as_child(win_id, &cef::Rect::default())
        } else {
            window_info
        };

        let browser_settings = cef::BrowserSettings::default();
        let cef_url = cef::CefString::from(url_str.as_str());

        #[cfg(not(feature = "cef-stub"))]
        let result = cef::browser_host_create_browser(
            Some(&window_info),
            Some(&mut _client),
            Some(&cef_url),
            Some(&browser_settings),
            None,
            None,
        );

        #[cfg(not(feature = "cef-stub"))]
        if result != 1 {
            eprintln!("[CEF] Failed to create browser (result={})", result);
            return Err("Failed to create CEF browser".to_string());
        }

        #[cfg(not(feature = "cef-stub"))]
        eprintln!("[CEF] Browser creation initiated for {}", url_str);

        wrapper.setup_input_controllers();

        Ok(wrapper)
    }

    fn setup_input_controllers(&self) {
        let browser_for_key_press = self.browser.clone();
        let browser_for_key_release = self.browser.clone();

        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, keycode, modifier| {
            if let Some(browser) = browser_for_key_press.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let cef_event = build_cef_key_event(keyval, keycode, modifier, true);
                    host.send_key_event(Some(&cef_event));
                }
            }
            glib::Propagation::Stop
        });
        key_controller.connect_key_released(move |_, keyval, keycode, modifier| {
            if let Some(browser) = browser_for_key_release.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let cef_event = build_cef_key_event(keyval, keycode, modifier, false);
                    host.send_key_event(Some(&cef_event));
                }
            }
        });
        self.widget.add_controller(key_controller);

        let browser_for_motion = self.browser.clone();
        let motion_controller = EventControllerMotion::new();
        motion_controller.connect_motion(move |_, x, y| {
            if let Some(browser) = browser_for_motion.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let cef_event = build_cef_mouse_move_event(x as i32, y as i32, 0);
                    host.send_mouse_move_event(Some(&cef_event), 0);
                }
            }
        });
        self.widget.add_controller(motion_controller);

        let browser_for_press = self.browser.clone();
        let browser_for_release = self.browser.clone();

        let click_controller = GestureClick::new();
        click_controller.connect_pressed(move |gesture, _, x, y| {
            if let Some(browser) = browser_for_press.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let button = gesture.current_button();
                    let cef_button = match button { 1 => 0, 2 => 1, 3 => 2, _ => 0 };
                    let cef_event = build_cef_mouse_event(x as i32, y as i32, cef_button);
                    let mouse_button = match cef_button {
                        0 => cef::MouseButtonType::LEFT,
                        1 => cef::MouseButtonType::MIDDLE,
                        _ => cef::MouseButtonType::RIGHT,
                    };
                    host.send_mouse_click_event(Some(&cef_event), mouse_button, 0, 1);
                }
            }
        });
        click_controller.connect_released(move |gesture, _, x, y| {
            if let Some(browser) = browser_for_release.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let button = gesture.current_button();
                    let cef_button = match button { 1 => 0, 2 => 1, 3 => 2, _ => 0 };
                    let cef_event = build_cef_mouse_event(x as i32, y as i32, cef_button);
                    let mouse_button = match cef_button {
                        0 => cef::MouseButtonType::LEFT,
                        1 => cef::MouseButtonType::MIDDLE,
                        _ => cef::MouseButtonType::RIGHT,
                    };
                    host.send_mouse_click_event(Some(&cef_event), mouse_button, 1, 1);
                }
            }
        });
        self.widget.add_controller(click_controller);

        let browser_for_scroll = self.browser.clone();
        let scroll_controller = EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::BOTH_AXES
        );
        scroll_controller.connect_scroll(move |_, dx, dy| {
            if let Some(browser) = browser_for_scroll.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    let delta_x = (dx * 100.0) as i32;
                    let delta_y = (dy * 100.0) as i32;
                    let cef_event = build_cef_mouse_event(0, 0, 0);
                    host.send_mouse_wheel_event(Some(&cef_event), delta_x, delta_y);
                }
            }
            glib::Propagation::Stop
        });
        self.widget.add_controller(scroll_controller);

        let browser_for_enter = self.browser.clone();
        let browser_for_leave = self.browser.clone();
        let has_focus_for_enter = self.has_focus.clone();
        let has_focus_for_leave = self.has_focus.clone();

        let focus_controller = gtk4::EventControllerFocus::new();
        focus_controller.connect_enter(move |_| {
            if let Some(browser) = browser_for_enter.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    host.set_focus(1);
                }
            }
            *has_focus_for_enter.borrow_mut() = true;
        });
        focus_controller.connect_leave(move |_| {
            if let Some(browser) = browser_for_leave.borrow().as_ref() {
                if let Some(host) = browser.host() {
                    host.set_focus(0);
                }
            }
            *has_focus_for_leave.borrow_mut() = false;
        });
        self.widget.add_controller(focus_controller);
    }

    pub fn load_uri(&self, url: &str) {
        *self.url.borrow_mut() = url.to_string();
        *self.title.borrow_mut() = format!("Iron - {}", url);
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(frame) = browser.main_frame() {
                let cef_url = cef::CefString::from(url);
                frame.load_url(Some(&cef_url));
            }
        }
    }

    pub fn reload(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.reload();
        }
    }

    pub fn go_back(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.go_back();
        }
    }

    pub fn go_forward(&self) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            browser.go_forward();
        }
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
        *self.can_go_back.borrow()
    }

    pub fn can_go_forward(&self) -> bool {
        *self.can_go_forward.borrow()
    }

    pub fn execute_js(&self, script: &str) {
        if let Some(browser) = self.browser.borrow().as_ref() {
            if let Some(frame) = browser.main_frame() {
                let code = cef::CefString::from(script);
                let url = cef::CefString::from("about:blank");
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

fn convert_bgra_to_rgba(buffer: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * 4;
            let b = buffer[src_idx];
            let g = buffer[src_idx + 1];
            let r = buffer[src_idx + 2];
            let a = buffer[src_idx + 3];
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    rgba
}

fn build_cef_key_event(keyval: gdk::Key, keycode: u32, modifier: gdk::ModifierType, is_press: bool) -> cef::KeyEvent {
    let mut event = cef::KeyEvent::default();
    event.type_ = if is_press { cef::KeyEventType::KEYDOWN } else { cef::KeyEventType::KEYUP };
    event.modifiers = map_gdk_modifier(modifier);
    event.windows_key_code = keyval_to_windows_key_code(keyval);
    event.native_key_code = keycode as i32;
    if let Some(c) = keyval.to_unicode() {
        event.unmodified_character = c as u16;
        event.character = c as u16;
    }
    event
}

fn build_cef_mouse_event(x: i32, y: i32, button: i32) -> cef::MouseEvent {
    let mut event = cef::MouseEvent::default();
    event.x = x;
    event.y = y;
    event.modifiers = if button == 0 { 0 } else { 1 << button };
    event
}

fn build_cef_mouse_move_event(x: i32, y: i32, modifiers: u32) -> cef::MouseEvent {
    let mut event = cef::MouseEvent::default();
    event.x = x;
    event.y = y;
    event.modifiers = modifiers;
    event
}

fn map_gdk_modifier(modifier: gdk::ModifierType) -> u32 {
    let mut cef_mod = 0u32;
    if modifier.contains(gdk::ModifierType::CONTROL_MASK) { cef_mod |= 1; }
    if modifier.contains(gdk::ModifierType::SHIFT_MASK)   { cef_mod |= 2; }
    if modifier.contains(gdk::ModifierType::ALT_MASK)     { cef_mod |= 4; }
    if modifier.contains(gdk::ModifierType::SUPER_MASK)   { cef_mod |= 8; }
    cef_mod
}

fn keyval_to_windows_key_code(keyval: gdk::Key) -> i32 {
    match keyval {
        gdk::Key::Return | gdk::Key::KP_Enter => 13,
        gdk::Key::Tab       => 9,
        gdk::Key::Escape    => 27,
        gdk::Key::BackSpace => 8,
        gdk::Key::Delete    => 46,
        gdk::Key::Insert    => 45,
        gdk::Key::Home      => 36,
        gdk::Key::End       => 35,
        gdk::Key::Page_Up   => 33,
        gdk::Key::Page_Down => 34,
        gdk::Key::Left      => 37,
        gdk::Key::Up        => 38,
        gdk::Key::Right     => 39,
        gdk::Key::Down      => 40,
        gdk::Key::F1  => 112, gdk::Key::F2  => 113, gdk::Key::F3  => 114,
        gdk::Key::F4  => 115, gdk::Key::F5  => 116, gdk::Key::F6  => 117,
        gdk::Key::F7  => 118, gdk::Key::F8  => 119, gdk::Key::F9  => 120,
        gdk::Key::F10 => 121, gdk::Key::F11 => 122, gdk::Key::F12 => 123,
        _ => {
            if let Some(c) = keyval.to_unicode() { c as i32 } else { 0 }
        }
    }
}

#[cfg(target_os = "linux")]
fn get_window_handle(_surface: &gdk::Surface) -> u64 { 0 }

#[cfg(not(target_os = "linux"))]
fn get_window_handle(_surface: &gdk::Surface) -> u64 { 0 }