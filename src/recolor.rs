use std::collections::HashMap;
use std::path::PathBuf;

/// Noctalia palette tokens that we read from colors.json
#[derive(Debug, Clone)]
pub struct NoctaliaPalette {
    pub tokens: HashMap<String, String>, // name -> hex color
}

impl NoctaliaPalette {
    /// Load the Noctalia palette from ~/.config/noctalia/colors.json
    pub fn load() -> Option<Self> {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("noctalia")
            .join("colors.json");
        
        let content = std::fs::read_to_string(&path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        let mut tokens = HashMap::new();
        if let Some(obj) = json.as_object() {
            for (key, val) in obj {
                if let Some(hex) = val.as_str() {
                    tokens.insert(key.clone(), hex.to_string());
                }
            }
        }
        
        Some(NoctaliaPalette { tokens })
    }
    
    pub fn get(&self, token: &str) -> Option<&str> {
        self.tokens.get(token).map(|s| s.as_str())
    }
}

/// HSL color representation
#[derive(Debug, Clone, Copy)]
struct Hsl {
    h: f64, // 0-360
    s: f64, // 0-1
    l: f64, // 0-1
}

impl Hsl {
    /// Parse a hex color (#RRGGBB or #AARRGGBB) to HSL
    fn from_hex(hex: &str) -> Option<Self> {
        let t = hex.trim().trim_start_matches('#');
        let rgb = if t.len() == 6 {
            (
                u8::from_str_radix(&t[0..2], 16).ok()?,
                u8::from_str_radix(&t[2..4], 16).ok()?,
                u8::from_str_radix(&t[4..6], 16).ok()?,
            )
        } else if t.len() == 8 {
            // Skip alpha
            (
                u8::from_str_radix(&t[2..4], 16).ok()?,
                u8::from_str_radix(&t[4..6], 16).ok()?,
                u8::from_str_radix(&t[6..8], 16).ok()?,
            )
        } else {
            return None;
        };
        
        let r = rgb.0 as f64 / 255.0;
        let g = rgb.1 as f64 / 255.0;
        let b = rgb.2 as f64 / 255.0;
        
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        
        let s = if max == min {
            0.0
        } else {
            let d = max - min;
            if l > 0.5 {
                d / (2.0 - max - min)
            } else {
                d / (max + min)
            }
        };
        
        let h = if max == min {
            0.0
        } else if max == r {
            (60.0 * ((g - b) / (max - min)) + 360.0) % 360.0
        } else if max == g {
            (60.0 * ((b - r) / (max - min)) + 120.0) % 360.0
        } else {
            (60.0 * ((r - g) / (max - min)) + 240.0) % 360.0
        };
        
        Some(Hsl { h, s, l })
    }
    
