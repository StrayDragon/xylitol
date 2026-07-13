//! Slash / bang parsing for the product TUI host (c480 / c492 / c494).

use crate::runtime_protocol::XyBashResult;

use super::bridge::UiEntry;

/// Idle slash resolved for the async host loop (or applied locally).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingSlash {
    Exit,
    CycleModel,
    SetModel(String),
}

/// Idle `!` / `!!` bash request for the async host loop (c492).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingBash {
    pub command: String,
    pub exclude_from_context: bool,
}

/// Parse trimmed editor text for bang-bash (c492).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BangParse {
    NotBang,
    /// `!` or `!!` with empty command body.
    Empty {
        exclude_from_context: bool,
    },
    Cmd {
        command: String,
        exclude_from_context: bool,
    },
}

/// Classify idle input for `!` / `!!` bash (after slash handling).
pub fn parse_bang_command(text: &str) -> BangParse {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("!!") {
        let cmd = rest.trim();
        if cmd.is_empty() {
            BangParse::Empty {
                exclude_from_context: true,
            }
        } else {
            BangParse::Cmd {
                command: cmd.to_string(),
                exclude_from_context: true,
            }
        }
    } else if let Some(rest) = trimmed.strip_prefix('!') {
        let cmd = rest.trim();
        if cmd.is_empty() {
            BangParse::Empty {
                exclude_from_context: false,
            }
        } else {
            BangParse::Cmd {
                command: cmd.to_string(),
                exclude_from_context: false,
            }
        }
    } else {
        BangParse::NotBang
    }
}

/// Parse idle slash MVP (`/exit`, `/model` [id]).
pub fn parse_slash_command(text: &str) -> Option<PendingSlash> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix('/')?;
    if rest.is_empty() {
        return None;
    }
    let mut parts = rest.splitn(2, char::is_whitespace);
    let cmd = parts.next()?.to_ascii_lowercase();
    let arg = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    match (cmd.as_str(), arg) {
        ("exit" | "quit", _) => Some(PendingSlash::Exit),
        ("model", None) => Some(PendingSlash::CycleModel),
        ("model", Some(id)) => Some(PendingSlash::SetModel(id)),
        _ => None,
    }
}

/// Format bash outcome lines for live scrollback (c492 / att9).
pub fn bash_result_entries(command: &str, result: &XyBashResult) -> Vec<UiEntry> {
    let mut out = vec![UiEntry::System {
        text: format!("$ {command}"),
    }];
    let code = result.exit_code;
    let failed = result.cancelled || code.is_some_and(|c| c != 0);
    let mut body = result.output.trim_end().to_string();
    if body.len() > 4000 {
        body = format!("{}…", &body[..4000]);
    }
    if result.truncated {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str("(truncated)");
    }
    if result.cancelled {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str("(cancelled)");
    }
    if let Some(c) = code {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&format!("(exit {c})"));
    }
    if body.is_empty() {
        body = "(no output)".into();
    }
    if failed {
        out.push(UiEntry::Error { text: body });
    } else {
        out.push(UiEntry::System { text: body });
    }
    out
}
