mod cac;
mod cef_browser;
mod cef_init;
mod command;
mod config;
mod download;
mod find;
mod fuzzy;
mod hints;
mod history;
mod noctalia;
mod search;
mod session;
mod settings;

use command::CommandInput;
use config::Config;
use download::DownloadManager;
use find::FindOverlay;
use hints::HintManager;
use history::HistoryManager;
use noctalia::ThemeManager;
use session::SessionManager;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::{
    Align, Box as GtkBox, CssProvider, Entry, EventControllerKey, gdk, Label, ListBox,
    ListBoxRow, Orientation, Overlay, ScrolledWindow, SelectionMode, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4::prelude::{WidgetExt, GtkWindowExt};

fn main() {
    let app = adw::Application::new(
        Some("org.blueak.iron"),
        gio::ApplicationFlags::HANDLES_OPEN,
    );

    app.connect_activate(move |app| {
        let cfg = Rc::new(RefCell::new(Config::load()));
        let session_mgr = session::build_session_mgr();
        let history_mgr = Rc::new(RefCell::new(HistoryManager::new()));
        let args: Vec<String> = std::env::args().collect();
        let urls: Vec<&str> = args.iter()
            .skip(1)
            .map(|s| s.as_str())
            .filter(|s| s.starts_with("http://") || s.starts_with("https://"))
            .collect();

        if urls.is_empty() {
            let _win = build_window(app, cfg.clone(), session_mgr.clone(), history_mgr.clone(), None);
        } else {
            for url in urls {
                let _win = build_window(app, cfg.clone(), session_mgr.clone(), history_mgr.clone(), Some(url));
            }
        }
    });

    app.connect_open(|app, files, _hint| {
        let cfg = Rc::new(RefCell::new(Config::load()));
        let session_mgr = session::build_session_mgr();
        let history_mgr = Rc::new(RefCell::new(HistoryManager::new()));
        for file in files {
            let uri = file.uri();
            let _win = build_window(app, cfg.clone(), session_mgr.clone(), history_mgr.clone(), Some(&uri));
        }
    });

    let open_folder_action = gio::SimpleAction::new("open-folder", Some(&glib::VariantTy::STRING));
    open_folder_action.connect_activate(move |_action, param| {
        if let Some(variant) = param {
            if let Some(folder) = variant.str() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(folder)
                    .spawn();
            }
        }
    });
    app.add_action(&open_folder_action);

    app.run();
}

const ALL_COMMANDS: [(&str, &str); 18] = [
    ("duplicate", "Duplicate current window"),
    ("copy-address", "Copy current page URL"),
    ("downloads", "Show recent downloads"),
    ("find", "Find text in page"),
    ("open", "Load URL in current window"),
    ("new-window-open", "Open URL in new window"),
    ("search", "Search using default engine"),
    ("search-add", "Add a search engine"),
    ("search-del", "Remove a search engine"),
    ("back", "Go back in history"),
    ("forward", "Go forward in history"),
    ("reload", "Reload the page"),
    ("settings", "Open keybinding editor"),
    ("default-browser", "Set as default browser"),
    ("cac-status", "Check smart-card readiness"),
    ("clear-site-data", "Clear all site data"),
    ("clear-cookies", "Clear cookies only"),
    ("reload-theme", "Reload Noctalia theme manually"),
];

#[derive(Clone, Copy, PartialEq)]
enum OverlaySection { Command, History }

struct OverlayState {
    selected_cmd: i32,
    selected_hist: i32,
    active: OverlaySection,
    cmd_navigated: bool,
    hist_navigated: bool,
}

