use xylitol_plugin_sdk::XyBashResult;

use super::bridge::{BashBlockStatus, UiEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingSlash {
    Exit,
    OpenModels,
    SetModel(String),
    OpenTree,
    ForkAtLeaf,
    Compact {
        instructions: Option<String>,
    },
    Export {
        path: Option<String>,
    },
    Import {
        path: String,
    },
    SessionDump,
    OpenSessionResume,
    SessionNew,
    SessionClone,
    SessionName {
        name: Option<String>,
    },
    Reload,
    Trust {
        mode: crate::driver::ProjectTrustMode,
    },
    HistoryCopyLast,
    Theme {
        arg: Option<String>,
    },
    OpenMcp,
    OpenCapabilities,
    OpenDesigningScenes,
    SetPlanMode(String),
    PluginCommand {
        plugin: String,
        args: String,
    },
    Usage(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashPermit {
    Allow,
    Reject,
}

pub type BusySlashPolicy = SlashPermit;

pub const BUSY_SESSION_SWITCH_NOTICE: &str =
    "agent busy — finish turn or Esc abort before switching session";

pub const RELOADING_WAIT_NOTICE: &str = "reloading — wait";

pub const RELOAD_CANCELLED_NOTICE: &str = "reload cancelled";

pub const RELOAD_FAILED_NOTICE: &str = "reload failed — see report";

pub const TOAST_NOTICE_TTL: std::time::Duration = std::time::Duration::from_secs(4);

pub const TOAST_NOTICE_ERROR_PREFIX: &str = "Error: ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashAllowances {
    pub when_agent_busy: SlashPermit,
    pub exec: xylitol_protocol::registry::Exec,
}

pub fn slash_allowances(slash: &PendingSlash) -> SlashAllowances {
    use SlashPermit::*;
    use xylitol_protocol::registry::Exec;
    match slash {
        PendingSlash::Exit => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Exclusive,
        },
        PendingSlash::SessionDump => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::HistoryCopyLast => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Inline,
        },
        PendingSlash::Export { .. } => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::OpenMcp | PendingSlash::OpenSessionResume => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::OpenCapabilities => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Inline,
        },
        PendingSlash::SessionNew | PendingSlash::SessionClone => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::SessionName { .. } => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::Import { .. } => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::OpenTree | PendingSlash::ForkAtLeaf => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::Theme { .. } => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Inline,
        },
        PendingSlash::OpenModels | PendingSlash::SetPlanMode(_) => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Inline,
        },
        PendingSlash::OpenDesigningScenes => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Inline,
        },
        PendingSlash::SetModel(_) => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::Trust { .. } => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::Usage(_) => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::Compact { .. } => SlashAllowances {
            when_agent_busy: Allow,
            exec: Exec::Queued,
        },
        PendingSlash::Reload => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Queued,
        },
        PendingSlash::PluginCommand { .. } => SlashAllowances {
            when_agent_busy: Reject,
            exec: Exec::Inline,
        },
    }
}

pub fn busy_slash_policy(slash: &PendingSlash) -> SlashPermit {
    slash_allowances(slash).when_agent_busy
}

