use gtk4::{Align, Box as GtkBox, Button, CssProvider, Entry, Label, Orientation, STYLE_PROVIDER_PRIORITY_APPLICATION};
use webkit6::prelude::*;

const MAX_MATCH_COUNT: u32 = 1000;

pub struct FindOverlay {
    pub active: bool,
    pub entry: Option<gtk4::Entry>,
    container: Option<GtkBox>,
    match_label: Option<Label>,
    controller: Option<webkit6::FindController>,
}

impl FindOverlay {
    pub fn new() -> Self {
        FindOverlay {
            active: false,
            entry: None,
            container: None,
            match_label: None,
            controller: None,
        }
    }

    pub fn activate(
        &mut self,
        overlay: &gtk4::Overlay,
        webview: &webkit6::WebView,
        css_provider: &CssProvider,
    ) {
        if self.active {
            if let Some(entry) = &self.entry {
                entry.grab_focus();
            }
            return;
        }

        let controller = webview.find_controller().expect("WebView has FindController");
        self.controller = Some(controller.clone());

        let container = GtkBox::new(Orientation::Horizontal, 8);
        container.add_css_class("toolbar");
        container.style_context().add_provider(css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);
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

        let controller_search = controller.clone();
        let match_lbl = match_label.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            if text.is_empty() {
                controller_search.search_finish();
                match_lbl.set_text("0 matches");
            } else {
                let opts = webkit6::FindOptions::CASE_INSENSITIVE.bits()
                    | webkit6::FindOptions::WRAP_AROUND.bits();
                controller_search.search(&text, opts, MAX_MATCH_COUNT);
            }
        });

        let controller_enter = controller.clone();
        entry.connect_activate(move |_| {
            controller_enter.search_next();
        });

        let controller_next = controller.clone();
        next_btn.connect_clicked(move |_| {
            controller_next.search_next();
        });

        let controller_prev = controller.clone();
        prev_btn.connect_clicked(move |_| {
            controller_prev.search_previous();
        });

        let match_lbl_found = match_label.clone();
        controller.connect_found_text(move |_, count| {
            match_lbl_found.set_text(&format!(
                "{} match{}",
                count,
                if count == 1 { "" } else { "es" }
            ));
        });

        let match_lbl_fail = match_label.clone();
        controller.connect_failed_to_find_text(move |_| {
            match_lbl_fail.set_text("No matches");
        });

        let container_close = container.clone();
        let controller_close = controller.clone();
        close_btn.connect_clicked(move |_| {
            controller_close.search_finish();
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
        if let Some(controller) = &self.controller {
            controller.search_finish();
        }
        if let Some(container) = self.container.take() {
            container.unparent();
        }
        self.match_label = None;
        self.entry = None;
        self.controller = None;
        self.active = false;
    }

    pub fn search_next(&self) {
        if let Some(controller) = &self.controller {
            controller.search_next();
        }
    }

    pub fn search_previous(&self) {
        if let Some(controller) = &self.controller {
            controller.search_previous();
        }
    }
}
