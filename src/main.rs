mod command;
mod config;
mod hints;
mod noctalia;

use command::CommandInput;
use config::Config;
use hints::HintManager;
use noctalia::ThemeManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::{EventControllerKey, gdk, Box as GtkBox, CssProvider, Overlay, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4::prelude::WidgetExt;
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

        let hints: Rc<RefCell<HintManager>> = Rc::new(RefCell::new(HintManager::new()));
        let cmd_bar: Rc<RefCell<Option<GtkBox>>> = Rc::new(RefCell::new(None));
        let cmd_entry: Rc<RefCell<Option<gtk4::Entry>>> = Rc::new(RefCell::new(None));

        let css_provider = CssProvider::new();
        css_provider.load_from_string(
            ".command-bar { background: rgba(30, 30, 40, 0.95); border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; padding: 4px 8px; }",
        );

        let overlay_for_cmd = overlay.clone();
        let overlay_for_esc = overlay.clone();
        let hints_clone = hints.clone();
        let cmd_bar_clone = cmd_bar.clone();
        let cmd_entry_clone = cmd_entry.clone();
        let css_provider_clone = css_provider.clone();
        let wv_weak = webview.downgrade();

        let key_ctl = EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, keyval, _keycode, modifier| {
            let hints_active = hints_clone.borrow().active;

            if hints_active {
                match keyval {
                    gdk::Key::Escape => {
                        if let Some(wv) = wv_weak.upgrade() {
                            hints_clone.borrow_mut().deactivate(&wv);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::BackSpace => {
                        if let Some(wv) = wv_weak.upgrade() {
                            hints_clone.borrow_mut().handle_backspace(&wv);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                        if let Some(wv) = wv_weak.upgrade() {
                            hints_clone.borrow_mut().deactivate(&wv);
                        }
                        return glib::Propagation::Stop;
                    }
                    _ if keyval.to_unicode().is_some_and(|c| c.is_ascii_graphic()) => {
                        if let Some(c) = keyval.to_unicode() {
                            if let Some(wv) = wv_weak.upgrade() {
                                hints_clone.borrow_mut().handle_key(c, &wv);
                            }
                        }
                        return glib::Propagation::Stop;
                    }
                    _ => {
                        if let Some(wv) = wv_weak.upgrade() {
                            hints_clone.borrow_mut().deactivate(&wv);
                        }
                        return glib::Propagation::Stop;
                    }
                }
            }

            if let Some(binding) = Config::load().get_binding_by_keyval(keyval, modifier) {
                match binding.action.as_str() {
                    "hint" => {
                        if let Some(wv) = wv_weak.upgrade() {
                            hints_clone.borrow_mut().activate(&wv);
                        }
                        return glib::Propagation::Stop;
                    }
                    "command" => {
                        if cmd_bar_clone.borrow().is_some() {
                            return glib::Propagation::Proceed;
                        }

                        let entry = gtk4::Entry::new();
                        entry.set_placeholder_text(Some(":open "));
                        entry.set_width_chars(60);
                        entry.set_halign(gtk4::Align::Center);
                        entry.set_valign(gtk4::Align::Start);
                        entry.set_margin_top(10);
                        entry.set_margin_start(80);
                        entry.set_margin_end(80);
                        entry.add_css_class("command-bar");
                        entry.style_context().add_provider(&css_provider_clone, STYLE_PROVIDER_PRIORITY_APPLICATION);

                        let container = GtkBox::new(gtk4::Orientation::Horizontal, 0);
                        container.add_css_class("command-bar");
                        container.style_context().add_provider(&css_provider_clone, STYLE_PROVIDER_PRIORITY_APPLICATION);
                        container.append(&entry);

                        let wv_for_cmd = wv_weak.clone();
                        let cmd_bar_c = cmd_bar_clone.clone();
                        let cmd_entry_c = cmd_entry_clone.clone();

                        entry.connect_activate(move |e| {
                            let text = e.text().to_string();
                            let input = CommandInput::new(&text);
                            if let Some(cmd) = input.parse() {
                                if let Some(w) = wv_for_cmd.upgrade() {
                                    match cmd {
                                        command::Command::Open(url) => w.load_uri(&url),
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
                            if let Some(bar) = cmd_bar_c.borrow_mut().take() {
                                bar.unparent();
                            }
                            cmd_entry_c.borrow_mut().take();
                            if let Some(w) = wv_for_cmd.upgrade() {
                                w.grab_focus();
                            }
                        });

                        let entry_key_ctl = EventControllerKey::new();
                        entry.add_controller(entry_key_ctl.clone());
                        let cmd_bar_esc = cmd_bar_clone.clone();
                        let cmd_entry_esc = cmd_entry_clone.clone();
                        let wv_weak_esc = wv_weak.clone();
                        entry_key_ctl.connect_key_pressed(move |_, k, _, _| {
                            if k == gdk::Key::Escape {
                                if let Some(bar) = cmd_bar_esc.borrow_mut().take() {
                                    bar.unparent();
                                }
                                cmd_entry_esc.borrow_mut().take();
                                if let Some(w) = wv_weak_esc.upgrade() {
                                    w.grab_focus();
                                }
                                return glib::Propagation::Stop;
                            }
                            glib::Propagation::Proceed
                        });

                        *cmd_bar_clone.borrow_mut() = Some(container.clone());
                        *cmd_entry_clone.borrow_mut() = Some(entry.clone());
                        overlay.add_overlay(&container);
                        entry.grab_focus();

                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }

            glib::Propagation::Proceed
        });
        window.add_controller(key_ctl);

        window.present();

        ThemeManager::start_watch(tm, &webview);
    });

    app.run();
}