pub fn busy_slash_refuse_label(slash: &PendingSlash) -> &'static str {
    match slash {
        PendingSlash::Reload => "/reload",
        PendingSlash::Trust { .. } => "/trust",
        PendingSlash::Theme { .. } => "/theme",
        PendingSlash::OpenModels | PendingSlash::SetModel(_) => "/model",
        PendingSlash::OpenTree => "/session-tree",
        PendingSlash::ForkAtLeaf => "/session-fork",
        PendingSlash::OpenSessionResume => "/session-resume",
        PendingSlash::SessionNew => "/session-new",
        PendingSlash::SessionClone => "/session-clone",
        PendingSlash::Import { .. } => "/session-import",
        PendingSlash::Usage(_) => "slash",
        PendingSlash::Exit => "/exit",
        PendingSlash::Compact { .. } => "/session-compact",
        PendingSlash::Export { .. } => "/session-export",
        PendingSlash::SessionDump => "/session",
        PendingSlash::SessionName { .. } => "/session-name",
        PendingSlash::HistoryCopyLast => "/history-copy-last",
        PendingSlash::OpenMcp => "/mcp",
        PendingSlash::OpenCapabilities => "/capabilities",
        PendingSlash::SetPlanMode(_) => "/plan",
        PendingSlash::OpenDesigningScenes => "/designing",
        PendingSlash::PluginCommand { .. } => "plugin",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingBash {
    pub command: String,
    pub exclude_from_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BangParse {
    NotBang,
    Empty {
        exclude_from_context: bool,
    },
    Cmd {
        command: String,
        exclude_from_context: bool,
    },
}

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
    if let Some(plugin) = cmd.strip_prefix("plugin:").filter(|p| !p.is_empty()) {
        return Some(PendingSlash::PluginCommand {
            plugin: plugin.to_string(),
            args: arg.unwrap_or_default(),
        });
    }
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
            mode: crate::driver::ProjectTrustMode::TrustCwd,
        }),
        ("trust", Some(arg)) => Some(parse_trust_arg(&arg)),
        ("history-copy-last", None) => Some(PendingSlash::HistoryCopyLast),
        ("history-copy-last", Some(_)) => Some(PendingSlash::Usage(
            "usage: /history-copy-last (no arguments)",
        )),
        ("theme", arg) => Some(PendingSlash::Theme { arg }),
        ("mcp" | "mcps", arg) => Some(parse_mcp_arg(arg)),
        ("capabilities" | "caps", arg) => Some(parse_capabilities_arg(arg)),
        ("plan", None) => Some(PendingSlash::SetPlanMode("toggle".into())),
        ("plan", Some(arg)) => Some(parse_plan_arg(&arg)),
        _ => None,
    }
}

fn parse_mcp_arg(arg: Option<String>) -> PendingSlash {
    match arg {
        None => PendingSlash::OpenMcp,
        Some(_) => PendingSlash::Usage("usage: /mcp (no arguments)"),
    }
}

fn parse_capabilities_arg(arg: Option<String>) -> PendingSlash {
    match arg {
        None => PendingSlash::OpenCapabilities,
        Some(_) => PendingSlash::Usage("usage: /capabilities (no arguments)"),
    }
}

fn parse_trust_arg(arg: &str) -> PendingSlash {
    use crate::driver::ProjectTrustMode;

    match arg.to_ascii_lowercase().as_str() {
        "self" | "this_dir" | "this-dir" => PendingSlash::Trust {
            mode: ProjectTrustMode::TrustCwd,
        },
        "parent" => PendingSlash::Trust {
            mode: ProjectTrustMode::TrustParent,
        },
        "deny" | "no" => PendingSlash::Trust {
            mode: ProjectTrustMode::Deny,
        },
        _ => PendingSlash::Usage("usage: /trust [self|this_dir|parent|deny]"),
    }
}

fn parse_plan_arg(arg: &str) -> PendingSlash {
    match arg.to_ascii_lowercase().as_str() {
        "on" | "enter" => PendingSlash::SetPlanMode("plan".into()),
        "off" | "exit" | "execute" => PendingSlash::SetPlanMode("execute".into()),
        _ => PendingSlash::Usage("usage: /plan [on|off]"),
    }
}

pub fn parse_slash_command_for_surface(text: &str, designing: bool) -> Option<PendingSlash> {
    if designing && let Some(slash) = parse_designing_slash(text) {
        return Some(slash);
    }
    parse_slash_command(text)
}

fn parse_designing_slash(text: &str) -> Option<PendingSlash> {
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
        ("designing", None) => Some(PendingSlash::OpenDesigningScenes),
        ("designing", Some(_)) => Some(PendingSlash::Usage("usage: /designing (no arguments)")),
        _ => None,
    }
}

pub fn bash_block_status(result: &XyBashResult) -> BashBlockStatus {
    if result.cancelled {
        BashBlockStatus::Cancelled
    } else if result.exit_code.is_some_and(|c| c != 0) {
        BashBlockStatus::Error
    } else {
        BashBlockStatus::Success
    }
}

