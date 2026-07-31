//! Slash / bang parsing for the product TUI host (c480 / c492 / c494).

use crate::protocol::ports::XyBashResult;

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
    /// `/session-compact` with optional focus instructions (c1670).
    Compact {
        instructions: Option<String>,
    },
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

/// Whether a parsed slash may run under a host gate (c1580 / c1200).
///
/// Exhaustive over [`PendingSlash`] via [`slash_allowances`] — new variants MUST
/// fill every column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashPermit {
    /// Clear editor and enqueue `pending.slash` (or quit for Exit).
    Allow,
    /// Clear editor, scroll notice, MUST NOT dispatch.
    Reject,
}

/// Alias kept for call sites / specs that say «busy slash policy» (c1580).
pub type BusySlashPolicy = SlashPermit;

/// Per-slash allowances across host gates (c1200).
///
/// **Single exhaustive table**: add a [`PendingSlash`] arm → fill every field.
/// Future gates (e.g. reload-in-flight) add a column here — do not fork parallel matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashAllowances {
    pub when_agent_busy: SlashPermit,
    pub when_mcp_connecting: SlashPermit,
}

/// Canonical slash gate table. Prefer this over ad-hoc matches.
pub fn slash_allowances(slash: &PendingSlash) -> SlashAllowances {
    use SlashPermit::*;
    match slash {
        // Leave / readonly / export
        PendingSlash::Exit => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        PendingSlash::SessionDump => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        PendingSlash::HistoryCopyLast => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        PendingSlash::Export { .. } => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        // Session nav (MCP connecting: allow; agent busy: mostly reject)
        PendingSlash::OpenSessionResume => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::SessionNew => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::SessionClone => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::SessionName { .. } => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        PendingSlash::Import { .. } => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::OpenTree => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::ForkAtLeaf => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        // Chrome / trust
        PendingSlash::Theme { .. } => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::OpenModels => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::SetModel(_) => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Allow,
        },
        PendingSlash::Trust { .. } => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::DebugScene(_) => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        PendingSlash::Usage(_) => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Allow,
        },
        // Agent-like / races with bootstrap
        PendingSlash::Compact { .. } => SlashAllowances {
            when_agent_busy: Allow,
            when_mcp_connecting: Reject,
        },
        PendingSlash::Reload => SlashAllowances {
            when_agent_busy: Reject,
            when_mcp_connecting: Reject,
        },
    }
}

/// Agent-busy column (c1580).
pub fn busy_slash_policy(slash: &PendingSlash) -> SlashPermit {
    slash_allowances(slash).when_agent_busy
}

/// MCP connecting column (c1200) — independent of agent-busy.
pub fn mcp_connecting_slash_policy(slash: &PendingSlash) -> SlashPermit {
    slash_allowances(slash).when_mcp_connecting
}

/// Short label for refuse notes (`agent busy — /reload refused`).
pub fn busy_slash_refuse_label(slash: &PendingSlash) -> &'static str {
    match slash {
        PendingSlash::Reload => "/reload",
        PendingSlash::Trust { .. } => "/trust",
        PendingSlash::Theme { .. } => "/theme",
        PendingSlash::OpenModels => "/model",
        PendingSlash::OpenTree => "/session-tree",
        PendingSlash::ForkAtLeaf => "/session-fork",
        PendingSlash::OpenSessionResume => "/session-resume",
        PendingSlash::SessionNew => "/session-new",
        PendingSlash::SessionClone => "/session-clone",
        PendingSlash::Import { .. } => "/session-import",
        PendingSlash::DebugScene(_) => "/debug",
        PendingSlash::Usage(_) => "slash",
        // Allow variants are not refuse-noted; keep arms for exhaustiveness.
        PendingSlash::Exit => "/exit",
        PendingSlash::SetModel(_) => "/model",
        PendingSlash::Compact { .. } => "/session-compact",
        PendingSlash::Export { .. } => "/session-export",
        PendingSlash::SessionDump => "/session",
        PendingSlash::SessionName { .. } => "/session-name",
        PendingSlash::HistoryCopyLast => "/history-copy-last",
    }
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
        ("session-compact", None) => Some(PendingSlash::Compact { instructions: None }),
        ("session-compact", Some(text)) => Some(PendingSlash::Compact {
            instructions: Some(text),
        }),
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
        // Keep Full output footer if present near the end.
        if let Some(idx) = body.rfind("\n[Full output:") {
            let footer = body[idx + 1..].to_string();
            let head = &body[..idx.min(4000)];
            body = format!("{head}…\n{footer}");
        } else {
            body = format!("{}…", &body[..4000]);
        }
    }
    if result.truncated && !body.contains("[Full output:") {
        if !body.is_empty() {
            body.push('\n');
        }
        if let Some(path) = result.full_output_path.as_deref() {
            body.push_str(&format!(
                "[Full output: {path}. Truncated: (see file) lines shown (50.0KB limit)]"
            ));
        } else {
            body.push_str("(truncated)");
        }
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
    use super::{
        BusySlashPolicy, PendingSlash, SlashPermit, busy_slash_policy, mcp_connecting_slash_policy,
        parse_slash_command, slash_allowances,
    };

    #[test]
    fn busy_policy_allow_and_reject_table() {
        assert_eq!(
            busy_slash_policy(&PendingSlash::SessionName {
                name: Some("n".into())
            }),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::SetModel("m".into())),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::HistoryCopyLast),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::SessionDump),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Export { path: None }),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Exit),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Compact { instructions: None }),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Reload),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenModels),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Usage("x")),
            BusySlashPolicy::Reject
        );
    }

    #[test]
    fn mcp_connecting_slash_whitelist() {
        assert_eq!(
            mcp_connecting_slash_policy(&PendingSlash::OpenSessionResume),
            SlashPermit::Allow
        );
        assert_eq!(
            mcp_connecting_slash_policy(&PendingSlash::Reload),
            SlashPermit::Reject
        );
        assert_eq!(
            mcp_connecting_slash_policy(&PendingSlash::Compact { instructions: None }),
            SlashPermit::Reject
        );
        assert_eq!(
            mcp_connecting_slash_policy(&PendingSlash::Trust {
                mode: crate::app::core::driver::ProjectTrustMode::TrustCwd,
            }),
            SlashPermit::Allow
        );
        assert_eq!(
            mcp_connecting_slash_policy(&PendingSlash::Exit),
            SlashPermit::Allow
        );
    }

    #[test]
    fn slash_allowances_single_table_columns_differ() {
        let a = slash_allowances(&PendingSlash::OpenSessionResume);
        assert_eq!(a.when_agent_busy, SlashPermit::Reject);
        assert_eq!(a.when_mcp_connecting, SlashPermit::Allow);
        let b = slash_allowances(&PendingSlash::Compact { instructions: None });
        assert_eq!(b.when_agent_busy, SlashPermit::Allow);
        assert_eq!(b.when_mcp_connecting, SlashPermit::Reject);
    }

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
