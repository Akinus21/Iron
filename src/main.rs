mod noctalia;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use webkit6::prelude::*;

use noctalia::ThemeManager;

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
        window.present();

        ThemeManager::start_watch(tm, &webview);
    });

    app.run();
}
