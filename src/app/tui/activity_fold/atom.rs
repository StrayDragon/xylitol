//! Exhaustive `UiEntry` → activity contribution.
//!
//! New transcript block = new `UiEntry` variant + an arm here + (tools only)
//! [`tool_activity_role`]. No `_ => {}`. No runtime plugin registry.

use crate::app::tui::bridge::UiEntry;
use crate::protocol::tool_name::is_mcp_tool_name;

/// Identity of the in-flight thinking burst (not yet a `UiEntry::Thinking`).
/// Distinct from [`crate::app::tui::bridge::allocate_thinking_id`] (`{hash}-{n}`).
pub const STREAMING_THINK_ID: &str = "live:streaming";

/// How an explore atom counts pathless calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExploreKind {
    /// `read` / `ls` — pathless still occupies an anonymous file slot.
    File,
    /// grep / glob / find — pathless qualifies Explored without a fake N.
    Search,
}

/// One middle's contribution to a cluster header.
///
/// Counting is encoded in the variant: unique paths (Edit / Explore), call
/// count (Run / Used), omit (Noise / Projection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityAtom {
    Edit {
        path: Option<String>,
        diff_pm: Option<(u32, u32)>,
    },
    Explore {
        path: Option<String>,
        kind: ExploreKind,
    },
    Run,
    Used {
        display_name: String,
    },
    /// Live identity is the Thinking entry id, or [`STREAMING_THINK_ID`].
    Think {
        id: String,
    },
    Ask {
        id: String,
    },
    Compaction,
    /// Envelope / chrome rows (User, Assistant, notices, bang Bash). Omitted.
    Noise,
    /// Todo checklist is a projection of `todo_*` results, not a Used call.
    Projection,
}

impl ActivityAtom {
    pub fn streaming_think() -> Self {
        Self::Think {
            id: STREAMING_THINK_ID.into(),
        }
    }

    pub fn is_foldable_middle(&self) -> bool {
        !matches!(self, Self::Noise)
    }
}

/// Closed tool-name → role table. Unknown names (MCP, `todo_*`, new tools)
/// are Used-by-call. MUST NOT use `contains("search")` — that silently
/// promoted unknown tools to Explored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolActivityRole {
    Edit,
    ExploreFile,
    ExploreSearch,
    Run,
    Used,
}

pub fn tool_activity_role(name: &str) -> ToolActivityRole {
    let n = name.to_ascii_lowercase();
    match n.as_str() {
        "edit" | "write" | "apply_patch" | "strreplace" | "str_replace" => ToolActivityRole::Edit,
        "read" | "ls" => ToolActivityRole::ExploreFile,
        "grep" | "rg" | "search" | "glob" | "find" | "codebase_search" | "semantic_search" => {
            ToolActivityRole::ExploreSearch
        }
        "bash" | "shell" | "run_terminal_cmd" | "execute" => ToolActivityRole::Run,
        _ => ToolActivityRole::Used,
    }
}

/// Exhaustive: adding a `UiEntry` variant without an arm is a compile error.
pub fn activity_atom(entry: &UiEntry) -> ActivityAtom {
    match entry {
        UiEntry::Tool {
            name,
            args_preview,
            tool_path,
            display_diff,
            ..
        } => fill_tool_atom(name, tool_path, args_preview, display_diff.as_deref()),
        UiEntry::Diff { display_diff, .. } => ActivityAtom::Edit {
            path: None,
            diff_pm: count_diff_pm(display_diff),
        },
        UiEntry::Thinking { id, .. } => ActivityAtom::Think { id: id.clone() },
        UiEntry::Ask { id, .. } => ActivityAtom::Ask { id: id.clone() },
        UiEntry::Compaction { .. } => ActivityAtom::Compaction,
        UiEntry::Todo { .. } => ActivityAtom::Projection,
        UiEntry::Bash { .. }
        | UiEntry::User { .. }
        | UiEntry::Assistant { .. }
        | UiEntry::ScrollNotice { .. }
        | UiEntry::Error { .. } => ActivityAtom::Noise,
    }
}

fn fill_tool_atom(
    name: &str,
    tool_path: &Option<String>,
    args_preview: &str,
    display_diff: Option<&str>,
) -> ActivityAtom {
    let path = tool_path_of(tool_path, args_preview);
    let diff_pm = display_diff.and_then(count_diff_pm);
    match tool_activity_role(name) {
        ToolActivityRole::Edit => ActivityAtom::Edit { path, diff_pm },
        ToolActivityRole::ExploreFile => ActivityAtom::Explore {
            path,
            kind: ExploreKind::File,
        },
        ToolActivityRole::ExploreSearch => ActivityAtom::Explore {
            path,
            kind: ExploreKind::Search,
        },
        ToolActivityRole::Run => ActivityAtom::Run,
        ToolActivityRole::Used => ActivityAtom::Used {
            display_name: used_display_name(name),
        },
    }
}

fn used_display_name(name: &str) -> String {
    if is_mcp_tool_name(name) {
        if let Some(rest) = name.strip_prefix("mcp__") {
            return rest.rsplit("__").next().unwrap_or(rest).to_string();
        }
        if let Some(rest) = name.strip_prefix("mcp:") {
            return rest.rsplit(':').next().unwrap_or(rest).to_string();
        }
        if let Some(rest) = name.strip_prefix("mcp-") {
            return rest.rsplit('-').next().unwrap_or(rest).to_string();
        }
    }
    name.to_string()
}

