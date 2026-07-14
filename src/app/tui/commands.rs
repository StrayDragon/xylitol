//! Slash / bang parsing for the product TUI host (c480 / c492 / c494).

use crate::runtime_protocol::XyBashResult;

use super::bridge::{BashBlockStatus, UiEntry};

/// Idle slash resolved for the async host loop (or applied locally).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingSlash {
    Exit,
    /// Bare `/model` — open fuzzy picker (c630).
    OpenModels,
    SetModel(String),
    /// `/debug` / `/debug <scene>` hand-test fixtures (c710; debug builds).
    DebugScene(String),
    /// `/tree` — open MessageHistory tree (c700).
    OpenTree,
    /// `/fork` — fork at current leaf (c700).
    ForkAtLeaf,
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

/// Parse idle slash MVP (`/exit`, `/model` [id], and debug-build `/debug` [scene]).
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
        ("model", None) => Some(PendingSlash::OpenModels),
        ("model", Some(id)) => Some(PendingSlash::SetModel(id)),
        ("tree", None) => Some(PendingSlash::OpenTree),
        ("fork", None) => Some(PendingSlash::ForkAtLeaf),
        // Space form only (`/debug scene`). Colon form intentionally unsupported.
        #[cfg(debug_assertions)]
        ("debug", None) => Some(PendingSlash::DebugScene("list".into())),
        #[cfg(debug_assertions)]
        ("debug", Some(scene)) => Some(PendingSlash::DebugScene(scene)),
        _ => None,
    }
}

/// Map execute result → bang block tint.
pub fn bash_block_status(result: &XyBashResult) -> BashBlockStatus {
    if result.cancelled {
        BashBlockStatus::Cancelled
    } else if result.exit_code.is_some_and(|c| c != 0) {
        BashBlockStatus::Error
    } else {
        BashBlockStatus::Success
    }
}

/// Format bang outcome body (no `$ cmd` header — lives on the Bash block).
pub fn bash_output_body(result: &XyBashResult) -> String {
    let code = result.exit_code;
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
        if result.cancelled {
            return String::new();
        }
        return "(no output)".into();
    }
    body
}

/// Build a finished bang [`UiEntry::Bash`] (c668).
pub fn bash_result_entries(command: &str, result: &XyBashResult) -> Vec<UiEntry> {
    vec![UiEntry::Bash {
        command: command.to_string(),
        status: bash_block_status(result),
        output: bash_output_body(result),
        exclude_from_context: false,
    }]
}
