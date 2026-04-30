mod hints;
mod noctalia;

use hints::HintManager;
use noctalia::ThemeManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::prelude::*;
use webkit6::prelude::*;

fn main() {
    let app = adw::Application::new(Some("org.blueak.iron"), gio::ApplicationFlags::default());

    app.connect_activate(|app| {
        let tm = Rc::new(RefCell::new(ThemeManager::new()));
        tm.borrow_mut().load();

        let window = adw::ApplicationWindow::new(app);
        window.set_default_size(1024, 768);
        window.set_title(Some("Iron"));

        let webview = webkit6::WebView::builder()
            .user_content_manager(&webkit6::UserContentManager::new())
            .build();

        tm.borrow().apply_webkit_css(&webview);
        webview.load_uri("https://www.rust-lang.org");

        window.set_content(Some(&webview));

        let hints = Rc::new(RefCell::new(HintManager::new()));

        let hints_clone = hints.clone();
        let wv_weak = webview.downgrade();
        let key_ctl = gtk4::EventControllerKey::builder()
            .on_key_pressed(move |_, keyval, _keycode, modifier| {
                let Some(wv) = wv_weak.upgrade() else {
                    return gtk4::Propagation::proceed;
                };
                let mut h = hints_clone.borrow_mut();

                if h.active {
                    match keyval {
                        gdk::Key::Escape => {
                            h.deactivate(&wv);
                            return gtk4::Propagation::stop;
                        }
                        gdk::Key::BackSpace => {
                            h.handle_backspace(&wv);
                            return gtk4::Propagation::stop;
                        }
                        gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                            h.deactivate(&wv);
                            return gtk4::Propagation::stop;
                        }
                        _ if keyval.to_unicode().is_some_and(|c| c.is_ascii_graphic()) => {
                            if let Some(c) = keyval.to_unicode() {
                                h.handle_key(c, &wv);
                            }
                            return gtk4::Propagation::stop;
                        }
                        _ => {
                            h.deactivate(&wv);
                            return gtk4::Propagation::stop;
                        }
                    }
                }

                if keyval == gdk::Key::f && modifier.is_empty() {
                    h.activate(&wv);
                    return gtk4::Propagation::stop;
                }

                gtk4::Propagation::proceed
            })
            .build();
        webview.add_controller(&key_ctl);

        window.present();

        ThemeManager::start_watch(tm, &webview);
    });

    app.run();
}
