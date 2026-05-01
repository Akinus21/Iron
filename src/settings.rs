use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};

use crate::config::{Config, KeyBinding};

const PROTECTED_ACTIONS: [&str; 2] = ["hint", "command"];

/// Build and attach a full-window settings overlay to the given `Overlay`.
/// Returns a `GtkBox` representing the overlay widget so the caller can
/// track it and remove it later.
pub fn show_settings_overlay(
    overlay: &gtk4::Overlay,
    config: Rc<RefCell<Config>>,
) -> GtkBox {
    let full = GtkBox::new(Orientation::Vertical, 0);
    full.add_css_class("command-overlay");
    full.add_css_class("background");
    full.set_halign(Align::Fill);
    full.set_valign(Align::Fill);

    // --- Title ---
    let title = Label::new(Some("Settings"));
    title.add_css_class("title-1");
    title.set_margin_top(24);
    title.set_margin_start(80);
    title.set_margin_end(80);
    title.set_halign(Align::Start);
    full.append(&title);

    // --- Scrollable content ---
    let scroll = ScrolledWindow::builder().vexpand(true).build();
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_start(80);
    content.set_margin_end(80);
    content.set_margin_bottom(24);

    // Section: Current keybindings
    let kb_title = Label::new(Some("Key Bindings"));
    kb_title.add_css_class("title-2");
    kb_title.set_halign(Align::Start);
    content.append(&kb_title);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    let config_list = config.clone();
    rebuild_binding_rows(&list, &config_list.borrow().normal.bindings, config.clone(), overlay);
    content.append(&list);

    // Add-new row
    let add_box = GtkBox::new(Orientation::Horizontal, 6);
    add_box.set_margin_top(8);

    let key_entry = Entry::new();
    key_entry.set_placeholder_text(Some("Key (e.g. f, colon, g)"));
    key_entry.set_hexpand(true);

    let mod_entry = Entry::new();
    mod_entry.set_placeholder_text(Some("Modifiers (e.g. shift ctrl)"));
    mod_entry.set_hexpand(true);

    let act_entry = Entry::new();
    act_entry.set_placeholder_text(Some("Action (e.g. hint, command)"));
    act_entry.set_hexpand(true);

    let add_btn = Button::with_label("Add Binding");
    add_btn.add_css_class("suggested-action");

    add_box.append(&key_entry);
    add_box.append(&mod_entry);
    add_box.append(&act_entry);
    add_box.append(&add_btn);
    content.append(&add_box);

    let list_weak = list.downgrade();
    let config_weak = config.clone();
    let overlay_weak = overlay.downgrade();
    add_btn.connect_clicked(move |_btn| {
        let key = key_entry.text().to_string().trim().to_lowercase();
        let mods: Vec<String> = mod_entry
            .text()
            .to_string()
            .split_whitespace()
            .map(|s| s.to_lowercase().to_string())
            .collect();
        let action = act_entry.text().to_string().trim().to_lowercase();

        if key.is_empty() || action.is_empty() {
            return;
        }

        {
            let mut cfg = config_weak.borrow_mut();
            cfg.normal.bindings.push(KeyBinding {
                key,
                modifier: mods,
                action,
            });
            let _ = cfg.save();
        }

        if let Some(list) = list_weak.upgrade() {
            rebuild_binding_rows(
                &list,
                &config_weak.borrow().normal.bindings,
                config_weak.clone(),
                &overlay_weak.upgrade().unwrap(),
            );
        }

        key_entry.set_text("");
        mod_entry.set_text("");
        act_entry.set_text("");
    });

    scroll.set_child(Some(&content));
    full.append(&scroll);

    // --- Bottom escape hint ---
    let esc_hint = Label::new(Some("Press Escape to close settings"));
    esc_hint.add_css_class("caption");
    esc_hint.add_css_class("command-help");
    esc_hint.set_margin_bottom(12);
    full.append(&esc_hint);

    overlay.add_overlay(&full);
    full
}

fn rebuild_binding_rows(
    list: &ListBox,
    bindings: &[KeyBinding],
    config: Rc<RefCell<Config>>,
    overlay: &gtk4::Overlay,
) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }

    for (idx, binding) in bindings.iter().enumerate() {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        let mod_str = if binding.modifier.is_empty() {
            "—".to_string()
        } else {
            binding.modifier.join(" ").to_uppercase()
        };

        let label = Label::new(Some(&format!(
            "{}  │  {}  →  {}",
            mod_str, binding.key, binding.action
        )));
        label.set_hexpand(true);
        label.set_halign(Align::Start);
        hbox.append(&label);

        let protected = PROTECTED_ACTIONS.contains(&binding.action.as_str());

        if !protected {
            let del_btn = Button::with_label("Remove");
            del_btn.add_css_class("destructive-action");
            let config_c = config.clone();
            let list_weak = list.downgrade();
            let overlay_ref = overlay.clone();
            del_btn.connect_clicked(move |_btn| {
                {
                    let mut cfg = config_c.borrow_mut();
                    if idx < cfg.normal.bindings.len() {
                        cfg.normal.bindings.remove(idx);
                        let _ = cfg.save();
                    }
                }
                if let Some(list) = list_weak.upgrade() {
                    rebuild_binding_rows(
                        &list,
                        &config_c.borrow().normal.bindings,
                        config_c.clone(),
                        &overlay_ref,
                    );
                }
            });
            hbox.append(&del_btn);
        } else {
            let tag = Label::new(Some("protected"));
            tag.add_css_class("caption");
            tag.set_opacity(0.5);
            hbox.append(&tag);
        }

        row.set_child(Some(&hbox));
        list.append(&row);
    }
}