pub fn bash_output_body(result: &XyBashResult) -> String {
    let code = result.exit_code;
    let mut body = result.output.trim_end().to_string();
    if body.len() > 4000 {
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

pub fn bash_result_entries(command: &str, result: &XyBashResult) -> Vec<UiEntry> {
    vec![UiEntry::Bash {
        id: crate::bridge::UiModel::bash_block_id_from_command(&[], command),
        command: command.to_string(),
        status: bash_block_status(result),
        output: bash_output_body(result),
        exclude_from_context: false,
    }]
}

#[cfg(test)]
mod parse_tests {
    use super::{
        BusySlashPolicy, PendingSlash, SlashPermit, busy_slash_policy, busy_slash_refuse_label,
        parse_bang_command, parse_slash_command, parse_slash_command_for_surface, slash_allowances,
    };

    #[test]
    fn slash_bang_parse_surface_snapshot() {
        let inputs = [
            "",
            "plain text",
            "/",
            "/unknown-verb",
            "/exit",
            "/quit",
            "/model",
            "/model gpt-x",
            "/session-tree",
            "/session-fork",
            "/session-compact",
            "/session-compact focus tests first",
            "/session-export",
            "/session-export /tmp/a.json",
            "/session-import",
            "/session-import x.json",
            "/session",
            "/session extra",
            "/session-resume",
            "/session-resume abc",
            "/session-new",
            "/session-new x",
            "/session-clone",
            "/session-clone x",
            "/session-name",
            "/session-name n1",
            "/reload",
            "/reload arg",
            "/trust",
            "/trust parent",
            "/trust deny",
            "/trust bogus",
            "/history-copy-last",
            "/history-copy-last arg",
            "/theme dark",
            "/mcp",
            "/mcps arg",
            "!",
            "!!",
            "!ls -la",
            "!!make test",
        ];
        let mut lines = Vec::with_capacity(inputs.len());
        for input in inputs {
            lines.push(format!(
                "{input:?} =>\n  slash={:?}\n  bang={:?}",
                parse_slash_command(input),
                parse_bang_command(input)
            ));
        }
        insta::assert_snapshot!("slash_bang_parse_surface", lines.join("\n"));
    }

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
            busy_slash_policy(&PendingSlash::Trust {
                mode: crate::driver::ProjectTrustMode::TrustCwd
            }),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenTree),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::ForkAtLeaf),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::SessionNew),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::SessionClone),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Import {
                path: "/tmp/x.jsonl".into()
            }),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenModels),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Theme { arg: None }),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenSessionResume),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::Usage("x")),
            BusySlashPolicy::Reject
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenMcp),
            BusySlashPolicy::Allow
        );
        assert_eq!(
            busy_slash_policy(&PendingSlash::OpenDesigningScenes),
            BusySlashPolicy::Allow
        );
    }

    #[test]
    fn slash_allowances_busy_only_table() {
        let a = slash_allowances(&PendingSlash::OpenSessionResume);
        assert_eq!(a.when_agent_busy, SlashPermit::Allow);
        let b = slash_allowances(&PendingSlash::Compact { instructions: None });
        assert_eq!(b.when_agent_busy, SlashPermit::Allow);
        let c = slash_allowances(&PendingSlash::Reload);
        assert_eq!(c.when_agent_busy, SlashPermit::Reject);
        let d = slash_allowances(&PendingSlash::OpenMcp);
        assert_eq!(d.when_agent_busy, SlashPermit::Allow);
        let e = slash_allowances(&PendingSlash::Theme { arg: None });
        assert_eq!(e.when_agent_busy, SlashPermit::Allow);
        let f = slash_allowances(&PendingSlash::OpenModels);
        assert_eq!(f.when_agent_busy, SlashPermit::Allow);
    }

    #[test]
    fn parse_plugin_command() {
        assert_eq!(
            parse_slash_command("/plugin:x-magpie refresh"),
            Some(PendingSlash::PluginCommand {
                plugin: "x-magpie".into(),
                args: "refresh".into(),
            })
        );
        assert_eq!(
            parse_slash_command("/plugin:x-magpie"),
            Some(PendingSlash::PluginCommand {
                plugin: "x-magpie".into(),
                args: String::new(),
            })
        );
        assert_eq!(parse_slash_command("/plugin:"), None);
    }

    #[test]
    fn parse_mcp_bare() {
        assert_eq!(parse_slash_command("/mcp"), Some(PendingSlash::OpenMcp));
        assert_eq!(parse_slash_command("/mcps"), Some(PendingSlash::OpenMcp));
        assert_eq!(
            parse_slash_command("/mcp extra"),
            Some(PendingSlash::Usage("usage: /mcp (no arguments)"))
        );
    }

    #[test]
    fn parse_capabilities_bare() {
        assert_eq!(
            parse_slash_command("/capabilities"),
            Some(PendingSlash::OpenCapabilities)
        );
        assert_eq!(
            parse_slash_command("/caps"),
            Some(PendingSlash::OpenCapabilities)
        );
        assert_eq!(
            parse_slash_command("/capabilities extra"),
            Some(PendingSlash::Usage("usage: /capabilities (no arguments)"))
        );
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
        use crate::driver::ProjectTrustMode;
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

    #[test]
    fn parse_designing_slash_only_on_designing_surface() {
        assert_eq!(parse_slash_command("/designing"), None);
        assert_eq!(parse_slash_command_for_surface("/designing", false), None);
        assert_eq!(
            parse_slash_command_for_surface("/designing", true),
            Some(PendingSlash::OpenDesigningScenes)
        );
        assert_eq!(
            parse_slash_command_for_surface("/designing extra", true),
            Some(PendingSlash::Usage("usage: /designing (no arguments)"))
        );
    }

    #[test]
    fn parse_plan_modes() {
        assert_eq!(
            parse_slash_command("/plan"),
            Some(PendingSlash::SetPlanMode("toggle".into()))
        );
        assert_eq!(
            parse_slash_command("/plan on"),
            Some(PendingSlash::SetPlanMode("plan".into()))
        );
        assert_eq!(
            parse_slash_command("/plan enter"),
            Some(PendingSlash::SetPlanMode("plan".into()))
        );
        assert_eq!(
            parse_slash_command("/plan off"),
            Some(PendingSlash::SetPlanMode("execute".into()))
        );
        assert_eq!(
            parse_slash_command("/plan exit"),
            Some(PendingSlash::SetPlanMode("execute".into()))
        );
        assert_eq!(
            parse_slash_command("/plan execute"),
            Some(PendingSlash::SetPlanMode("execute".into()))
        );
        assert_eq!(
            parse_slash_command("/plan sideways"),
            Some(PendingSlash::Usage("usage: /plan [on|off]"))
        );
    }

    #[test]
    fn busy_slash_refuse_label_covers_every_slash() {
        use crate::driver::ProjectTrustMode;

        let cases = [
            (PendingSlash::Reload, "/reload"),
            (
                PendingSlash::Trust {
                    mode: ProjectTrustMode::TrustCwd,
                },
                "/trust",
            ),
            (PendingSlash::Theme { arg: None }, "/theme"),
            (PendingSlash::OpenModels, "/model"),
            (PendingSlash::OpenTree, "/session-tree"),
            (PendingSlash::ForkAtLeaf, "/session-fork"),
            (PendingSlash::OpenSessionResume, "/session-resume"),
            (PendingSlash::SessionNew, "/session-new"),
            (PendingSlash::SessionClone, "/session-clone"),
            (PendingSlash::Import { path: "x".into() }, "/session-import"),
            (PendingSlash::Usage("u"), "slash"),
            (PendingSlash::Exit, "/exit"),
            (PendingSlash::SetModel("m".into()), "/model"),
            (
                PendingSlash::Compact { instructions: None },
                "/session-compact",
            ),
            (PendingSlash::Export { path: None }, "/session-export"),
            (PendingSlash::SessionDump, "/session"),
            (
                PendingSlash::SessionName {
                    name: Some("n".into()),
                },
                "/session-name",
            ),
            (PendingSlash::HistoryCopyLast, "/history-copy-last"),
            (PendingSlash::OpenMcp, "/mcp"),
            (PendingSlash::OpenCapabilities, "/capabilities"),
            (PendingSlash::SetPlanMode("plan".into()), "/plan"),
            (PendingSlash::OpenDesigningScenes, "/designing"),
        ];
        for (slash, expected) in cases {
            assert_eq!(busy_slash_refuse_label(&slash), expected, "{slash:?}");
        }
    }
}
