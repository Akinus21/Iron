mod command;
mod config;
mod hints;
mod noctalia;
mod settings;

use command::CommandInput;
use config::Config;
use hints::HintManager;
use noctalia::ThemeManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::{
    Align, Box as GtkBox, CssProvider, Entry, EventControllerKey, gdk, Label, ListBox,
    ListBoxRow, Orientation, Overlay, ScrolledWindow, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4::prelude::WidgetExt;
use webkit6::prelude::*;

fn main() {
    let app = adw::Application::new(Some("org.blueak.iron"), gio::ApplicationFlags::default());

    app.connect_activate(|app| {
        let cfg = Rc::new(RefCell::new(Config::load()));
        let _win = build_window(app, cfg.clone(), Some("https://www.rust-lang.org"));
    });

    app.run();
}

fn build_window(
    app: &adw::Application,
    cfg: Rc<RefCell<Config>>,
    initial_url: Option<&str>,
) -> adw::ApplicationWindow {
    let tm = Rc::new(RefCell::new(ThemeManager::new()));
    tm.borrow_mut().load();

    let window = adw::ApplicationWindow::new(app);
    window.set_default_size(1024, 768);
    window.set_title(Some("Iron"));

    let overlay = Overlay::new();

    let webview = webkit6::WebView::builder()
        .user_content_manager(&webkit6::UserContentManager::new())
        .build();

    // Dark mode preference is communicated via the injected CSS stylesheet
    // (color-scheme: dark) rather than a WebKit API setting.

    tm.borrow().apply_webkit_css(&webview);
    let url = initial_url.unwrap_or("https://www.rust-lang.org");
    webview.load_uri(url);

    overlay.set_child(Some(&webview));
    window.set_content(Some(&overlay));

    let hints: Rc<RefCell<HintManager>> = Rc::new(RefCell::new(HintManager::new()));
    let cmd_overlay: Rc<RefCell<Option<GtkBox>>> = Rc::new(RefCell::new(None));

    let css_provider = CssProvider::new();
    css_provider.load_from_string(
        ".command-overlay { opacity: 0.97; padding: 24px; }\n\
         .command-section { margin-top: 12px; margin-bottom: 12px; }\n\
         .command-row { padding: 6px 12px; }\n\
         .command-help { opacity: 0.65; }",
    );

    let hints_clone = hints.clone();
    let cmd_overlay_clone = cmd_overlay.clone();
    let css_provider_clone = css_provider.clone();
    let wv_weak = webview.downgrade();
    let cfg_clone = cfg.clone();
    let app_clone = app.clone(); // own the Application so the closure can be 'static

    let key_ctl = EventControllerKey::new();
    key_ctl.connect_key_pressed(move |_, keyval, _keycode, modifier| {
        let hints_active = hints_clone.borrow().active;

        // Always reload config before resolving a binding so edits take effect immediately
        cfg_clone.borrow_mut().reload();

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

        if let Some(binding) = cfg_clone.borrow().get_binding_by_keyval(keyval, &modifier) {
            match binding.action.as_str() {
                "hint" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        hints_clone.borrow_mut().activate(&wv);
                    }
                    return glib::Propagation::Stop;
                }
                "command" => {
                    if cmd_overlay_clone.borrow().is_some() {
                        return glib::Propagation::Proceed;
                    }

                    let full_overlay = GtkBox::new(Orientation::Vertical, 0);
                    full_overlay.add_css_class("command-overlay");
                    full_overlay.add_css_class("background");
                    full_overlay.style_context().add_provider(
                        &css_provider_clone,
                        STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                    full_overlay.set_halign(Align::Fill);
                    full_overlay.set_valign(Align::Fill);

                    // --- Search entry (top) ---
                    let entry = Entry::new();
                    entry.set_placeholder_text(Some("Type a command..."));
                    entry.set_margin_top(24);
                    entry.set_margin_start(80);
                    entry.set_margin_end(80);
                    entry.add_css_class("heading");
                    full_overlay.append(&entry);

                    // --- Scrollable content ---
                    let scroll = ScrolledWindow::builder().vexpand(true).build();
                    let content = GtkBox::new(Orientation::Vertical, 8);
                    content.set_margin_start(80);
                    content.set_margin_end(80);
                    content.set_margin_bottom(24);

                    // Section: Current keybindings
                    let kb_title = Label::new(Some("Key Bindings"));
                    kb_title.add_css_class("title-2");
                    kb_title.set_halign(Align::Start);
                    content.append(&kb_title);

                    let kb_list = ListBox::new();
                    kb_list.set_selection_mode(gtk4::SelectionMode::None);
                    for b in &cfg_clone.borrow().normal.bindings {
                        let row = ListBoxRow::new();
                        let h = GtkBox::new(Orientation::Horizontal, 8);
                        let mod_lbl = Label::new(Some(&format!(
                            "{}",
                            if b.modifier.is_empty() {
                                "—".to_string()
                            } else {
                                b.modifier.join(" ").to_uppercase()
                            }
                        )));
                        mod_lbl.add_css_class("monospace");
                        mod_lbl.set_width_chars(12);
                        let key_lbl = Label::new(Some(&b.key));
                        key_lbl.add_css_class("monospace");
                        key_lbl.set_width_chars(10);
                        let act_lbl = Label::new(Some(&b.action));
                        act_lbl.add_css_class("caption");
                        act_lbl.add_css_class("command-help");
                        h.append(&mod_lbl);
                        h.append(&key_lbl);
                        h.append(&act_lbl);
                        row.set_child(Some(&h));
                        kb_list.append(&row);
                    }
                    content.append(&kb_list);

                    // Section: Available commands
                    let cmd_title = Label::new(Some("Commands"));
                    cmd_title.add_css_class("title-2");
                    cmd_title.set_halign(Align::Start);
                    cmd_title.set_margin_top(12);
                    content.append(&cmd_title);

                    let cmd_list = ListBox::new();
                    cmd_list.set_selection_mode(gtk4::SelectionMode::None);
                    for (name, desc) in &[
                        (":open URL", "Load URL in current window"),
                        (":new-window-open URL (nwo)", "Open URL in a new BlueAK window"),
                        (":back (b)", "Go back in history"),
                        (":forward (f)", "Go forward in history"),
                        (":reload (r)", "Reload the current page"),
                        (":settings (set)", "Open the settings window"),
                    ] {
                        let row = ListBoxRow::new();
                        let h = GtkBox::new(Orientation::Horizontal, 8);
                        let n = Label::new(Some(*name));
                        n.add_css_class("monospace");
                        n.set_width_chars(30);
                        let d = Label::new(Some(*desc));
                        d.add_css_class("caption");
                        d.add_css_class("command-help");
                        h.append(&n);
                        h.append(&d);
                        row.set_child(Some(&h));
                        cmd_list.append(&row);
                    }
                    content.append(&cmd_list);

                    scroll.set_child(Some(&content));
                    full_overlay.append(&scroll);

                    // --- Bottom escape hint ---
                    let esc_hint =
                        Label::new(Some("Press Escape to close this overlay"));
                    esc_hint.add_css_class("caption");
                    esc_hint.add_css_class("command-help");
                    esc_hint.set_margin_bottom(12);
                    full_overlay.append(&esc_hint);

                    let wv_for_cmd = wv_weak.clone();
                    let cmd_overlay_c = cmd_overlay_clone.clone();
                    let cfg_cmd = cfg_clone.clone();
                    let app_for_cmd = app_clone.clone();

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
                                    command::Command::Settings => {
                                        if let Some(window) = w.root().and_downcast::<adw::ApplicationWindow>() {
                                            settings::show_settings_window(
                                                &window,
                                                cfg_cmd.clone(),
                                            );
                                        }
                                    }
                                    command::Command::NewWindowOpen(url) => {
                                        let _ = build_window(
                                            &app_for_cmd,
                                            cfg_cmd.clone(),
                                            Some(&url),
                                        );
                                    }
                                }
                            }
                        }
                        if let Some(bar) = cmd_overlay_c.borrow_mut().take() {
                            bar.unparent();
                        }
                        if let Some(w) = wv_for_cmd.upgrade() {
                            w.grab_focus();
                        }
                    });

                    let entry_key_ctl = EventControllerKey::new();
                    entry.add_controller(entry_key_ctl.clone());
                    let cmd_overlay_esc = cmd_overlay_clone.clone();
                    let wv_weak_esc = wv_weak.clone();
                    entry_key_ctl.connect_key_pressed(move |_, k, _, _| {
                        if k == gdk::Key::Escape {
                            if let Some(bar) = cmd_overlay_esc.borrow_mut().take() {
                                bar.unparent();
                            }
                            if let Some(w) = wv_weak_esc.upgrade() {
                                w.grab_focus();
                            }
                            return glib::Propagation::Stop;
                        }
                        glib::Propagation::Proceed
                    });

                    *cmd_overlay_clone.borrow_mut() = Some(full_overlay.clone());
                    overlay.add_overlay(&full_overlay);
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

    window
}

/// If a hint is currently matched (single visible hint), open that link
/// in a new ApplicationWindow.
fn open_current_hint_in_new_window(
    _webview: &webkit6::WebView,
    _hints: &HintManager,
    _app: &adw::Application,
    _cfg: Rc<RefCell<Config>>,
) {
    // TODO wire this up once hints track the currently matched element
}