fn build_window(
    app: &adw::Application,
    cfg: Rc<RefCell<Config>>,
    _session_mgr: Rc<RefCell<SessionManager>>,
    history_mgr: Rc<RefCell<HistoryManager>>,
    initial_url: Option<&str>,
) -> adw::ApplicationWindow {
    let tm = Rc::new(RefCell::new(ThemeManager::new()));
    tm.borrow_mut().load();

    let window = adw::ApplicationWindow::new(app);
    window.set_default_size(1024, 768);
    window.set_title(Some("Iron"));
    let _icon_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("res/org.blueak.iron.svg")))
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("res/org.blueak.iron.svg"));

    let overlay = Overlay::new();

    // Initialize CEF on first window creation
    let cef_config = cef_init::CefConfig {
        track: cfg.borrow().cef_track.to_string(),
        enable_window_sleep: cfg.borrow().enable_window_sleep,
        ..Default::default()
    };
    
    if let Err(e) = cef_init::initialize_cef(&cef_config) {
        eprintln!("CEF initialization warning: {}", e);
    }

    // Create CEF browser wrapper (placeholder until full integration)
    let url = initial_url.unwrap_or(&cfg.borrow().home_page);
    let browser = cef_browser::CefBrowserWrapper::new(
        window.surface().as_ref(),
        url,
        false, // off-screen rendering disabled for now
    ).unwrap_or_else(|e| {
        eprintln!("Failed to create CEF browser: {}", e);
        // Fallback: create empty box
        cef_browser::CefBrowserWrapper::new(
            window.surface().as_ref(),
            "about:blank",
            false,
        ).unwrap()
    });

    // ---- History tracking (CEF version) ----
    let browser_hist = browser.clone();
    let hist_mgr_clone = history_mgr.clone();
    
    // Note: CEF doesn't have direct load_changed signals like WebKitGTK
    // We'll track history on URL changes via client handler callbacks
    // For now, add initial URL to history
    hist_mgr_clone.borrow_mut().add(url, Some("Loading..."));

    // Add CEF browser widget to overlay
    overlay.set_child(Some(&browser.widget));

    let download_mgr: Rc<RefCell<DownloadManager>> = Rc::new(RefCell::new(DownloadManager::new()));
    // Note: CEF download handling will be implemented separately
    window.set_content(Some(&overlay));

    let hints: Rc<RefCell<HintManager>> = Rc::new(RefCell::new(HintManager::new()));
    let cmd_overlay: Rc<RefCell<Option<GtkBox>>> = Rc::new(RefCell::new(None));
    let find_overlay: Rc<RefCell<FindOverlay>> = Rc::new(RefCell::new(FindOverlay::new()));

    let noctalia_provider = CssProvider::new();
    tm.borrow().apply_gtk_css(&noctalia_provider);
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::RootExt::display(&window),
        &noctalia_provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let css_provider = CssProvider::new();
    css_provider.load_from_string(
        ".command-overlay { padding: 24px; font-size: 13px; }\n\
         .command-col-title { font-size: 14px; font-weight: 600; opacity: 0.7; margin-bottom: 8px; }\n\
         .command-row { padding: 4px 8px; }\n\
         .command-row-small { font-size: 12px; }\n\
         .command-help { opacity: 0.5; font-size: 12px; }\n\
         .command-overlay, .command-row, .command-selected, .command-col-title {\n\
           transition: background-color 300ms ease-in-out, color 300ms ease-in-out;\n\
         }",
    );
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::RootExt::display(&window),
        &css_provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let hints_clone = hints.clone();
    let cmd_overlay_clone = cmd_overlay.clone();
    let wv_weak = webview.downgrade();
    let cfg_clone = cfg.clone();
    let app_clone = app.clone();
    let find_overlay_clone = find_overlay.clone();
    let overlay_clone = overlay.clone();
    let download_mgr_clone = download_mgr.clone();
    let history_mgr_clone = history_mgr.clone();

    let tm_watch = tm.clone();
    let noctalia_provider_watch = noctalia_provider.clone();

    let key_ctl = EventControllerKey::new();
    key_ctl.connect_key_pressed(move |_, keyval, _keycode, modifier| {
        let hints_active = hints_clone.borrow().active;
        cfg_clone.borrow_mut().reload();

        if hints_active {
            match keyval {
                gdk::Key::Escape => {
                    hints_clone.borrow_mut().deactivate(&browser);
                    return glib::Propagation::Stop;
                }
                gdk::Key::BackSpace => {
                    hints_clone.borrow_mut().handle_backspace(&browser);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                    hints_clone.borrow_mut().commit(&browser);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Down => {
                    hints_clone.borrow_mut().select_next(&browser);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Up => {
                    hints_clone.borrow_mut().select_prev(&browser);
                    return glib::Propagation::Stop;
                }
                _ if keyval.to_unicode().is_some_and(|c| c.is_ascii_graphic()) => {
                    if let Some(c) = keyval.to_unicode() {
                        hints_clone.borrow_mut().handle_key(c, &browser);
                    }
                    return glib::Propagation::Stop;
                }
                _ => {
                    hints_clone.borrow_mut().deactivate(&browser);
                    return glib::Propagation::Stop;
                }
            }
        }

        let find_active = find_overlay_clone.borrow().active;
        if find_active {
            match keyval {
                gdk::Key::Escape => {
                    find_overlay_clone.borrow_mut().deactivate(&overlay_clone);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                    find_overlay_clone.borrow().search_next();
                    return glib::Propagation::Stop;
                }
                gdk::Key::n => {
                    find_overlay_clone.borrow().search_next();
                    return glib::Propagation::Stop;
                }
                gdk::Key::p => {
                    find_overlay_clone.borrow().search_previous();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }

        if let Some(binding) = cfg_clone.borrow().get_binding_by_keyval(keyval, &modifier) {
            match binding.action.as_str() {
                "hint" => {
                    hints_clone.borrow_mut().activate(&browser);
                    return glib::Propagation::Stop;
                }
                "command" => {
                    if cmd_overlay_clone.borrow().is_some() {
                        return glib::Propagation::Proceed;
                    }

                    let full_overlay = GtkBox::new(Orientation::Vertical, 0);
                    full_overlay.add_css_class("command-overlay");
                    full_overlay.add_css_class("background");
                    full_overlay.set_halign(Align::Fill);
                    full_overlay.set_valign(Align::Fill);

                    let entry = Entry::new();
                    entry.set_placeholder_text(Some("Type a command..."));
                    entry.set_margin_top(16);
                    entry.set_margin_start(80);
                    entry.set_margin_end(80);
                    full_overlay.append(&entry);

                    // ---- Three-column layout ----
                    let columns = GtkBox::new(Orientation::Horizontal, 12);
                    columns.set_homogeneous(true);
                    columns.set_margin_start(80);
                    columns.set_margin_end(80);
                    columns.set_margin_top(8);
                    columns.set_vexpand(true);

                    // Left: Commands
                    let left_col = GtkBox::new(Orientation::Vertical, 4);
                    left_col.add_css_class("command-col");
                    left_col.set_size_request(280, -1);
                    let cmd_title_lbl = Label::new(Some("Commands"));
                    cmd_title_lbl.add_css_class("command-col-title");
                    cmd_title_lbl.set_halign(Align::Start);
                    left_col.append(&cmd_title_lbl);
                    let cmd_list_widget = ListBox::new();
                    cmd_list_widget.set_selection_mode(SelectionMode::None);
                    left_col.append(&cmd_list_widget);
                    let left_scroll = ScrolledWindow::builder().vexpand(true).child(&left_col).build();
                    columns.append(&left_scroll);

                    // Center: History
                    let center_col = GtkBox::new(Orientation::Vertical, 4);
                    center_col.add_css_class("command-col");
                    center_col.set_size_request(280, -1);
                    let hist_title_lbl = Label::new(Some("History"));
                    hist_title_lbl.add_css_class("command-col-title");
                    hist_title_lbl.set_halign(Align::Start);
                    center_col.append(&hist_title_lbl);
                    let hist_list_widget = ListBox::new();
                    hist_list_widget.set_selection_mode(SelectionMode::None);
                    center_col.append(&hist_list_widget);
                    let center_scroll = ScrolledWindow::builder().vexpand(true).child(&center_col).build();
                    columns.append(&center_scroll);

                    // Right: Keybindings
                    let right_col = GtkBox::new(Orientation::Vertical, 4);
                    right_col.add_css_class("command-col");
                    right_col.set_size_request(280, -1);
                    let kb_title_lbl = Label::new(Some("Keybindings"));
                    kb_title_lbl.add_css_class("command-col-title");
                    kb_title_lbl.set_halign(Align::Start);
                    right_col.append(&kb_title_lbl);
                    let kb_list_widget = ListBox::new();
                    kb_list_widget.set_selection_mode(SelectionMode::None);
                    for b in &cfg_clone.borrow().normal.bindings {
                        let row = ListBoxRow::new();
                        let h = GtkBox::new(Orientation::Horizontal, 8);
                        h.set_margin_top(3);
                        h.set_margin_bottom(3);
                        h.set_margin_start(8);
                        h.set_margin_end(8);
                        let mod_str = if b.modifier.is_empty() {
                            "—".to_string()
                        } else {
                            b.modifier.join(" ").to_uppercase()
                        };
                        let mod_lbl = Label::new(Some(&mod_str));
                        mod_lbl.add_css_class("monospace");
                        mod_lbl.add_css_class("command-row-small");
                        mod_lbl.set_width_chars(10);
                        let key_lbl = Label::new(Some(&b.key));
                        key_lbl.add_css_class("monospace");
                        key_lbl.add_css_class("command-row-small");
                        key_lbl.set_width_chars(8);
                        let act_lbl = Label::new(Some(&b.action));
                        act_lbl.add_css_class("command-help");
                        act_lbl.set_hexpand(true);
                        act_lbl.set_halign(Align::Start);
                        h.append(&mod_lbl);
                        h.append(&key_lbl);
                        h.append(&act_lbl);
                        row.set_child(Some(&h));
                        kb_list_widget.append(&row);
                    }
                    right_col.append(&kb_list_widget);
                    let right_scroll = ScrolledWindow::builder().vexpand(true).child(&right_col).build();
                    columns.append(&right_scroll);

                    full_overlay.append(&columns);

                    let esc_hint = Label::new(Some("↑/↓ navigate · Tab commit · Enter execute · Esc close"));
                    esc_hint.add_css_class("caption");
                    esc_hint.add_css_class("command-help");
                    esc_hint.set_margin_bottom(12);
                    full_overlay.append(&esc_hint);

                    // ---- State ----
                    let state = Rc::new(RefCell::new(OverlayState {
                        selected_cmd: -1,
                        selected_hist: -1,
                        active: OverlaySection::Command,
                        cmd_navigated: false,
                        hist_navigated: false,
                    }));

                    let all_cmd_names: Vec<&str> = ALL_COMMANDS.iter().map(|(n, _)| *n).collect();

                    // ---- Populate initial lists ----
                    rebuild_cmd_list(&cmd_list_widget, &all_cmd_names, -1);
                    let recent = history_mgr_clone.borrow().recent(20);
                    rebuild_hist_list(&hist_list_widget, &recent, -1);

                    // ---- Clones for closures ----
                    let wv_for_cmd = wv_weak.clone();
                    let cmd_overlay_c = cmd_overlay_clone.clone();
                    let cfg_cmd = cfg_clone.clone();
                    let app_for_cmd = app_clone.clone();
                    let find_overlay_cmd = find_overlay_clone.clone();
                    let overlay_cmd = overlay_clone.clone();
                    let download_mgr_cmd = download_mgr_clone.clone();
                    let session_mgr_cmd = session_mgr_clone.clone();
                    let history_mgr_cmd = history_mgr_clone.clone();
                    let tm_cmd = tm.clone();
                    let noctalia_provider_cmd = noctalia_provider.clone();
                    let history_mgr_changed = history_mgr_clone.clone();
                    let entry_state = state.clone();
                    let entry_cmd_list = cmd_list_widget.clone();
                    let entry_hist_list = hist_list_widget.clone();

                    // ---- Text change handler ----
                    entry.connect_changed(move |e| {
                        let text = e.text().to_string();
                        let cursor = e.position();
                        let space_pos = text.find(' ');
                        let (cmd_part, arg_part) = match space_pos {
                            Some(pos) => (&text[..pos], &text[pos + 1..]),
                            None => (&text[..], ""),
                        };

                        let in_command = space_pos.map_or(true, |pos| cursor as usize <= pos);
                        let mut st = entry_state.borrow_mut();
                        st.selected_cmd = -1;
                        st.selected_hist = -1;
                        st.cmd_navigated = false;
                        st.hist_navigated = false;

                        if in_command {
                            st.active = OverlaySection::Command;
                            let filtered = fuzzy::filter(&all_cmd_names.iter().copied().collect::<Vec<_>>(), cmd_part, 50);
                            let filtered_refs: Vec<&str> = filtered.into_iter().collect();
                            rebuild_cmd_list(&entry_cmd_list, &filtered_refs, -1);
                            let recent = history_mgr_changed.borrow().recent(20);
                            rebuild_hist_list(&entry_hist_list, &recent, -1);
                        } else if command::is_url_command(cmd_part) {
                            st.active = OverlaySection::History;
                            rebuild_cmd_list(&entry_cmd_list, &[cmd_part], -1);
                            let filtered = history_mgr_changed.borrow().fuzzy(arg_part, 50);
                            rebuild_hist_list(&entry_hist_list, &filtered, -1);
                        } else {
                            st.active = OverlaySection::Command;
                            let filtered = fuzzy::filter(
                                &all_cmd_names.iter().copied().collect::<Vec<_>>(),
                                &text,
                                50,
                            );
                            let filtered_refs: Vec<&str> = filtered.into_iter().collect();
                            rebuild_cmd_list(&entry_cmd_list, &filtered_refs, -1);
                            let recent = history_mgr_changed.borrow().recent(20);
                            rebuild_hist_list(&entry_hist_list, &recent, -1);
                        }
                    });

                    // ---- Enter execution ----
                    entry.connect_activate(move |e| {
                        let text = e.text().to_string();
                        let input = CommandInput::new(&text);
                        if let Some(cmd) = input.parse() {
                            if let Some(w) = wv_for_cmd.upgrade() {
                                match cmd {
                                    command::Command::Open(url) => w.load_uri(&url),
                                    command::Command::Back => { if w.can_go_back() { w.go_back(); } }
                                    command::Command::Forward => { if w.can_go_forward() { w.go_forward(); } }
                                    command::Command::Reload => w.reload(),
                                    command::Command::Duplicate => {
                                        let url = w.uri().map(|u| u.to_string()).unwrap_or_else(|| "https://www.rust-lang.org".to_string());
                                        let _ = build_window(&app_for_cmd, cfg_cmd.clone(), session_mgr_cmd.clone(), history_mgr_cmd.clone(), Some(&url));
                                    }
                                    command::Command::CopyAddress => {
                                        let url = w.uri().map(|u| u.to_string()).unwrap_or_default();
                                        if !url.is_empty() {
                                            if let Some(d) = gdk::Display::default() {
                                                d.clipboard().set_text(&url);
                                            }
                                        }
                                    }
                                    command::Command::Settings => {
                                        let settings_box = settings::show_settings_overlay(&overlay_cmd, cfg_cmd.clone());
                                        settings_box.grab_focus();
                                        let settings_key_ctl = EventControllerKey::new();
                                        let settings_box_esc = settings_box.clone();
                                        settings_key_ctl.connect_key_pressed(move |_, k, _, _| {
                                            if k == gdk::Key::Escape {
                                                settings_box_esc.unparent();
                                                return glib::Propagation::Stop;
                                            }
                                            glib::Propagation::Proceed
                                        });
                                        settings_box.add_controller(settings_key_ctl);
                                    }
                                    command::Command::NewWindowOpen(url) => {
                                        let _ = build_window(&app_for_cmd, cfg_cmd.clone(), session_mgr_cmd.clone(), history_mgr_cmd.clone(), Some(&url));
                                    }
                                    command::Command::SetDefaultBrowser => {
                                        // Step 1: ensure the .desktop file is installed locally
                                        match ensure_local_desktop_file() {
                                            Ok(_path) => {
                                                // Step 2: tell xdg-settings to use it
                                                let status = std::process::Command::new("xdg-settings")
                                                    .args(["set", "default-url-scheme-handler", "https", "org.blueak.iron.desktop"])
                                                    .status();
                                                match status {
                                                    Ok(s) if s.success() => eprintln!("Iron is now the default browser"),
                                                    Ok(s) => eprintln!("Failed to set default browser: {:?}", s.code()),
                                                    Err(_) => eprintln!("Could not run xdg-settings"),
                                                }
                                                // Also set for http
                                                let _ = std::process::Command::new("xdg-settings")
                                                    .args(["set", "default-url-scheme-handler", "http", "org.blueak.iron.desktop"])
                                                    .status();
                                            }
                                            Err(e) => eprintln!("Could not install .desktop file: {}", e),
                                        }
                                    }
                                    command::Command::CacStatus => eprintln!("{}", cac::status_text()),
                                    command::Command::SearchAdd(name, template) => {
                                        let mut cfg_mut = cfg_cmd.borrow_mut();
                                        cfg_mut.search.insert(search::SearchEngine { name, template });
                                        let _ = cfg_mut.save();
                                    }
                                    command::Command::SearchDel(name) => {
                                        let mut cfg_mut = cfg_cmd.borrow_mut();
                                        cfg_mut.search.remove(&name);
                                        let _ = cfg_mut.save();
                                    }
                                    command::Command::Search(query) => {
                                        if let Some(e) = cfg_cmd.borrow().search.default_engine().cloned() {
                                            if let Some(w) = wv_for_cmd.upgrade() {
                                                w.load_uri(&e.build_url(&query));
                                            }
                                        }
                                    }
                                    command::Command::Find(query) => {
                                        if let Some(w) = wv_for_cmd.upgrade() {
                                            find_overlay_cmd.borrow_mut().activate(&overlay_cmd, &w);
                                            if let Some(ent) = &find_overlay_cmd.borrow().entry {
                                                ent.set_text(&query);
                                                ent.set_position(-1);
                                                ent.grab_focus();
                                            }
                                        }
                                    }
                                    command::Command::Downloads => {
                                        let mgr = download_mgr_cmd.borrow();
                                        let recent = mgr.recent(10);
                                        if recent.is_empty() {
                                            eprintln!("No downloads yet");
                                        } else {
                                            for item in recent {
                                                let status = if item.done { "done" } else if item.failed { "failed" } else { "in progress" };
                                                eprintln!("{} [{}] - {}", item.filename, status, item.path);
                                            }
                                        }
                                    }
                                    command::Command::ClearSiteData => {
                                        if let Some(w) = wv_for_cmd.upgrade() {
                                            session_mgr_cmd.borrow().clear_all_site_data(&w);
                                        }
                                    }
                                    command::Command::ClearCookies => {
                                        if let Some(w) = wv_for_cmd.upgrade() {
                                            session_mgr_cmd.borrow().clear_cookies(&w);
                                        }
                                    }
                                    command::Command::History => {
                                        let hist_box = show_history_overlay(&overlay_cmd, history_mgr_cmd.clone());
                                        let hist_key_ctl = EventControllerKey::new();
                                        let hist_box_esc = hist_box.clone();
                                        hist_key_ctl.connect_key_pressed(move |_, k, _, _| {
                                            if k == gdk::Key::Escape {
                                                hist_box_esc.unparent();
                                                return glib::Propagation::Stop;
                                            }
                                            glib::Propagation::Proceed
                                        });
                                        hist_box.add_controller(hist_key_ctl);
                                    }
                                    command::Command::ClearHistory => {
                                        history_mgr_cmd.borrow_mut().clear();
                                        eprintln!("History cleared");
                                    }
                                    command::Command::DeleteHistory(url) => {
                                        history_mgr_cmd.borrow_mut().delete(&url);
                                        eprintln!("Deleted {} from history", url);
                                    }
                                    command::Command::ReloadTheme => {
                                        tm_cmd.borrow_mut().load();
                                        tm_cmd.borrow().apply_gtk_css(&noctalia_provider_cmd);
                                        if let Some(w) = wv_for_cmd.upgrade() {
                                            tm_cmd.borrow().apply_webkit_css(&w);
                                        }
                                        eprintln!("Theme reloaded manually");
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

                    // ---- Key navigation ----
                    let key_state = state.clone();
                    let key_cmd_list = cmd_list_widget.clone();
                    let key_hist_list = hist_list_widget.clone();
                    let key_entry = entry.clone();

                    let entry_key_ctl = EventControllerKey::new();
                    entry.add_controller(entry_key_ctl.clone());
                    let cmd_overlay_esc = cmd_overlay_clone.clone();
                    let wv_weak_esc = wv_weak.clone();

                    entry_key_ctl.connect_key_pressed(move |_, k, _, _| {
                        match k {
                            gdk::Key::Escape => {
                                if let Some(bar) = cmd_overlay_esc.borrow_mut().take() {
                                    bar.unparent();
                                }
                                if let Some(w) = wv_weak_esc.upgrade() {
                                    w.grab_focus();
                                }
                                return glib::Propagation::Stop;
                            }
                            gdk::Key::Up => {
                                let mut st = key_state.borrow_mut();
                                match st.active {
                                    OverlaySection::Command => {
                                        let count = listbox_row_count(&key_cmd_list);
                                        if count > 0 {
                                            st.selected_cmd = if st.selected_cmd <= 0 { count - 1 } else { st.selected_cmd - 1 };
                                            st.cmd_navigated = true;
                                        }
                                    }
                                    OverlaySection::History => {
                                        let count = listbox_row_count(&key_hist_list);
                                        if count > 0 {
                                            st.selected_hist = if st.selected_hist <= 0 { count - 1 } else { st.selected_hist - 1 };
                                            st.hist_navigated = true;
                                        }
                                    }
                                }
                                drop(st);
                                update_highlight(&key_state, &key_cmd_list, &key_hist_list);
                                return glib::Propagation::Stop;
                            }
                            gdk::Key::Down => {
                                let mut st = key_state.borrow_mut();
                                match st.active {
                                    OverlaySection::Command => {
                                        let count = listbox_row_count(&key_cmd_list);
                                        if count > 0 {
                                            st.selected_cmd = if st.selected_cmd >= count - 1 { 0 } else { st.selected_cmd + 1 };
                                            st.cmd_navigated = true;
                                        }
                                    }
                                    OverlaySection::History => {
                                        let count = listbox_row_count(&key_hist_list);
                                        if count > 0 {
                                            st.selected_hist = if st.selected_hist >= count - 1 { 0 } else { st.selected_hist + 1 };
                                            st.hist_navigated = true;
                                        }
                                    }
                                }
                                drop(st);
                                update_highlight(&key_state, &key_cmd_list, &key_hist_list);
                                return glib::Propagation::Stop;
                            }
                            gdk::Key::Tab => {
                                let st = key_state.borrow();
                                let action = match st.active {
                                    OverlaySection::Command if st.cmd_navigated && st.selected_cmd >= 0 => {
                                        if let Some(name) = cmd_name_at_index(&key_cmd_list, st.selected_cmd) {
                                            let new_text = if command::is_url_command(&name) {
                                                format!("{} ", name)
                                            } else {
                                                name.clone()
                                            };
                                            Some(new_text)
                                        } else { None }
                                    }
                                    OverlaySection::History if st.hist_navigated && st.selected_hist >= 0 => {
                                        if let Some(url) = hist_url_at_index(&key_hist_list, st.selected_hist) {
                                            let text = key_entry.text().to_string();
                                            if let Some(pos) = text.find(' ') {
                                                let cmd = &text[..pos];
                                                Some(format!("{} {}", cmd, url))
                                            } else { None }
                                        } else { None }
                                    }
                                    _ => None,
                                };
                                drop(st);
                                if let Some(new_text) = action {
                                    key_entry.set_text(&new_text);
                                    key_entry.set_position(-1);
                                }
                                return glib::Propagation::Stop;
                            }
                            gdk::Key::space => {
                                let st = key_state.borrow();
                                let action = match st.active {
                                    OverlaySection::Command if st.cmd_navigated && st.selected_cmd >= 0 => {
                                        if let Some(name) = cmd_name_at_index(&key_cmd_list, st.selected_cmd) {
                                            let new_text = if command::is_url_command(&name) {
                                                format!("{} ", name)
                                            } else {
                                                name.clone()
                                            };
                                            Some(new_text)
                                        } else { None }
                                    }
                                    OverlaySection::History if st.hist_navigated && st.selected_hist >= 0 => {
                                        if let Some(url) = hist_url_at_index(&key_hist_list, st.selected_hist) {
                                            let text = key_entry.text().to_string();
                                            if let Some(pos) = text.find(' ') {
                                                let cmd = &text[..pos];
                                                Some(format!("{} {}", cmd, url))
                                            } else { None }
                                        } else { None }
                                    }
                                    _ => None,
                                };
                                drop(st);
                                if let Some(new_text) = action {
                                    key_entry.set_text(&new_text);
                                    key_entry.set_position(-1);
                                    return glib::Propagation::Stop;
                                }
                                glib::Propagation::Proceed
                            }
                            _ => glib::Propagation::Proceed,
                        }
                    });

                    *cmd_overlay_clone.borrow_mut() = Some(full_overlay.clone());
                    overlay.add_overlay(&full_overlay);
                    entry.grab_focus();

                    return glib::Propagation::Stop;
                }
                "find" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        find_overlay_clone.borrow_mut().activate(&overlay_clone, &wv);
                    }
                    return glib::Propagation::Stop;
                }
                "reload" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        wv.reload();
                    }
                    return glib::Propagation::Stop;
                }
                "duplicate" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        let url = wv.uri().map(|u| u.to_string()).unwrap_or_else(|| "https://www.rust-lang.org".to_string());
                        let _ = build_window(&app_clone, cfg_clone.clone(), session_mgr_clone.clone(), history_mgr_clone.clone(), Some(&url));
                    }
                    return glib::Propagation::Stop;
                }
                "back" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        if wv.can_go_back() { wv.go_back(); }
                    }
                    return glib::Propagation::Stop;
                }
                "forward" => {
                    if let Some(wv) = wv_weak.upgrade() {
                        if wv.can_go_forward() { wv.go_forward(); }
                    }
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }

        glib::Propagation::Proceed
    });
    window.add_controller(key_ctl);

    window.present();
    ThemeManager::start_watch(tm_watch, &webview, &noctalia_provider_watch);
    window
}

// ---- Helper functions for command overlay ----

fn rebuild_cmd_list(list: &ListBox, items: &[&str], selected: i32) {
    while let Some(c) = list.first_child() {
        list.remove(&c);
    }
    for (idx, item) in items.iter().enumerate() {
        let row = ListBoxRow::new();
        let h = GtkBox::new(Orientation::Horizontal, 6);
        h.set_margin_top(3);
        h.set_margin_bottom(3);
        h.set_margin_start(8);
        h.set_margin_end(8);
        let lbl = Label::new(Some(item));
        lbl.add_css_class("command-row-small");
        lbl.set_halign(Align::Start);
        h.append(&lbl);
        row.set_child(Some(&h));
        if idx as i32 == selected {
            row.add_css_class("command-selected");
        }
        list.append(&row);
    }
}

fn rebuild_hist_list(list: &ListBox, items: &[history::HistoryItem], selected: i32) {
    while let Some(c) = list.first_child() {
        list.remove(&c);
    }
    for (idx, item) in items.iter().enumerate() {
        let row = ListBoxRow::new();
        let v = GtkBox::new(Orientation::Vertical, 2);
        v.set_margin_top(3);
        v.set_margin_bottom(3);
        v.set_margin_start(8);
        v.set_margin_end(8);
        let title = if item.title.is_empty() { &item.url } else { &item.title };
        let title_lbl = Label::new(Some(title));
        title_lbl.add_css_class("command-row-small");
        title_lbl.set_halign(Align::Start);
        v.append(&title_lbl);
        if !item.title.is_empty() && item.title != item.url {
            let url_lbl = Label::new(Some(&item.url));
            url_lbl.add_css_class("command-help");
            url_lbl.set_halign(Align::Start);
            v.append(&url_lbl);
        }
        row.set_child(Some(&v));
        if idx as i32 == selected {
            row.add_css_class("command-selected");
        }
        list.append(&row);
    }
}

fn listbox_row_count(list: &ListBox) -> i32 {
    let mut count = 0;
    let mut child = list.first_child();
    while let Some(c) = child {
        count += 1;
        child = c.next_sibling();
    }
    count
}

fn update_highlight(state: &Rc<RefCell<OverlayState>>, cmd_list: &ListBox, hist_list: &ListBox) {
    for i in 0..listbox_row_count(cmd_list) {
        if let Some(row) = cmd_list.row_at_index(i) {
            row.remove_css_class("command-selected");
        }
    }
    for i in 0..listbox_row_count(hist_list) {
        if let Some(row) = hist_list.row_at_index(i) {
            row.remove_css_class("command-selected");
        }
    }
    let st = state.borrow();
    match st.active {
        OverlaySection::Command => {
            if let Some(row) = cmd_list.row_at_index(st.selected_cmd) {
                row.add_css_class("command-selected");
            }
        }
        OverlaySection::History => {
            if let Some(row) = hist_list.row_at_index(st.selected_hist) {
                row.add_css_class("command-selected");
            }
        }
    }
}

fn cmd_name_at_index(list: &ListBox, idx: i32) -> Option<String> {
    let row = list.row_at_index(idx)?;
    let child = row.child()?;
    let hbox = child.downcast_ref::<GtkBox>()?;
    let first_widget = hbox.first_child()?;
    let lbl = first_widget.downcast_ref::<Label>()?;
    Some(lbl.text().to_string())
}

fn hist_url_at_index(list: &ListBox, idx: i32) -> Option<String> {
    let row = list.row_at_index(idx)?;
    let child = row.child()?;
    let vbox = child.downcast_ref::<GtkBox>()?;
    let first_widget = vbox.first_child()?;
    let first_lbl = first_widget.downcast_ref::<Label>()?;
    if let Some(second_widget) = first_lbl.next_sibling() {
        if let Some(second_lbl) = second_widget.downcast_ref::<Label>() {
            return Some(second_lbl.text().to_string());
        }
    }
    Some(first_lbl.text().to_string())
}

fn show_history_overlay(overlay: &Overlay, history_mgr: Rc<RefCell<HistoryManager>>) -> GtkBox {
    let full = GtkBox::new(Orientation::Vertical, 0);
    full.add_css_class("command-overlay");
    full.set_halign(Align::Fill);
    full.set_valign(Align::Fill);

    let title = Label::new(Some("History"));
    title.add_css_class("title-1");
    title.set_margin_top(24);
    title.set_margin_start(80);
    title.set_margin_end(80);
    title.set_halign(Align::Start);
    full.append(&title);

    let scroll = ScrolledWindow::builder().vexpand(true).build();
    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::None);

    let items = history_mgr.borrow().all();
    for item in items {
        let row = ListBoxRow::new();
        let v = GtkBox::new(Orientation::Vertical, 2);
        v.set_margin_top(4);
        v.set_margin_bottom(4);
        v.set_margin_start(12);
        v.set_margin_end(12);
        let disp = if item.title.is_empty() { &item.url } else { &item.title };
        let title_lbl = Label::new(Some(disp));
        title_lbl.add_css_class("command-row-small");
        title_lbl.set_halign(Align::Start);
        v.append(&title_lbl);
        if !item.title.is_empty() && item.title != item.url {
            let url_lbl = Label::new(Some(&item.url));
            url_lbl.add_css_class("command-help");
            url_lbl.set_halign(Align::Start);
            v.append(&url_lbl);
        }
        row.set_child(Some(&v));
        list.append(&row);
    }

    scroll.set_child(Some(&list));
    full.append(&scroll);

    let esc_hint = Label::new(Some("Press Escape to close"));
    esc_hint.add_css_class("caption");
    esc_hint.add_css_class("command-help");
    esc_hint.set_margin_bottom(12);
    full.append(&esc_hint);

    overlay.add_overlay(&full);
    full
}

/// Ensure the local .desktop file exists in ~/.local/share/applications/.
/// This is required for xdg-settings to recognise it as a valid handler.
fn ensure_local_desktop_file() -> Result<std::path::PathBuf, std::io::Error> {
    let local_apps = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".to_string())))
        .join("applications");

    std::fs::create_dir_all(&local_apps)?;

    let dest = local_apps.join("org.blueak.iron.desktop");
    if dest.exists() {
        return Ok(dest);
    }

    // Try to copy from the install location first (standard FHS paths).
    let candidates = [
        std::path::PathBuf::from("/usr/share/applications/org.blueak.iron.desktop"),
        std::path::PathBuf::from("/usr/local/share/applications/org.blueak.iron.desktop"),
    ];
    for src in candidates {
        if src.exists() {
            std::fs::copy(&src, &dest)?;
            return Ok(dest);
        }
    }

    // Fallback: write the desktop entry inline so the binary is self-contained.
    // Use the absolute path of the running binary so xdg-open can find it
    // regardless of whether `iron` is on $PATH (important on Silverblue/atomic).
    let exec_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "iron".to_string());
    let desktop_content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Iron\n\
         Comment=GTK4 keyboard-driven web browser for BlueAK\n\
         Exec={exec_path} %u\n\
         Icon=org.blueak.iron\n\
         Categories=Network;WebBrowser;\n\
         MimeType=text/html;text/xml;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;\n\
         StartupNotify=true\n\
         Terminal=false\n\
         NoDisplay=false\n"
    );
    std::fs::write(&dest, &desktop_content)?;
    Ok(dest)
}

// Note: CEF configuration is handled in cef_init.rs
// CEF uses Chrome UA by default, JavaScript/WebGL/media enabled by default
