use gtk4;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct KeyBinding {
    pub key: String,
    #[serde(default)]
    pub modifier: Vec<String>,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Mode {
    #[serde(default)]
    pub bindings: Vec<KeyBinding>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub normal: Mode,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
        }

        Self::write_default(&config_path);
        Self::default()
    }

    fn config_path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("iron")
            .join("config.toml")
    }

    fn write_default(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let default = Self::default();
        let toml = toml::to_string(&default).unwrap_or_default();
        let _ = std::fs::write(path, toml);
    }

    fn modifier_flag(mod_str: &str) -> gtk4::gdk::ModifierType {
        match mod_str.to_uppercase().as_str() {
            "SHIFT" => gtk4::gdk::ModifierType::SHIFT_MASK,
            "CTRL" | "CONTROL" => gtk4::gdk::ModifierType::CONTROL_MASK,
            "ALT" => gtk4::gdk::ModifierType::ALT_MASK,
            "META" | "SUPER" | "WIN" => gtk4::gdk::ModifierType::META_MASK,
            _ => gtk4::gdk::ModifierType::empty(),
        }
    }

    pub fn get_binding_by_keyval(&self, keyval: gtk4::gdk::Key, modifier: &gtk4::gdk::ModifierType) -> Option<&KeyBinding> {
        let key_name = Self::keyval_to_string(keyval);

        for binding in &self.normal.bindings {
            if binding.key.to_lowercase() != key_name.to_lowercase() {
                continue;
            }

            let mut all_present = true;
            for mod_str in &binding.modifier {
                let flag = Self::modifier_flag(mod_str);
                if !modifier.contains(flag) {
                    all_present = false;
                    break;
                }
            }

            if all_present {
                return Some(binding);
            }
        }
        None
    }

    fn keyval_to_string(keyval: gtk4::gdk::Key) -> String {
        if let Some(c) = keyval.to_unicode() {
            return c.to_string();
        }

        match keyval {
            gtk4::gdk::Key::BackSpace => "backspace".to_string(),
            gtk4::gdk::Key::Tab => "tab".to_string(),
            gtk4::gdk::Key::Return => "return".to_string(),
            gtk4::gdk::Key::Escape => "escape".to_string(),
            gtk4::gdk::Key::Delete => "delete".to_string(),
            gtk4::gdk::Key::Up => "up".to_string(),
            gtk4::gdk::Key::Down => "down".to_string(),
            gtk4::gdk::Key::Left => "left".to_string(),
            gtk4::gdk::Key::Right => "right".to_string(),
            gtk4::gdk::Key::Home => "home".to_string(),
            gtk4::gdk::Key::End => "end".to_string(),
            gtk4::gdk::Key::Page_Up => "pageup".to_string(),
            gtk4::gdk::Key::Page_Down => "pagedown".to_string(),
            gtk4::gdk::Key::Insert => "insert".to_string(),
            gtk4::gdk::Key::KP_Enter => "kp-enter".to_string(),
            gtk4::gdk::Key::ISO_Enter => "iso-enter".to_string(),
            gtk4::gdk::Key::F1 => "f1".to_string(),
            gtk4::gdk::Key::F2 => "f2".to_string(),
            gtk4::gdk::Key::F3 => "f3".to_string(),
            gtk4::gdk::Key::F4 => "f4".to_string(),
            gtk4::gdk::Key::F5 => "f5".to_string(),
            gtk4::gdk::Key::F6 => "f6".to_string(),
            gtk4::gdk::Key::F7 => "f7".to_string(),
            gtk4::gdk::Key::F8 => "f8".to_string(),
            gtk4::gdk::Key::F9 => "f9".to_string(),
            gtk4::gdk::Key::F10 => "f10".to_string(),
            gtk4::gdk::Key::F11 => "f11".to_string(),
            gtk4::gdk::Key::F12 => "f12".to_string(),
            _ => format!("{:?}", keyval).to_lowercase(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            normal: Mode {
                bindings: vec![
                    KeyBinding {
                        key: "f".to_string(),
                        modifier: vec![],
                        action: "hint".to_string(),
                    },
                    KeyBinding {
                        key: "colon".to_string(),
                        modifier: vec!["shift".to_string()],
                        action: "command".to_string(),
                    },
                ],
            },
        }
    }
}