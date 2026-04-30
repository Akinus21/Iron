use webkit6::{glib::Error as JsError, javascriptcore::Value as JsValue, WebView};
use webkit6::prelude::WebViewExt;

/// JavaScript module for hint overlays — injected on activate, self-contained.
const HINT_JS_MODULE: &str = r#"
(function() {
  const CHARS = "asdfjklghqwer";
  let _hints = [];

  function labels(count) {
    const out = [];
    for (const c of CHARS) { out.push(c); if (out.length >= count) return out; }
    for (const c1 of CHARS) {
      for (const c2 of CHARS) {
        out.push(c1 + c2);
        if (out.length >= count) return out;
      }
    }
    return out;
  }

  window.__iron_hints_activate = function() {
    window.__iron_hints_deactivate();
    const els = document.querySelectorAll(
      'a[href], button, input[type="submit"], [role="button"], [onclick], summary, [tabindex]',
    );
    const lbls = labels(els.length);
    _hints = [];
    let i = 0;
    for (const el of els) {
      if (i >= lbls.length) break;
      if (el.offsetParent === null) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      if (rect.top < 0 || rect.left < 0 || rect.top > window.innerHeight || rect.left > window.innerWidth) continue;
      const label = lbls[i++];
      const div = document.createElement('div');
      div.className = '__iron_hint';
      div.textContent = label;
      div.style.cssText = [
        'position:fixed',
        'z-index:2147483647',
        'background:#ffdd57',
        'color:#111',
        'font:bold 11px monospace',
        'padding:1px 4px',
        'border-radius:3px',
        'pointer-events:none',
        'box-shadow:0 1px 3px rgba(0,0,0,.4)',
        'left:' + Math.max(0, rect.left) + 'px',
        'top:' + Math.max(0, rect.top) + 'px',
      ].join(';');
      document.body.appendChild(div);
      _hints.push({el, label, div});
    }
  };

  window.__iron_hints_filter = function(prefix) {
    let visible = 0;
    let last = null;
    for (const h of _hints) {
      if (h.label.startsWith(prefix)) {
        h.div.style.display = '';
        visible++;
        last = h;
      } else {
        h.div.style.display = 'none';
      }
    }
    if (visible === 1 && last) {
      last.el.click();
      window.__iron_hints_deactivate();
    }
  };

  window.__iron_hints_deactivate = function() {
    for (const h of _hints) h.div.remove();
    _hints = [];
  };
})();
"#;

pub struct HintManager {
    pub active: bool,
    typed: String,
}

impl HintManager {
    pub fn new() -> Self {
        HintManager {
            active: false,
            typed: String::with_capacity(4),
        }
    }

    /// Inject hints into the page and start capture.
    pub fn activate(&mut self, webview: &WebView) {
        if self.active {
            self.deactivate(webview);
        }
        self.active = true;
        self.typed.clear();
        webview.evaluate_javascript(
            HINT_JS_MODULE,
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
        webview.evaluate_javascript(
            "__iron_hints_activate();",
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Append `c` to the typed prefix and filter visible hints.
    /// Auto-clicks + deactivates if only one hint matches.
    pub fn handle_key(&mut self, c: char, webview: &WebView) {
        self.typed.push(c);
        webview.evaluate_javascript(
            &format!("__iron_hints_filter('{}');", self.typed),
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Pop the last typed character and re-filter.
    pub fn handle_backspace(&mut self, webview: &WebView) {
        if self.typed.pop().is_some() {
            let js = if self.typed.is_empty() {
                "__iron_hints_filter('');".to_string()
            } else {
                format!("__iron_hints_filter('{}');", self.typed)
            };
            webview.evaluate_javascript(
                &js,
                None::<&gio::Cancellable>,
                None::<&gio::Cancellable>,
                None::<&gio::Cancellable>,
                |_: Result<JsValue, JsError>| {},
            );
        }
    }

    /// Remove all hint overlays and exit hint mode.
    pub fn deactivate(&mut self, webview: &WebView) {
        self.active = false;
        self.typed.clear();
        webview.evaluate_javascript(
            "__iron_hints_deactivate();",
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            None::<&gio::Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }
}
