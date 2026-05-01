pub enum Command {
    Open(String),
    NewWindowOpen(String),
    Back,
    Forward,
    Reload,
    Settings,
    SetDefaultBrowser,
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
            "settings" | "set" => Some(Command::Settings),
            "default-browser" | "db" => Some(Command::SetDefaultBrowser),
            _ => None,
        }
    }
}
