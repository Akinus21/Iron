mod hints;
mod noctalia;

use hints::HintManager;
use noctalia::ThemeManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use glib::Propagation;
use gtk4::EventControllerKey;
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
        let key_ctl = gtk4::EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, keyval, _keycode, modifier| {
            let Some(wv) = wv_weak.upgrade() else {
                return Propagation::proceed;
            };
            let mut h = hints_clone.borrow_mut();

            if h.active {
                match keyval {
                    gtk4::gdk::Key::Escape => {
                        h.deactivate(&wv);
                        return Propagation::stop;
                    }
                    gtk4::gdk::Key::BackSpace => {
                        h.handle_backspace(&wv);
                        return Propagation::stop;
                    }
                    gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter | gtk4::gdk::Key::ISO_Enter => {
                        h.deactivate(&wv);
                        return Propagation::stop;
                    }
                    _ if keyval.to_unicode().is_some_and(|c| c.is_ascii_graphic()) => {
                        if let Some(c) = keyval.to_unicode() {
                            h.handle_key(c, &wv);
                        }
                        return Propagation::stop;
                    }
                    _ => {
                        h.deactivate(&wv);
                        return Propagation::stop;
                    }
                }
            }

            if keyval == gtk4::gdk::Key::F && modifier.is_empty() {
                h.activate(&wv);
                return Propagation::stop;
            }

            Propagation::proceed
        });
        webview.add_controller(&key_ctl);

        window.present();

        ThemeManager::start_watch(tm, &webview);
    });

    app.run();
}
