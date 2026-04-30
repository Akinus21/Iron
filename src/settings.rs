use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Window};

use crate::config::{Config, KeyBinding, Mode};

const PROTECTED_ACTIONS: [&str; 2] = ["hint", "command"];

pub fn show_settings_window(parent: &gtk4::ApplicationWindow, config: Rc<RefCell<Config>>) {
    let win = Window::builder()
        .title("Iron Settings")
        .modal(true)
        .transient_for(parent)
        .default_width(600)
        .default_height(400)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    let title = Label::new(Some("Key Bindings"));
    title.add_css_class("title-3");
    vbox.append(&title);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");

    let config_clone = config.clone();
    rebuild_binding_rows(&list, &config_clone.borrow().normal.bindings, config.clone(), &win);

    let scroll = ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .build();
    vbox.append(&scroll);

    // Add-new row
    let add_box = GtkBox::new(Orientation::Horizontal, 6);
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
    vbox.append(&add_box);

    let list_weak = list.downgrade();
    let config_weak = config.clone();
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
                &win,
            );
        }

        key_entry.set_text("");
        mod_entry.set_text("");
        act_entry.set_text("");
    });

    win.set_child(Some(&vbox));
    win.present();
}

fn rebuild_binding_rows(
    list: &ListBox,
    bindings: &[KeyBinding],
    config: Rc<RefCell<Config>>,
    win: &Window,
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
            let win_ref = win.clone();
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
                        &win_ref,
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
