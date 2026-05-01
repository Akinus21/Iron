pub enum Command {
    Open(String),
    NewWindowOpen(String),
    Back,
    Forward,
    Reload,
    Duplicate,
    CopyAddress,
    Downloads,
    Settings,
    SetDefaultBrowser,
    CacStatus,
    SearchAdd(String, String),
    SearchDel(String),
    Search(String),
    Find(String),
    ClearSiteData,
    ClearCookies,
    History,
    ClearHistory,
    DeleteHistory(String),
    ReloadTheme,
}

/// Commands that accept a URL as their first argument.
pub const URL_COMMANDS: [&str; 4] = ["open", "o", "new-window-open", "nwo"];

/// Check whether a command name is one that expects a URL argument.
pub fn is_url_command(cmd: &str) -> bool {
    URL_COMMANDS.contains(&cmd)
}

pub struct CommandInput {
    pub raw: String,
}

impl CommandInput {
    pub fn new(raw: &str) -> Self {
        CommandInput {
            raw: raw.trim().to_string(),
        }
    }

    pub fn parse(&self) -> Option<Command> {
        if self.raw.is_empty() {
            return None;
        }

        // search commands need special handling for multi-word queries
        if self.raw.starts_with("search-add ") || self.raw == "search-add" {
            return Self::parse_search_add(&self.raw);
        }
        if self.raw.starts_with("search-del ") || self.raw == "search-del" {
            return Self::parse_search_del(&self.raw);
        }
        if self.raw.starts_with("search ") || self.raw == "search" {
            return Self::parse_search(&self.raw);
        }
        if self.raw.starts_with("find ") || self.raw == "find" {
            return Self::parse_find(&self.raw);
        }

        let (cmd, rest) = self.raw.split_once(' ').unwrap_or((&self.raw[..], ""));
        let rest = rest.trim();

        match cmd {
            "open" | "o" => {
                if rest.is_empty() {
                    None
                } else {
                    let url = if rest.starts_with("http") {
                        rest.to_string()
                    } else {
                        format!("https://{}", rest)
                    };
                    Some(Command::Open(url))
                }
            }
            "new-window-open" | "nwo" => {
                if rest.is_empty() {
                    None
                } else {
                    let url = if rest.starts_with("http") {
                        rest.to_string()
                    } else {
                        format!("https://{}", rest)
                    };
                    Some(Command::NewWindowOpen(url))
                }
            }
            "back" | "b" => Some(Command::Back),
            "forward" | "f" => Some(Command::Forward),
            "reload" | "r" => Some(Command::Reload),
            "duplicate" | "dup" => Some(Command::Duplicate),
            "copy-address" | "cpa" => Some(Command::CopyAddress),
            "settings" | "set" => Some(Command::Settings),
            "default-browser" | "db" => Some(Command::SetDefaultBrowser),
            "cac-status" | "cac" => Some(Command::CacStatus),
            "downloads" | "dl" => Some(Command::Downloads),
            "clear-site-data" | "csd" => Some(Command::ClearSiteData),
            "clear-cookies" | "cc" => Some(Command::ClearCookies),
            "history" | "hist" => Some(Command::History),
            "clear-history" | "ch" => Some(Command::ClearHistory),
            "delete-history" | "dh" => Some(Command::DeleteHistory(rest.to_string())),
            "reload-theme" | "rt" => Some(Command::ReloadTheme),
            _ => None,
        }
    }

    fn parse_search_add(raw: &str) -> Option<Command> {
        let after = raw.strip_prefix("search-add")?.trim();
        let (name, template) = after.split_once(' ')?;
        Some(Command::SearchAdd(name.trim().to_string(), template.trim().to_string()))
    }

    fn parse_search_del(raw: &str) -> Option<Command> {
        let after = raw.strip_prefix("search-del")?.trim();
        if after.is_empty() {
            None
        } else {
            Some(Command::SearchDel(after.to_string()))
        }
    }

    fn parse_search(raw: &str) -> Option<Command> {
        let after = raw.strip_prefix("search")?.trim();
        if after.is_empty() {
            None
        } else {
            Some(Command::Search(after.to_string()))
        }
    }

    fn parse_find(raw: &str) -> Option<Command> {
        let after = raw.strip_prefix("find")?.trim();
        if after.is_empty() {
            None
        } else {
            Some(Command::Find(after.to_string()))
        }
    }
}
