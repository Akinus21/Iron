use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct NoctaliaTokens {
    #[serde(default)]
    pub accent_color: String,
    #[serde(default)]
    pub window_bg_color: String,
    #[serde(default)]
    pub fg_color: String,
}

impl NoctaliaTokens {
    pub fn load() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_default()
            .join("noctalia")
            .join("colors.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn to_css(&self) -> String {
        let mut css = String::new();
        if !self.accent_color.is_empty() {
            css.push_str(&format!("@define-color accent_color {};\n", self.accent_color));
        }
        if !self.window_bg_color.is_empty() {
            css.push_str(&format!("@define-color window_bg_color {};\n", self.window_bg_color));
        }
        if !self.fg_color.is_empty() {
            css.push_str(&format!("@define-color fg_color {};\n", self.fg_color));
        }
        css
    }
}
