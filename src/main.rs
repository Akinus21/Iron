mod command;
mod hints;
mod noctalia;

use command::CommandInput;
use hints::HintManager;
use noctalia::ThemeManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gio::prelude::*;
use gtk4::{EventControllerKey, gdk, Overlay};
use webkit6::prelude::*;

fn main() {
    let app = adw::Application::new(Some("org.blueak.iron"), gio::ApplicationFlags::default());

    app.connect_activate(|app| {
        let tm = Rc::new(RefCell::new(ThemeManager::new()));
        tm.borrow_mut().load();

        let window = adw::ApplicationWindow::new(app);
        window.set_default_size(1024, 768);
        window.set_title(Some("Iron"));

        let overlay = Overlay::new();

        let webview = webkit6::WebView::builder()
            .user_content_manager(&webkit6::UserContentManager::new())
            .build();

        tm.borrow().apply_webkit_css(&webview);
        webview.load_uri("https://www.rust-lang.org");

        overlay.set_child(Some(&webview));
        window.set_content(Some(&overlay));

        let hints = Rc::new(RefCell::new(HintManager::new()));
        let cmd_entry: Rc<RefCell<Option<gtk4::Entry>>> = Rc::new(RefCell::new(None));

        let hints_clone = hints.clone();
        let wv_weak = webview.downgrade();
        let overlay_clone = overlay.downgrade();
        let cmd_entry_clone = cmd_entry.clone();
        let key_ctl = EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, keyval, _keycode, modifier| {
            let Some(wv) = wv_weak.upgrade() else {
                return glib::Propagation::Proceed;
            };

            if keyval == gdk::Key::F && modifier.is_empty() {
                hints_clone.borrow_mut().activate(&wv);
                return glib::Propagation::Stop;
            }

            if keyval == gdk::Key::colon && modifier.contains(gdk::ModifierType::SHIFT_MASK) {
                let overlay = match overlay_clone.upgrade() {
                    Some(a) => a,
                    None => return glib::Propagation::Proceed,
                };

                let entry = gtk4::Entry::new();
                entry.set_placeholder_text(Some(":"));
                entry.set_width_chars(60);
                entry.set_halign(gtk4::Align::Center);
                entry.set_valign(gtk4::Align::Start);
                entry.set_margin_top(10);
                entry.set_margin_start(100);
                entry.set_margin_end(100);

                let wv_for_cmd = wv.downgrade();
                let entry_for_closure = entry.clone();
                let cmd_entry_c = cmd_entry_clone.clone();

                entry.connect_activate(move |e| {
                    let text = e.text().to_string();
                    let input = CommandInput::new(&text);
                    if let Some(cmd) = input.parse() {
                        if let Some(w) = wv_for_cmd.upgrade() {
                            match cmd {
                                command::Command::Open(url) => {
                                    w.load_uri(&url);
                                }
                                command::Command::Back => {
                                    if w.can_go_back() {
                                        w.go_back();
                                    }
                                }
                                command::Command::Forward => {
                                    if w.can_go_forward() {
                                        w.go_forward();
                                    }
                                }
                                command::Command::Reload => {
                                    w.reload();
                                }
                            }
                        }
                    }
                    if let Some(old) = cmd_entry_c.borrow_mut().take() {
                        drop(old);
                    }
                });

                let cmd_entry_e = cmd_entry_clone.clone();
                entry.connect_key_pressed(move |_, k, _, _| {
                    if k == gdk::Key::Escape {
                        if let Some(old) = cmd_entry_e.borrow_mut().take() {
                            drop(old);
                        }
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                });

                *cmd_entry_clone.borrow_mut() = Some(entry.clone());
                overlay.add_overlay(&entry);
                entry.grab_focus();

                return glib::Propagation::Stop;
            }

            let mut h = hints_clone.borrow_mut();
            if h.active {
                match keyval {
                    gdk::Key::Escape => {
                        h.deactivate(&wv);
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::BackSpace => {
                        h.handle_backspace(&wv);
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                        h.deactivate(&wv);
                        return glib::Propagation::Stop;
                    }
                    _ if keyval.to_unicode().is_some_and(|c| c.is_ascii_graphic()) => {
                        if let Some(c) = keyval.to_unicode() {
                            h.handle_key(c, &wv);
                        }
                        return glib::Propagation::Stop;
                    }
                    _ => {
                        h.deactivate(&wv);
                        return glib::Propagation::Stop;
                    }
                }
            }

            glib::Propagation::Proceed
        });
        webview.add_controller(key_ctl);

        window.present();

        ThemeManager::start_watch(tm, &webview);
    });

    app.run();
}