/// Streaming / empty path chrome (`preview.rs` `PATH_PLACEHOLDER`), not a real file.
pub(crate) fn is_path_placeholder(p: &str) -> bool {
    let p = p.trim();
    p.is_empty() || p == "..." || p == "…" || p == "$ ..." || p.starts_with("...")
}

fn tool_path_of(tool_path: &Option<String>, args_preview: &str) -> Option<String> {
    if let Some(p) = tool_path.as_deref().map(str::trim)
        && !is_path_placeholder(p)
    {
        return Some(p.to_string());
    }
    let preview = args_preview.trim();
    if is_path_placeholder(preview) {
        return None;
    }
    preview
        .split_whitespace()
        .next_back()
        .filter(|t| !is_path_placeholder(t) && (t.contains('.') || t.contains('/')))
        .map(str::to_string)
}

/// Count +/- lines in a unified diff; `None` when nothing reliable.
pub fn count_diff_pm(diff: &str) -> Option<(u32, u32)> {
    let mut plus = 0u32;
    let mut minus = 0u32;
    let mut any = false;
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
            continue;
        }
        if line.starts_with('+') {
            plus += 1;
            any = true;
        } else if line.starts_with('-') {
            minus += 1;
            any = true;
        }
    }
    if any { Some((plus, minus)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::{AskPhase, BashBlockStatus, CompactionBlockStatus, UiEntry};

    fn tool(name: &str, path: Option<&str>) -> UiEntry {
        UiEntry::Tool {
            id: name.into(),
            name: name.into(),
            args_preview: path.unwrap_or("").into(),
            tool_path: path.map(str::to_string),
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: true,
        }
    }

    fn assert_atom(entry: UiEntry, pred: impl FnOnce(&ActivityAtom) -> bool) {
        let atom = activity_atom(&entry);
        assert!(pred(&atom), "{entry:?} → {atom:?}");
    }

    #[test]
    fn every_ui_entry_variant_maps_without_wildcard() {
        assert_atom(UiEntry::User { text: "u".into() }, |a| {
            matches!(a, ActivityAtom::Noise)
        });
        assert_atom(UiEntry::Assistant { text: "a".into() }, |a| {
            matches!(a, ActivityAtom::Noise)
        });
        assert_atom(
            UiEntry::Thinking {
                id: "t1".into(),
                text: "hmm".into(),
                elapsed_secs: None,
            },
            |a| matches!(a, ActivityAtom::Think { id } if id == "t1"),
        );
        assert_atom(tool("read", Some("a.rs")), |a| {
            matches!(
                a,
                ActivityAtom::Explore {
                    kind: ExploreKind::File,
                    ..
                }
            )
        });
        assert_atom(
            UiEntry::Ask {
                id: "a1".into(),
                summary: "Ask".into(),
                detail_lines: vec![],
                phase: AskPhase::Waiting,
                expanded: false,
            },
            |a| matches!(a, ActivityAtom::Ask { id } if id == "a1"),
        );
        assert_atom(
            UiEntry::Diff {
                summary: "d".into(),
                display_diff: String::new(),
            },
            |a| matches!(a, ActivityAtom::Edit { path: None, .. }),
        );
        assert_atom(
            UiEntry::Bash {
                command: "ls".into(),
                status: BashBlockStatus::Success,
                output: String::new(),
                exclude_from_context: false,
            },
            |a| matches!(a, ActivityAtom::Noise),
        );
        assert_atom(
            UiEntry::Compaction {
                status: CompactionBlockStatus::Complete,
                summary: "c".into(),
                tokens_before: 1,
                detail: None,
            },
            |a| matches!(a, ActivityAtom::Compaction),
        );
        assert_atom(
            UiEntry::Todo {
                summary: "Todo".into(),
                detail_lines: vec![],
            },
            |a| matches!(a, ActivityAtom::Projection),
        );
        assert_atom(UiEntry::ScrollNotice { text: "n".into() }, |a| {
            matches!(a, ActivityAtom::Noise)
        });
        assert_atom(UiEntry::Error { text: "e".into() }, |a| {
            matches!(a, ActivityAtom::Noise)
        });
    }

    #[test]
    fn unknown_searchish_name_is_used_not_explored() {
        let atom = activity_atom(&tool("my_custom_search", None));
        assert!(
            matches!(atom, ActivityAtom::Used { ref display_name } if display_name == "my_custom_search"),
            "{atom:?}"
        );
    }

    #[test]
    fn streaming_think_binds_sentinel_id() {
        let atom = ActivityAtom::streaming_think();
        assert!(matches!(atom, ActivityAtom::Think { ref id } if id == STREAMING_THINK_ID));
        assert_ne!(
            atom,
            activity_atom(&UiEntry::Thinking {
                id: "abcd1234-0".into(),
                text: "x".into(),
                elapsed_secs: None,
            })
        );
    }

    #[test]
    fn todo_tools_are_used_checklist_is_projection() {
        assert!(matches!(
            activity_atom(&tool("todo_update", None)),
            ActivityAtom::Used { .. }
        ));
        assert!(matches!(
            activity_atom(&UiEntry::Todo {
                summary: "Todo".into(),
                detail_lines: vec![],
            }),
            ActivityAtom::Projection
        ));
    }
}