    fn to_hex(&self) -> String {
        let c = |x: f64| {
            let p = self.l + self.s * (self.l - 0.5_f64).abs().mul_add(-2.0, 1.0);
            let q = 2.0 * self.l - p;
            let h = self.h / 360.0;
            let t = |n: f64| {
                let k = (n + h * 12.0) % 12.0;
                let a = self.s * (p - q).abs().min(q.abs());
                if k < 1.0 { q + a * k } 
                else if k < 3.0 { p } 
                else if k < 4.0 { q + a * (4.0 - k) } 
                else { q }
            };
                (t(x) * 255.0).round() as u8
        };
        
        let r = c(0.0);
        let g = c(8.0);
        let b = c(4.0);
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }
}

/// A hue bucket defines the target hue and chroma for a range of input hues
#[derive(Debug, Clone)]
struct HueBucket {
    center_hue: f64,
    chroma: f64,
}

/// Build the hue bucket mapping from Noctalia tokens
fn build_buckets(palette: &NoctaliaPalette) -> HashMap<String, HueBucket> {
    let mut buckets: HashMap<String, HueBucket> = HashMap::new();
    
    // Map key Noctalia tokens to hue buckets
    let token_mappings: Vec<(&str, &str)> = vec![
        ("mPrimary", "primary"),
        ("mSecondary", "secondary"),
        ("mTertiary", "tertiary"),
        ("mError", "error"),
    ];
    
    for (token_name, bucket_name) in token_mappings {
        if let Some(hex) = palette.get(token_name) {
            if let Some(hsl) = Hsl::from_hex(hex) {
                if hsl.s > 0.08 {
                    buckets.insert(bucket_name.to_string(), HueBucket {
                        center_hue: hsl.h,
                        chroma: hsl.s,
                    });
                }
            }
        }
    }
    
    // Fallback: if primary not found, use sensible defaults
    if !buckets.contains_key("primary") {
        buckets.insert("primary".to_string(), HueBucket { center_hue: 210.0, chroma: 0.5 });
    }
    if !buckets.contains_key("secondary") {
        buckets.insert("secondary".to_string(), HueBucket { center_hue: 180.0, chroma: 0.4 });
    }
    if !buckets.contains_key("tertiary") {
        buckets.insert("tertiary".to_string(), HueBucket { center_hue: 300.0, chroma: 0.4 });
    }
    if !buckets.contains_key("error") {
        buckets.insert("error".to_string(), HueBucket { center_hue: 0.0, chroma: 0.6 });
    }
    
    buckets
}

/// Build achromatic mapping (surface/onSurface tokens)
fn build_achromatic_map(palette: &NoctaliaPalette) -> (String, String, String, String) {
    let surface = palette.get("mSurface").unwrap_or("#1a1a1a").to_string();
    let on_surface = palette.get("mOnSurface").unwrap_or("#e6e6e6").to_string();
    let surface_variant = palette.get("mSurfaceVariant").unwrap_or("#2a2a2a").to_string();
    let on_surface_variant = palette.get("mOnSurfaceVariant").unwrap_or("#a0a0a0").to_string();
    
    (surface, on_surface, surface_variant, on_surface_variant)
}

/// Serialize the palette mapping to JSON for the JS engine
fn palette_to_json(palette: &NoctaliaPalette) -> String {
    let buckets = build_buckets(palette);
    let (surf, on_surf, surf_var, on_surf_var) = build_achromatic_map(palette);
    
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"buckets\": {\n");
    
    let mut first = true;
    for (name, bucket) in &buckets {
        if !first { json.push_str(",\n"); }
        first = false;
        json.push_str(&format!(
            "    \"{}\": {{ \"hue\": {:.1}, \"chroma\": {:.3} }}",
            name, bucket.center_hue, bucket.chroma
        ));
    }
    json.push_str("\n  },\n");
    json.push_str(&format!(
        "  \"achromatic\": {{\n    \"surface\": \"{}\",\n    \"onSurface\": \"{}\",\n    \"surfaceVariant\": \"{}\",\n    \"onSurfaceVariant\": \"{}\"\n  }}\n",
        surf, on_surf, surf_var, on_surf_var
    ));
    json.push_str("}");
    json
}

