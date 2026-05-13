use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub struct CompatList {
    pub domains: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CompatToml {
    domains: Vec<String>,
}

impl Default for CompatList {
    fn default() -> Self {
        Self {
            domains: vec![
                "claude.ai".to_string(),
                "figma.com".to_string(),
                "docs.google.com".to_string(),
                "sheets.google.com".to_string(),
                "slides.google.com".to_string(),
            ],
        }
    }
}

impl CompatList {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".to_string())))
            .join("iron")
            .join("compat.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(data) = toml::from_str::<CompatToml>(&contents) {
                return Self { domains: data.domains };
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = CompatToml {
            domains: self.domains.clone(),
        };
        let contents = toml::to_string(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, contents)
    }

    pub fn add(&mut self, domain: &str) {
        if !self.domains.contains(&domain.to_string()) {
            self.domains.push(domain.to_string());
        }
    }

    pub fn remove(&mut self, domain: &str) -> bool {
        let pos = self.domains.iter().position(|d| *d == domain);
        if let Some(idx) = pos {
            self.domains.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> &[String] {
        &self.domains
    }

    pub fn matches(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return self.domains.iter().any(|d| host.contains(d));
            }
        }
        for domain in &self.domains {
            if url.contains(domain) {
                return true;
            }
        }
        false
    }

    pub fn open_external(url: &str) {
        let result = std::process::Command::new("xdg-open")
            .arg(url)
            .spawn();
        if let Err(e) = result {
            eprintln!("[compat] Failed to open {}: {}", url, e);
        }
    }
}