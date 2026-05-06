use gtk4::{Align, Box as GtkBox, Button, Entry, Label, Orientation};
use gtk4::prelude::{WidgetExt, EditableExt, EntryExt, BoxExt, ButtonExt};
use crate::cef_browser::CefBrowserWrapper;

const MAX_MATCH_COUNT: u32 = 1000;

pub struct FindOverlay {
    pub active: bool,
    pub entry: Option<gtk4::Entry>,
    container: Option<GtkBox>,
    match_label: Option<Label>,
    current_search: Option<String>,
}

impl FindOverlay {
    pub fn new() -> Self {
        FindOverlay {
            active: false,
            entry: None,
            container: None,
            match_label: None,
            current_search: None,
        }
    }

    pub fn activate(
        &mut self,
        overlay: &gtk4::Overlay,
        browser: &CefBrowserWrapper,
    ) {
        if self.active {
            if let Some(entry) = &self.entry {
                entry.grab_focus();
            }
            return;
        }

        let container = GtkBox::new(Orientation::Horizontal, 8);
        container.add_css_class("toolbar");
        container.set_margin_top(8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_halign(Align::Start);
        container.set_valign(Align::Start);

        let entry = Entry::new();
        entry.set_placeholder_text(Some("Find in page..."));
        entry.set_width_chars(24);
        entry.add_css_class("monospace");
        container.append(&entry);

        let match_label = Label::new(Some("0 matches"));
        match_label.add_css_class("caption");
        match_label.add_css_class("monospace");
        container.append(&match_label);

        let prev_btn = Button::with_label("◀");
        prev_btn.add_css_class("flat");
        container.append(&prev_btn);

        let next_btn = Button::with_label("▶");
        next_btn.add_css_class("flat");
        container.append(&next_btn);

        let close_btn = Button::with_label("✕");
        close_btn.add_css_class("flat");
        container.append(&close_btn);

        let browser_search = browser.clone();
        let match_lbl = match_label.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            if text.is_empty() {
                browser_search.stop_finding();
                match_lbl.set_text("0 matches");
            } else {
                browser_search.find(&text, true, false);
                match_lbl.set_text("Finding...");
            }
        });

        let browser_enter = browser.clone();
        entry.connect_activate(move |_| {
            browser_enter.search_next();
        });

        let browser_next = browser.clone();
        next_btn.connect_clicked(move |_| {
            browser_next.find("", true, false); // next
        });

        let browser_prev = browser.clone();
        prev_btn.connect_clicked(move |_| {
            browser_prev.find("", false, false); // previous
        });

        let container_close = container.clone();
        let browser_close = browser.clone();
        close_btn.connect_clicked(move |_| {
            browser_close.stop_finding();
            container_close.unparent();
        });

        overlay.add_overlay(&container);
        entry.grab_focus();

        self.container = Some(container);
        self.match_label = Some(match_label);
        self.entry = Some(entry);
        self.active = true;
    }

    pub fn deactivate(
        &mut self,
        _overlay: &gtk4::Overlay,
    ) {
        if let Some(container) = self.container.take() {
            container.unparent();
        }
        self.match_label = None;
        self.entry = None;
        self.current_search = None;
        self.active = false;
    }

    pub fn search_next(&self) {
        // TODO: Implement CEF search next
        if let Some(entry) = &self.entry {
            entry.grab_focus();
        }
    }

    pub fn search_previous(&self) {
        // Similar to search_next but backwards
        self.search_next();
    }
}