/// The JavaScript recoloring engine
const RECOLOR_JS: &str = r#"
(function() {
  const PALETTE = __IRON_PALETTE__;
  
  // Convert any CSS color string to HSL
  function parseColor(str) {
    if (!str || str === 'transparent' || str === 'none' || str === 'inherit' || str === 'initial' || str === 'unset') {
      return null;
    }
    
    str = str.trim().toLowerCase();
    
    // Named colors
    const namedColors = {
      black: [0,0,0], white: [255,255,255], red: [255,0,0], green: [0,128,0],
      blue: [0,0,255], yellow: [255,255,0], cyan: [0,255,255], magenta: [255,0,255],
      orange: [255,165,0], purple: [128,0,128], pink: [255,192,203],
      gray: [128,128,128], grey: [128,128,128], silver: [192,192,192],
      maroon: [128,0,0], olive: [128,128,0], lime: [0,255,0], teal: [0,128,128],
      navy: [0,0,128], aqua: [0,255,255], fuchsia: [255,0,255]
    };
    
    let r, g, b;
    
    if (str.startsWith('#')) {
      const hex = str.slice(1);
      if (hex.length === 3) {
        r = parseInt(hex[0]+hex[0], 16); g = parseInt(hex[1]+hex[1], 16); b = parseInt(hex[2]+hex[2], 16);
      } else if (hex.length === 6) {
        r = parseInt(hex.slice(0,2), 16); g = parseInt(hex.slice(2,4), 16); b = parseInt(hex.slice(4,6), 16);
      } else if (hex.length === 8) {
        r = parseInt(hex.slice(2,4), 16); g = parseInt(hex.slice(4,6), 16); b = parseInt(hex.slice(6,8), 16);
      } else { return null; }
    } else if (str.startsWith('rgb')) {
      const m = str.match(/rgba?\s*\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)/);
      if (m) { r = +m[1]; g = +m[2]; b = +m[3]; }
      else { return null; }
    } else if (str.startsWith('hsl')) {
      const m = str.match(/hsla?\s*\(\s*([\d.]+)\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%/);
      if (m) { return { h: +m[1], s: +m[2]/100, l: +m[3]/100 }; }
      else { return null; }
    } else if (namedColors[str]) {
      [r,g,b] = namedColors[str];
    } else {
      return null;
    }
    
    if (isNaN(r) || isNaN(g) || isNaN(b)) return null;
    
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    const l = (max + min) / 2;
    let s = 0, h = 0;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
      else if (max === g) h = ((b - r) / d + 2) * 60;
      else h = ((r - g) / d + 4) * 60;
    }
    return { h, s, l };
  }
  
  function hslToHex(h, s, l) {
    const c = (1 - Math.abs(2*l - 1)) * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = l - c / 2;
    let rp, gp, bp;
    if (h < 60) { rp = c; gp = x; bp = 0; }
    else if (h < 120) { rp = x; gp = c; bp = 0; }
    else if (h < 180) { rp = 0; gp = c; bp = x; }
    else if (h < 240) { rp = 0; gp = x; bp = c; }
    else if (h < 300) { rp = x; gp = 0; bp = c; }
    else { rp = c; gp = 0; bp = x; }
    const r = Math.round((rp + m) * 255);
    const g = Math.round((gp + m) * 255);
    const b = Math.round((bp + m) * 255);
    return '#' + [r,g,b].map(v => v.toString(16).padStart(2,'0')).join('');
  }
  
  // Determine which bucket a hue belongs to
  function getBucketName(hue) {
    // Find nearest bucket center
    let nearest = null, minDist = Infinity;
    for (const [name, bucket] of Object.entries(PALETTE.buckets)) {
      let d = Math.abs(hue - bucket.hue);
      if (d > 180) d = 360 - d;
      if (d < minDist) { minDist = d; nearest = name; }
    }
    return nearest;
  }
  
  function parseHex(hex) {
    const h = hex.slice(1);
    if (h.length === 3) {
      return [parseInt(h[0]+h[0],16), parseInt(h[1]+h[1],16), parseInt(h[2]+h[2],16)];
    }
    return [parseInt(h.slice(0,2),16), parseInt(h.slice(2,4),16), parseInt(h.slice(4,6),16)];
  }
  
  function rgbToLum(hex) {
    const [r,g,b] = parseHex(hex);
    // Perceived lightness
    return (0.299*r + 0.587*g + 0.114*b) / 255;
  }
  
  // Map a single HSL color through the palette
  function remapColor(h, s, l) {
    const ACHROMA_THRESHOLD = 0.08;
    
    // Achromatic: map lightness only using surface tokens
    if (s < ACHROMA_THRESHOLD) {
      const ac = PALETTE.achromatic;
      // Map lightness to surface scale
      // 0.0 -> surface (darkest), 1.0 -> onSurface (lightest)
      const surfL = rgbToLum(ac.surface);
      const onSurfL = rgbToLum(ac.onSurface);
      const surfVarL = rgbToLum(ac.surfaceVariant);
      const onSurfVarL = rgbToLum(ac.onSurfaceVariant);
      
      // Use a curve: very dark -> surface, very light -> onSurface, middle -> surfaceVariant/onSurfaceVariant
      let targetHex;
      if (l < 0.15) targetHex = ac.surface;
      else if (l < 0.35) targetHex = ac.surfaceVariant;
      else if (l < 0.65) targetHex = ac.onSurfaceVariant;
      else targetHex = ac.onSurface;
      
      return targetHex;
    }
    
    // Chromatic: find nearest bucket and shift
    const bucketName = getBucketName(h);
    if (!bucketName) return hslToHex(h, s, l);
    
    const bucket = PALETTE.buckets[bucketName];
    
    // Compute relative hue shift
    // If input is 20 deg from bucket center, output is 20 deg from theme center
    const sourceCenters = {
      primary: 210, secondary: 180, tertiary: 300, error: 0
    };
    const sourceCenter = sourceCenters[bucketName] || bucket.hue;
    const hueDelta = h - sourceCenter;
    const newHue = (bucket.hue + hueDelta + 360) % 360;
    
    // Scale chroma: preserve relative saturation
    const newSat = Math.min(1.0, s * (bucket.chroma / 0.5)); // normalize against assumed source chroma
    
    // Preserve lightness with slight curve adjustment for theme contrast
    const newLight = l;
    
    return hslToHex(newHue, newSat, newLight);
  }
  
  function remapValue(val) {
    const parsed = parseColor(val);
    if (!parsed) return val;
    const remapped = remapColor(parsed.h, parsed.s, parsed.l);
    return remapped || val;
  }
  
  // Extract colors from a complex CSS value (gradients, shadows, etc.)
  function remapComplexValue(val) {
    if (!val) return val;
    // Simple heuristic: replace color-like substrings
    // This is imperfect but covers most real-world cases
    return val.replace(/(#[0-9a-fA-F]{3,8}|rgba?\([^)]+\)|hsla?\([^)]+\))/g, function(match) {
      const remapped = remapValue(match);
      return remapped !== match ? remapped : match;
    });
  }
  
  const COLOR_PROPS = [
    'color', 'background-color', 'background',
    'border-color', 'border-top-color', 'border-right-color',
    'border-bottom-color', 'border-left-color',
    'outline-color', 'text-decoration-color',
    'box-shadow', 'text-shadow', 'fill', 'stroke',
    'caret-color', 'column-rule-color', 'accent-color',
    'border', 'border-top', 'border-right', 'border-bottom', 'border-left',
    'outline', 'text-decoration'
  ];
  
  // Elements that should never be recolored (pixel content)
  const SKIP_TAGS = new Set(['IMG', 'VIDEO', 'CANVAS', 'SVG', 'PICTURE', 'IFRAME']);
  
  function shouldSkipElement(el) {
    if (!el || !el.tagName) return false;
    if (SKIP_TAGS.has(el.tagName)) return true;
    return false;
  }
  
  function processStyle(style, el) {
    if (!style) return;
    for (const prop of COLOR_PROPS) {
      const val = style.getPropertyValue(prop);
      if (val) {
        let remapped;
        if (prop === 'box-shadow' || prop === 'text-shadow' || prop === 'background' || prop.startsWith('border')) {
          remapped = remapComplexValue(val);
        } else {
          remapped = remapValue(val);
        }
        if (remapped !== val) {
          try {
            style.setProperty(prop, remapped, 'important');
          } catch(e) {}
        }
      }
    }
  }
  
  function processSheet(sheet) {
    try {
      const rules = sheet.cssRules || sheet.rules;
      if (!rules) return;
      for (let i = 0; i < rules.length; i++) {
        const rule = rules[i];
        if (rule.style) {
          processStyle(rule.style, null);
        }
        if (rule.cssRules) {
          processSheet({ cssRules: rule.cssRules });
        }
      }
    } catch(e) {} // cross-origin sheets will throw, skip them
  }
  
  function processInlineStyles(root) {
    root = root || document;
    for (const el of root.querySelectorAll('[style]')) {
      if (!shouldSkipElement(el)) {
        processStyle(el.style, el);
      }
    }
  }
  
  function processShadowRoots(root) {
    root = root || document;
    for (const el of root.querySelectorAll('*')) {
      if (el.shadowRoot) {
        for (const sheet of el.shadowRoot.styleSheets) {
          processSheet(sheet);
        }
        processInlineStyles(el.shadowRoot);
        processShadowRoots(el.shadowRoot);
      }
    }
  }
  
  function processComputedStyles() {
    // For elements without inline styles, force color via inline to override computed
    // This is aggressive but ensures everything gets recolored
    const allEls = document.querySelectorAll('body, body *');
    for (const el of allEls) {
      if (shouldSkipElement(el)) continue;
      const computed = getComputedStyle(el);
      const color = computed.color;
      const bg = computed.backgroundColor;
      
      if (color && color !== 'rgba(0, 0, 0, 0)') {
        const remapped = remapValue(color);
        if (remapped !== color) {
          el.style.setProperty('color', remapped, 'important');
        }
      }
      if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
        const remapped = remapValue(bg);
        if (remapped !== bg) {
          el.style.setProperty('background-color', remapped, 'important');
        }
      }
    }
  }
  
  function applyRecolor() {
    // Process all stylesheets
    for (const sheet of document.styleSheets) {
      processSheet(sheet);
    }
    // Process inline styles
    processInlineStyles();
    // Process shadow DOM
    processShadowRoots();
    // Process computed styles for elements without explicit colors
    processComputedStyles();
  }
  
  window.__iron_recolor_apply = function() {
    applyRecolor();
    
    // MutationObserver for dynamically injected content
    const observer = new MutationObserver((mutations) => {
      let needsRecolor = false;
      for (const m of mutations) {
        for (const node of m.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            if (node.tagName === 'STYLE') {
              setTimeout(() => {
                if (node.sheet) processSheet(node.sheet);
              }, 50);
              needsRecolor = true;
            }
            if (node.tagName === 'LINK' && node.rel === 'stylesheet') {
              node.addEventListener('load', () => {
                if (node.sheet) processSheet(node.sheet);
              });
              needsRecolor = true;
            }
            if (!shouldSkipElement(node)) {
              if (node.style) processStyle(node.style, node);
              if (node.querySelectorAll) {
                for (const el of node.querySelectorAll('[style]')) {
                  if (!shouldSkipElement(el)) processStyle(el.style, el);
                }
              }
            }
          }
        }
        if (m.type === 'attributes' && m.attributeName === 'style') {
          if (!shouldSkipElement(m.target)) {
            processStyle(m.target.style, m.target);
          }
        }
      }
      if (needsRecolor) {
        setTimeout(applyRecolor, 100);
      }
    });
    
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['style', 'class']
    });
    
    window.__iron_recolor_observer = observer;
    
    // Re-apply periodically to catch lazy-loaded content
    window.__iron_recolor_interval = setInterval(applyRecolor, 2000);
  };
  
  window.__iron_recolor_deactivate = function() {
    if (window.__iron_recolor_observer) {
      window.__iron_recolor_observer.disconnect();
      window.__iron_recolor_observer = null;
    }
    if (window.__iron_recolor_interval) {
      clearInterval(window.__iron_recolor_interval);
      window.__iron_recolor_interval = null;
    }
  };
  
  // Auto-apply if palette was provided
  if (Object.keys(PALETTE.buckets).length > 0) {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', window.__iron_recolor_apply);
    } else {
      window.__iron_recolor_apply();
    }
  }
})();
"#;

