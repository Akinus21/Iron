// src/history.rs
use std::path::PathBuf;
use rusqlite::{Connection, params};

/// A single browsing history entry.
#[derive(Debug, Clone)]
pub struct HistoryItem {
    pub url: String,
    pub title: String,
    pub last_visit: i64,
    pub visit_count: i32,
}

/// Manages an SQLite-backed browsing history at
/// `~/.local/share/iron/history.sqlite`.
pub struct HistoryManager {
    db: Connection,
}

impl HistoryManager {
    pub fn new() -> Self {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let db = Connection::open(&path).unwrap_or_else(|_| {
            Connection::open_in_memory().expect("Cannot open in-memory SQLite either")
        });
        let _ = db.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                url TEXT PRIMARY KEY,
                title TEXT,
                last_visit INTEGER,
                visit_count INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_history_last_visit ON history(last_visit DESC);
            ");
        HistoryManager { db }
    }

    fn db_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("iron")
            .join("history.sqlite")
    }

    /// Record (or update) a visit to `url`.
    pub fn add(&mut self, url: &str, title: Option<&str>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = self.db.execute(
            "INSERT INTO history (url, title, last_visit, visit_count)
             VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(url) DO UPDATE SET
                 last_visit = excluded.last_visit,
                 visit_count = visit_count + 1,
                 title = COALESCE(excluded.title, history.title)",
            params![url, title.unwrap_or(""), now],
        );
    }

    /// Update the title for a URL (e.g. when the page finishes loading).
    pub fn update_title(&mut self, url: &str, title: &str) {
        if title.is_empty() {
            return;
        }
        let _ = self.db.execute(
            "UPDATE history SET title = ?1 WHERE url = ?2",
            params![title, url],
        );
    }

    /// Return the `limit` most recently visited items (unfiltered).
    pub fn recent(&self, limit: usize) -> Vec<HistoryItem> {
        self.query(
            "SELECT url, title, last_visit, visit_count
             FROM history
             ORDER BY last_visit DESC
             LIMIT ?1",
            params![limit as i64],
        )
    }

    /// Return all history items ordered by last_visit desc.
    pub fn all(&self) -> Vec<HistoryItem> {
        self.query(
            "SELECT url, title, last_visit, visit_count
             FROM history
             ORDER BY last_visit DESC",
            [],
        )
    }

    /// Fuzzy search history by URL and title, returning top `limit` matches
    /// ranked by fuzzy score.
    pub fn fuzzy(&self, query: &str, limit: usize) -> Vec<HistoryItem> {
        if query.is_empty() {
            return self.recent(limit);
        }
        let pattern = format!("%{}%", query);
        let raw = self.query(
            "SELECT url, title, last_visit, visit_count
             FROM history
             WHERE url LIKE ?1 OR title LIKE ?1
             ORDER BY last_visit DESC
             LIMIT ?2",
            params![&pattern, (limit * 3) as i64],
        );

        let mut scored: Vec<(i32, HistoryItem)> = raw
            .into_iter()
            .map(|item| {
                let url_score = crate::fuzzy::score(query, &item.url);
                let title_score = crate::fuzzy::score(query, &item.title);
                let best = url_score.max(title_score);
                (best, item)
            })
            .filter(|(s, _)| *s > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, item)| item).take(limit).collect()
    }

    /// Remove every entry from history.
    pub fn clear(&mut self) {
        let _ = self.db.execute("DELETE FROM history", []);
    }

    /// Remove a single URL from history.
    pub fn delete(&mut self, url: &str) {
        let _ = self.db.execute("DELETE FROM history WHERE url = ?1", params![url]);
    }

    fn query(&self,
        sql: &str,
        params: &[(&dyn rusqlite::ToSql)],
    ) -> Vec<HistoryItem> {
        let mut stmt = match self.db.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(params, |row| {
            Ok(HistoryItem {
                url: row.get(0)?,
                title: row.get(1)?,
                last_visit: row.get(2)?,
                visit_count: row.get(3)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
