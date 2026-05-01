//! Search engine registry and query builder.

use serde::{Deserialize, Serialize};

/// A named search engine with a URL template where `{}` is replaced by the query.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchEngine {
    pub name: String,
    pub template: String,
}

impl SearchEngine {
    /// Build a ready-to-navigate URL for this query.
    pub fn build_url(&self, query: &str) -> String {
        let encoded = urlencoding::encode(query);
        self.template.replace("{}", &encoded)
    }
}

/// Registry of all known engines, with one marked as default.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineRegistry {
    pub default: String,
    #[serde(default)]
    pub engines: Vec<SearchEngine>,
}

impl Default for EngineRegistry {
    fn default() -> Self {
        EngineRegistry {
            default: "duckduckgo".to_string(),
            engines: vec![
                SearchEngine {
                    name: "duckduckgo".to_string(),
                    template: "https://duckduckgo.com/?q={}".to_string(),
                },
                SearchEngine {
                    name: "google".to_string(),
                    template: "https://www.google.com/search?q={}".to_string(),
                },
            ],
        }
    }
}

impl EngineRegistry {
    /// Find an engine by name (case-insensitive).
    pub fn find(&self, name: &str) -> Option<&SearchEngine> {
        self.engines
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    /// Return the current default engine.
    pub fn default_engine(&self) -> Option<&SearchEngine> {
        self.find(&self.default)
    }

    /// Add or replace an engine.
    pub fn insert(&mut self, engine: SearchEngine) {
        if let Some(idx) = self
            .engines
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(&engine.name))
        {
            self.engines[idx] = engine;
        } else {
            self.engines.push(engine);
        }
    }

    /// Remove an engine by name. Returns `true` if something was removed.
    pub fn remove(&mut self, name: &str) -> bool {
        if let Some(idx) = self.engines.iter().position(|e| e.name.eq_ignore_ascii_case(name)) {
            self.engines.remove(idx);
            // if we just deleted the default, reset to the first remaining engine
            if self.default.eq_ignore_ascii_case(name) {
                self.default = self.engines.first().map(|e| e.name.clone()).unwrap_or_default();
            }
            true
        } else {
            false
        }
    }
}