/// Generate the complete recolor script with the current palette baked in.
/// Call this after Noctalia theme changes and inject via WebKit UserScript.
pub fn generate_recolor_script(palette: &NoctaliaPalette) -> String {
    let palette_json = palette_to_json(palette);
    RECOLOR_JS.replace("__IRON_PALETTE__", &palette_json)
}

/// Generate a minimal script that just exposes the palette for use by other JS
pub fn generate_palette_only(palette: &NoctaliaPalette) -> String {
    let palette_json = palette_to_json(palette);
    format!("window.__iron_palette = {};", palette_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hex_to_hsl() {
        let hsl = Hsl::from_hex("#ff0000").unwrap();
        assert!(hsl.h < 30.0 || hsl.h > 330.0); // red hue
        assert!(hsl.s > 0.9); // high saturation
    }
    
    #[test]
    fn test_gray_is_achromatic() {
        let hsl = Hsl::from_hex("#808080").unwrap();
        assert!(hsl.s < 0.1); // low saturation = achromatic
    }
    
    #[test]
    fn test_palette_load() {
        let palette = NoctaliaPalette {
            tokens: {
                let mut m = HashMap::new();
                m.insert("mPrimary".to_string(), "#3584e4".to_string());
                m.insert("mSurface".to_string(), "#1a1a1a".to_string());
                m.insert("mOnSurface".to_string(), "#e6e6e6".to_string());
                m
            }
        };
        let script = generate_recolor_script(&palette);
        assert!(script.contains("3584e4"));
        assert!(script.contains("__iron_recolor_apply"));
    }
}
