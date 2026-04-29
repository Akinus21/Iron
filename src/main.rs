mod noctalia;

use adw::prelude::*;
use webkit6::prelude::*;

fn main() {
    let app = adw::Application::new(Some("org.blueak.iron"), gio::ApplicationFlags::default());

    app.connect_activate(|app| {
        let tokens = noctalia::NoctaliaTokens::load();
        let css = tokens.to_css();

        let window = adw::ApplicationWindow::new(app);
        window.set_default_size(1024, 768);
        window.set_title(Some("Iron"));

        if !css.is_empty() {
            let provider = gtk4::CssProvider::new();
            provider.load_from_string(&css);
            gtk4::style_context_add_provider_for_display(
                &adw::prelude::WidgetExt::display(&window),
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let webview = webkit6::WebView::new();
        webview.load_uri("https://www.rust-lang.org");

        window.set_content(Some(&webview));
        window.present();
    });

    app.run();
}
