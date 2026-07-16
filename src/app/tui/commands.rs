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
    /// `/session-tree` — open MessageHistory tree (c700 / c1005).
    OpenTree,
    /// `/session-fork` — fork at current leaf (c700 / c1005).
    ForkAtLeaf,
    /// Bare `/session-compact` (c1010).
    Compact,
    /// `/session-export` with optional path (c1010).
    Export {
        path: Option<String>,
    },
    /// `/session-import` with required path — confirm before dispatch (c1010).
    Import {
        path: String,
    },
    /// Bare `/session` — info/stats dump (c1015).
    SessionDump,
    /// Bare `/session-resume` — open session SelectList (c1015).
    OpenSessionResume,
    /// Bare `/session-new` — empty session (c1020).
    SessionNew,
    /// Bare `/session-clone` — fork leaf at At (c1020).
    SessionClone,
    /// `/session-name` with optional display name (c1020).
    SessionName {
        name: Option<String>,
    },
    /// Bare `/reload` — hot-reload runtime resources (c1120).
    Reload,
    /// `/trust` [parent|deny] — persist trust decision without auto-reload (c1105).
    Trust {
        mode: crate::app::core::driver::ProjectTrustMode,
    },
    /// Bare `/history-copy-last` — copy last assistant text (c1110; busy allowed).
    HistoryCopyLast,
    /// `/theme` [dark|light|toggle|cycle] — open picker or apply (c1115).
    Theme {
        arg: Option<String>,
    },
    /// Slash usage / arity error (no dispatch).
    Usage(&'static str),
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
        ("session-tree", None) => Some(PendingSlash::OpenTree),
        ("session-fork", None) => Some(PendingSlash::ForkAtLeaf),
        ("session-compact", None) => Some(PendingSlash::Compact),
        ("session-compact", Some(_)) => Some(PendingSlash::Usage(
            "usage: /session-compact (no arguments; custom instructions not supported)",
        )),
        ("session-export", path) => Some(PendingSlash::Export { path }),
        ("session-import", None) => Some(PendingSlash::Usage("usage: /session-import <path>")),
        ("session-import", Some(path)) => Some(PendingSlash::Import { path }),
        ("session", None) => Some(PendingSlash::SessionDump),
        ("session", Some(_)) => Some(PendingSlash::Usage("usage: /session (no arguments)")),
        ("session-resume", None) => Some(PendingSlash::OpenSessionResume),
        ("session-resume", Some(_)) => {
            Some(PendingSlash::Usage("usage: /session-resume (no arguments)"))
        }
        ("session-new", None) => Some(PendingSlash::SessionNew),
        ("session-new", Some(_)) => Some(PendingSlash::Usage("usage: /session-new (no arguments)")),
        ("session-clone", None) => Some(PendingSlash::SessionClone),
        ("session-clone", Some(_)) => {
            Some(PendingSlash::Usage("usage: /session-clone (no arguments)"))
        }
        ("session-name", name) => Some(PendingSlash::SessionName { name }),
        ("reload", None) => Some(PendingSlash::Reload),
        ("reload", Some(_)) => Some(PendingSlash::Usage("usage: /reload (no arguments)")),
        ("trust", None) => Some(PendingSlash::Trust {
            mode: crate::app::core::driver::ProjectTrustMode::TrustCwd,
        }),
        ("trust", Some(arg)) => match arg.to_ascii_lowercase().as_str() {
            "self" | "this_dir" | "this-dir" => Some(PendingSlash::Trust {
                mode: crate::app::core::driver::ProjectTrustMode::TrustCwd,
            }),
            "parent" => Some(PendingSlash::Trust {
                mode: crate::app::core::driver::ProjectTrustMode::TrustParent,
            }),
            "deny" | "no" => Some(PendingSlash::Trust {
                mode: crate::app::core::driver::ProjectTrustMode::Deny,
            }),
            _ => Some(PendingSlash::Usage(
                "usage: /trust [self|this_dir|parent|deny]",
            )),
        },
        ("history-copy-last", None) => Some(PendingSlash::HistoryCopyLast),
        ("history-copy-last", Some(_)) => Some(PendingSlash::Usage(
            "usage: /history-copy-last (no arguments)",
        )),
        ("theme", arg) => Some(PendingSlash::Theme { arg }),
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

#[cfg(test)]
mod parse_tests {
    use super::{PendingSlash, parse_slash_command};

    #[test]
    fn parse_reload_bare() {
        assert_eq!(parse_slash_command("/reload"), Some(PendingSlash::Reload));
        assert_eq!(
            parse_slash_command("  /reload  "),
            Some(PendingSlash::Reload)
        );
    }

    #[test]
    fn parse_reload_with_args_usage() {
        assert_eq!(
            parse_slash_command("/reload foo"),
            Some(PendingSlash::Usage("usage: /reload (no arguments)"))
        );
    }

    #[test]
    fn parse_trust_modes() {
        use crate::app::core::driver::ProjectTrustMode;
        assert_eq!(
            parse_slash_command("/trust"),
            Some(PendingSlash::Trust {
                mode: ProjectTrustMode::TrustCwd
            })
        );
        assert_eq!(
            parse_slash_command("/trust self"),
            Some(PendingSlash::Trust {
                mode: ProjectTrustMode::TrustCwd
            })
        );
        assert_eq!(
            parse_slash_command("/trust this_dir"),
            Some(PendingSlash::Trust {
                mode: ProjectTrustMode::TrustCwd
            })
        );
        assert_eq!(
            parse_slash_command("/trust parent"),
            Some(PendingSlash::Trust {
                mode: ProjectTrustMode::TrustParent
            })
        );
        assert_eq!(
            parse_slash_command("/trust deny"),
            Some(PendingSlash::Trust {
                mode: ProjectTrustMode::Deny
            })
        );
        assert_eq!(
            parse_slash_command("/trust foo"),
            Some(PendingSlash::Usage(
                "usage: /trust [self|this_dir|parent|deny]"
            ))
        );
    }

    #[test]
    fn parse_history_copy_last() {
        assert_eq!(
            parse_slash_command("/history-copy-last"),
            Some(PendingSlash::HistoryCopyLast)
        );
        assert_eq!(
            parse_slash_command("/history-copy-last x"),
            Some(PendingSlash::Usage(
                "usage: /history-copy-last (no arguments)"
            ))
        );
    }

    #[test]
    fn parse_theme() {
        assert_eq!(
            parse_slash_command("/theme"),
            Some(PendingSlash::Theme { arg: None })
        );
        assert_eq!(
            parse_slash_command("/theme light"),
            Some(PendingSlash::Theme {
                arg: Some("light".into())
            })
        );
        assert_eq!(
            parse_slash_command("/theme toggle"),
            Some(PendingSlash::Theme {
                arg: Some("toggle".into())
            })
        );
    }
}
