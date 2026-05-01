use gio::Cancellable;
use webkit6::{glib::Error as JsError, javascriptcore::Value as JsValue, WebView, prelude::WebViewExt};

/// JavaScript module for hint overlays — injected on activate, self-contained.
/// Supports:
/// - hint generation, filtering, and auto-click on single visible match
/// - selection tracking (arrow keys)   
/// - commit-on-Enter (click currently selected / exact matching / last visible)
const HINT_JS_MODULE: &str = r#"
(function() {
  const CHARS = "asdfjklghqwer";
  let _hints = [];
  let _selection = -1;

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
    // Install a focus-capture listener that blurs anything trying to steal focus
    window.__iron_hint_focus_trap = function(e) {
      if (e.target && e.target !== document.body) {
        e.target.blur();
        e.stopImmediatePropagation();
      }
    };
    document.addEventListener('focus', window.__iron_hint_focus_trap, true);
    if (document.activeElement) { document.activeElement.blur(); }
    const els = document.querySelectorAll(
      'a[href], button, input[type="submit"], [role="button"], [onclick], summary, [tabindex]',
    );
    const lbls = labels(els.length);
    _hints = [];
    _selection = -1;
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
      // Track each hint's raw label, element, and DOM node
      _hints.push({ el: el, label: label, div: div, visible: true });
      document.body.appendChild(div);
    }
    __iron_hints_update_selection();
  };

  window.__iron_hints_filter = function(prefix) {
    let visible = 0;
    let last = null;
    for (const h of _hints) {
      if (h.label.startsWith(prefix)) {
        h.div.style.display = '';
        h.visible = true;
        visible++;
        last = h;
      } else {
        h.div.style.display = 'none';
        h.visible = false;
      }
    }
    // Reset selection to first visible on any prefix change
    _selection = -1;
    __iron_hints_update_selection();
    if (visible === 1 && last) {
      last.el.click();
      window.__iron_hints_deactivate();
    }
  };

  window.__iron_hints_select_next = function() {
    const vis = _hints.filter(function(h) { return h.visible; });
    if (vis.length === 0) return;
    // find current selected index among visible
    let idx = -1;
    for (let i = 0; i < vis.length; i++) { if (vis[i].selected) { idx = i; break; } }
    const nextIdx = (idx + 1) % vis.length;
    // clear all
    for (const h of _hints) { h.selected = false; }
    vis[nextIdx].selected = true;
    __iron_hints_update_selection();
  };

  window.__iron_hints_select_prev = function() {
    const vis = _hints.filter(function(h) { return h.visible; });
    if (vis.length === 0) return;
    let idx = -1;
    for (let i = 0; i < vis.length; i++) { if (vis[i].selected) { idx = i; break; } }
    let prevIdx = idx - 1;
    if (prevIdx < 0) prevIdx = vis.length - 1;
    for (const h of _hints) { h.selected = false; }
    vis[prevIdx].selected = true;
    __iron_hints_update_selection();
  };

  window.__iron_hints_commit = function() {
    // 1. Exact label match (commit typed prefix as exact label)
    const exact_match = _hints.filter(function(h) { return h.visible && h.label === window.__iron_hints_typed; })[0];
    if (exact_match) { exact_match.el.click(); window.__iron_hints_deactivate(); return; }
    // 2. Currently selected hint
    const selected = _hints.filter(function(h) { return h.visible && h.selected; })[0];
    if (selected) { selected.el.click(); window.__iron_hints_deactivate(); return; }
    // 3. Last remaining visible hint
    const vis = _hints.filter(function(h) { return h.visible; });
    if (vis.length === 1) { vis[0].el.click(); window.__iron_hints_deactivate(); return; }
    // Nothing to commit — just clear the overlays so the user isn't trapped
    window.__iron_hints_deactivate();
  };

  window.__iron_hints_update_selection = function() {
    const vis = _hints.filter(function(h) { return h.visible; });
    for (let i = 0; i < vis.length; i++) {
      const h = vis[i];
      if (h.selected) {
        h.div.style.cssText = h.div.style.cssText.replace(/background:#ffdd57/, 'background:#e08030');
        h.div.style.cssText = h.div.style.cssText.replace(/color:#111/, 'color:#fff');
      } else {
        h.div.style.cssText = h.div.style.cssText.replace(/background:#e08030/, 'background:#ffdd57');
        h.div.style.cssText = h.div.style.cssText.replace(/color:#fff/, 'color:#111');
      }
    }
  };

  window.__iron_hints_deactivate = function() {
    if (window.__iron_hint_focus_trap) {
      document.removeEventListener('focus', window.__iron_hint_focus_trap, true);
      window.__iron_hint_focus_trap = null;
    }
    for (const h of _hints) h.div.remove();
    _hints = [];
    _selection = -1;
  };

  window.__iron_hints_typed = '';
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
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
        webview.evaluate_javascript(
            "__iron_hints_activate(); window.__iron_hints_typed = '';",
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Append `c` to the typed prefix and filter visible hints.
    /// Auto-clicks + deactivates if only one hint matches.
    pub fn handle_key(&mut self, c: char, webview: &WebView) {
        self.typed.push(c);
        webview.evaluate_javascript(
            &format!("window.__iron_hints_typed = '{}'; __iron_hints_filter('{}');", self.typed, self.typed),
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Pop the last typed character and re-filter.
    pub fn handle_backspace(&mut self, webview: &WebView) {
        if self.typed.pop().is_some() {
            let js = if self.typed.is_empty() {
                "window.__iron_hints_typed = ''; __iron_hints_filter('');".to_string()
            } else {
                format!("window.__iron_hints_typed = '{}'; __iron_hints_filter('{}');", self.typed, self.typed)
            };
            webview.evaluate_javascript(
                &js,
                None::<&str>,
                None::<&str>,
                None::<&Cancellable>,
                |_: Result<JsValue, JsError>| {},
            );
        }
    }

    /// Move selection to the next visible hint.
    pub fn select_next(&mut self, webview: &WebView) {
        webview.evaluate_javascript(
            "__iron_hints_select_next();",
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Move selection to the previous visible hint.
    pub fn select_prev(&mut self, webview: &WebView) {
        webview.evaluate_javascript(
            "__iron_hints_select_prev();",
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }

    /// Commit the current selection (click it), or click the single visible hint.
    pub fn commit(&mut self, webview: &WebView) {
        self.active = false;
        webview.evaluate_javascript(
            "__iron_hints_commit();",
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
        self.typed.clear();
    }

    /// Remove all hint overlays and exit hint mode.
    pub fn deactivate(&mut self, webview: &WebView) {
        self.active = false;
        self.typed.clear();
        webview.evaluate_javascript(
            "__iron_hints_deactivate();",
            None::<&str>,
            None::<&str>,
            None::<&Cancellable>,
            |_: Result<JsValue, JsError>| {},
        );
    }
}
