//! Cluster / envelope summary lines (att24 / att27).

use time::OffsetDateTime;

use crate::app::tui::bridge::UiEntry;
use crate::app::tui::keybindings::with_keybindings;
use crate::app::tui::widgets::GlyphSet;
use crate::protocol::tool_name::is_mcp_tool_name;

#[cfg(test)]
use super::segment::SegmentLevel;
use super::segment::{
    ActivityCluster, ActivitySegment, cluster_middle_indices, middle_entry_indices,
};

/// Observable activity for one cluster header (att24).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    /// Unique edit/write paths (first-seen order). Pathless edits use a placeholder.
    pub edit_paths: Vec<String>,
    /// Unique read/ls/search paths.
    pub explore_paths: Vec<String>,
    /// Search ran with no usable path — still qualifies Explored, no fake N.
    pub search_no_path: bool,
    pub commands: u32,
    pub thinking: u32,
    /// MCP / unknown / todo_* display names (first-seen, unique; N=1 header).
    pub used_names: Vec<String>,
    /// Invocation count of those tools (Ran-style; N>1 header). Checklist row is not a call.
    pub used_calls: u32,
    pub asks: u32,
    pub compaction: u32,
    /// Reliable +/- from Diff / edit display_diff only.
    pub diff_plus: Option<u32>,
    pub diff_minus: Option<u32>,
}

impl ActivityCounts {
    /// Compaction-only: no cluster header (att23); paint the compaction block itself.
    pub fn omits_cluster_header(&self) -> bool {
        self.compaction > 0
            && self.edit_paths.is_empty()
            && self.explore_paths.is_empty()
            && !self.search_no_path
            && self.commands == 0
            && self.thinking == 0
            && self.used_names.is_empty()
            && self.used_calls == 0
            && self.asks == 0
    }

    pub fn is_thought_only(&self) -> bool {
        self.thinking > 0
            && self.edit_paths.is_empty()
            && self.explore_paths.is_empty()
            && !self.search_no_path
            && self.commands == 0
            && self.used_names.is_empty()
            && self.used_calls == 0
            && self.asks == 0
            && self.compaction == 0
    }
}

pub fn count_segment(entries: &[UiEntry], seg: &ActivitySegment) -> ActivityCounts {
    count_middles(entries, &middle_entry_indices(entries, seg))
}

/// Counts for one cluster only (open-cluster -2 vs frozen -3).
pub fn count_cluster(entries: &[UiEntry], cluster: &ActivityCluster) -> ActivityCounts {
    count_middles(entries, &cluster_middle_indices(entries, cluster))
}

pub fn cluster_omits_header(entries: &[UiEntry], cluster: &ActivityCluster) -> bool {
    count_cluster(entries, cluster).omits_cluster_header()
}

/// Thought-only cluster: fold into the Thinking/Thought header, no second L1 row.
pub fn cluster_is_thought_only(entries: &[UiEntry], cluster: &ActivityCluster) -> bool {
    count_cluster(entries, cluster).is_thought_only()
}

/// Counts for a live thinking stream before it is flushed to a Thinking entry.
pub fn streaming_thought_counts() -> ActivityCounts {
    ActivityCounts {
        thinking: 1,
        ..Default::default()
    }
}

fn push_unique(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|p| p == &path) {
        paths.push(path);
    }
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
    // Human preview is `{name} {path}` or just a path-ish token.
    preview
        .split_whitespace()
        .next_back()
        .filter(|t| !is_path_placeholder(t) && (t.contains('.') || t.contains('/')))
        .map(str::to_string)
}

fn count_middles(entries: &[UiEntry], indices: &[usize]) -> ActivityCounts {
    let mut c = ActivityCounts::default();
    let mut plus = 0u32;
    let mut minus = 0u32;
    let mut saw_diff_stats = false;
    let mut anon_edits = 0u32;
    let mut anon_explores = 0u32;

    for &idx in indices {
        match &entries[idx] {
            UiEntry::Tool {
                name,
                args_preview,
                tool_path,
                display_diff,
                ..
            } => {
                let n = name.to_ascii_lowercase();
                if is_search_tool(&n) {
                    match tool_path_of(tool_path, args_preview) {
                        Some(p) => push_unique(&mut c.explore_paths, p),
                        None => c.search_no_path = true,
                    }
                } else if is_command_tool(&n) {
                    c.commands += 1;
                } else if is_edit_tool(&n) {
                    match tool_path_of(tool_path, args_preview) {
                        Some(p) => push_unique(&mut c.edit_paths, p),
                        None => anon_edits += 1,
                    }
                } else if is_read_or_ls(&n) {
                    match tool_path_of(tool_path, args_preview) {
                        Some(p) => push_unique(&mut c.explore_paths, p),
                        None => anon_explores += 1,
                    }
                } else {
                    c.used_calls = c.used_calls.saturating_add(1);
                    push_unique(&mut c.used_names, used_display_name(name));
                }
                if let Some(diff) = display_diff.as_deref()
                    && let Some((p, m)) = count_diff_pm(diff)
                {
                    plus = plus.saturating_add(p);
                    minus = minus.saturating_add(m);
                    saw_diff_stats = true;
                }
            }
            UiEntry::Diff { display_diff, .. } => {
                anon_edits += 1;
                if let Some((p, m)) = count_diff_pm(display_diff) {
                    plus = plus.saturating_add(p);
                    minus = minus.saturating_add(m);
                    saw_diff_stats = true;
                }
            }
            UiEntry::Bash { .. } => c.commands += 1,
            UiEntry::Thinking { .. } => c.thinking += 1,
            UiEntry::Ask { .. } => c.asks += 1,
            UiEntry::Compaction { .. } => c.compaction += 1,
            // Checklist (UiEntry::Todo) is a projection of todo_* results, not a Used call.
            _ => {}
        }
    }

    for i in 0..anon_edits {
        c.edit_paths.push(format!("\0edit{i}"));
    }
    for i in 0..anon_explores {
        c.explore_paths.push(format!("\0explore{i}"));
    }

    if saw_diff_stats {
        c.diff_plus = Some(plus);
        c.diff_minus = Some(minus);
    }
    c
}

fn is_search_tool(name: &str) -> bool {
    matches!(
        name,
        "grep" | "rg" | "search" | "glob" | "find" | "codebase_search" | "semantic_search"
    ) || name.contains("search")
        || name.contains("grep")
}

fn is_command_tool(name: &str) -> bool {
    matches!(name, "bash" | "shell" | "run_terminal_cmd" | "execute")
}

fn is_edit_tool(name: &str) -> bool {
    matches!(
        name,
        "edit" | "write" | "apply_patch" | "strreplace" | "str_replace"
    )
}

fn is_read_or_ls(name: &str) -> bool {
    matches!(name, "read" | "ls")
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

fn basename(path: &str) -> &str {
    if path.starts_with('\0') {
        return "";
    }
    for part in path.rsplit(['/', '\\']) {
        let p = part.trim();
        if p.is_empty() || p == "." || p == ".." {
            continue;
        }
        if is_path_placeholder(p) {
            return "";
        }
        return p;
    }
    ""
}

fn files_word(n: u32) -> &'static str {
    if n == 1 { "file" } else { "files" }
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

fn tool_word(n: u32) -> &'static str {
    if n == 1 { "tool" } else { "tools" }
}

fn command_word(n: u32) -> &'static str {
    if n == 1 { "command" } else { "commands" }
}

fn file_clause(verb: &str, paths: &[String]) -> String {
    let n = paths.len() as u32;
    if n == 1 {
        let base = basename(&paths[0]);
        if !base.is_empty() {
            return format!("{verb} {base}");
        }
        return format!("{verb} 1 file");
    }
    format!("{verb} {n} {}", files_word(n))
}

#[cfg(test)]
pub fn format_l2_body(counts: &ActivityCounts) -> String {
    format_cluster_body(counts, false)
}

/// Open-cluster progressive vs sealed (att24).
pub fn format_cluster_body(counts: &ActivityCounts, progressive: bool) -> String {
    if counts.omits_cluster_header() {
        return String::new();
    }
    let mut parts = Vec::new();
    let (file_verb, cmd_verb) = if progressive {
        ("Editing", "Running")
    } else {
        ("Edited", "Ran")
    };
    if !counts.edit_paths.is_empty() {
        parts.push(file_clause(file_verb, &counts.edit_paths));
    } else if !counts.explore_paths.is_empty() {
        let explore_verb = if progressive { "Exploring" } else { "Explored" };
        parts.push(file_clause(explore_verb, &counts.explore_paths));
    } else if counts.search_no_path {
        parts.push(if progressive {
            "Exploring".into()
        } else {
            "Explored".into()
        });
    }
    if counts.commands > 0 {
        parts.push(format!(
            "{cmd_verb} {} {}",
            counts.commands,
            command_word(counts.commands)
        ));
    }
    let mut body = if !parts.is_empty() {
        parts.join(", ")
    } else if counts.used_calls > 0 {
        if counts.used_calls == 1 {
            match counts.used_names.as_slice() {
                [one] => format!("Used {one}"),
                _ => "Used 1 tool".into(),
            }
        } else {
            format!(
                "Used {} {}",
                counts.used_calls,
                tool_word(counts.used_calls)
            )
        }
    } else if counts.asks > 0 {
        "Asking questions".to_string()
    } else if counts.thinking > 0 {
        thought_header_body(None)
    } else {
        "Activity".to_string()
    };
    if let (Some(p), Some(m)) = (counts.diff_plus, counts.diff_minus) {
        body.push_str(&format!("  +{p} -{m}"));
    }
    body
}

pub fn format_duration(start: OffsetDateTime, end: OffsetDateTime) -> Option<String> {
    let secs = (end - start).whole_seconds();
    if secs < 0 {
        return None;
    }
    Some(format_elapsed_secs(secs as u64))
}

/// Whole-second elapsed label (`17s`, `2m`, `1h 3m`). `0` → `"0s"` for callers
/// that already decided to show a number; paint omits Thought duration at 0.
pub fn format_elapsed_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m {s}s")
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}m")
        }
    }
}

pub fn thought_header_body(duration: Option<&str>) -> String {
    match duration {
        Some(d) if !d.is_empty() => format!("Thought {d}"),
        _ => "Thought".to_string(),
    }
}

fn pretty_chord(raw: &str) -> String {
    raw.split('+')
        .map(|part| {
            if part.len() == 1 {
                part.to_ascii_uppercase()
            } else {
                let mut chars = part.chars();
                match chars.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Full-chord parenthetical for a product binding id (att27).
pub fn binding_chord_hint(binding_id: &'static str) -> String {
    crate::app::tui::keybindings::ensure_product_catalog();
    let raw: Option<String> = with_keybindings(|kb| kb.get_keys(binding_id).into_iter().next());
    match raw {
        Some(k) => format!("({})", pretty_chord(&k)),
        None => "(?)".into(),
    }
}

/// Envelope header (`Worked for`). Stays visible when expanded (▾) so it can fold again.
pub fn format_envelope_line(glyphs: GlyphSet, duration: Option<&str>, expanded: bool) -> String {
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let body = match duration {
        Some(d) => format!("Worked for {d}"),
        None => "Worked for".to_string(),
    };
    let hint = binding_chord_hint(if expanded {
        "app.activity.collapseNearest"
    } else {
        "app.activity.expandNearest"
    });
    format!("{marker} {body}  {hint}")
}

/// Paint one envelope summary row (marker + body + chord hint).
///
/// Cluster heads use [`format_cluster_header`]; this is envelope-only.
/// MUST NOT write `(Alt+E)`.
#[cfg(test)]
pub fn format_summary_line(
    level: SegmentLevel,
    glyphs: GlyphSet,
    _counts: &ActivityCounts,
    duration: Option<&str>,
) -> String {
    format_envelope_line(glyphs, duration, !matches!(level, SegmentLevel::L3))
}

/// Cluster header (L2/L0). `progressive` is the live open cluster.
/// `live_thinking`: current burst still streaming — show Thinking, not Thought Ns.
pub fn format_cluster_header(
    glyphs: GlyphSet,
    counts: &ActivityCounts,
    expanded: bool,
    progressive: bool,
    thought_dur: Option<&str>,
    live_thinking: bool,
) -> String {
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mut body = format_cluster_body(counts, progressive);
    if body == "Thought" {
        body = if live_thinking {
            "Thinking".to_string()
        } else {
            thought_header_body(thought_dur)
        };
    }
    let hint = binding_chord_hint(if expanded {
        "app.activity.collapseNearest"
    } else {
        "app.activity.expandNearest"
    });
    format!("{marker} {body}  {hint}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::UiEntry;

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

    fn thinking() -> UiEntry {
        UiEntry::Thinking {
            id: "t".into(),
            text: "hmm".into(),
            elapsed_secs: None,
        }
    }

    fn compaction() -> UiEntry {
        UiEntry::Compaction {
            status: crate::app::tui::bridge::CompactionBlockStatus::Complete,
            summary: "c".into(),
            tokens_before: 101_494,
            detail: None,
        }
    }

    #[test]
    fn diff_pm_counts_reliable_lines() {
        let diff = "@@ -1,2 +1,3 @@\n context\n-old\n+new\n+extra\n";
        assert_eq!(count_diff_pm(diff), Some((2, 1)));
    }

    #[test]
    fn explored_unique_paths_and_search_no_fake_file() {
        let entries = vec![
            tool("read", Some("a.rs")),
            tool("read", Some("a.rs")),
            tool("grep", None),
        ];
        let c = count_middles(&entries, &[0, 1, 2]);
        let s = format_l2_body(&c);
        assert!(s.contains("Explored a.rs"), "{s}");
        assert!(!s.contains("Explored 2"), "{s}");
        assert!(!s.contains("search"), "{s}");
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Exploring a.rs"), "{live}");
        assert!(!live.contains("Editing"), "{live}");
    }

    #[test]
    fn envelope_line_keeps_worked_for_when_expanded() {
        crate::app::tui::keybindings::ensure_product_catalog();
        let glyphs = GlyphSet::from_env();
        let collapsed = format_envelope_line(glyphs, Some("2m"), false);
        let expanded = format_envelope_line(glyphs, Some("2m"), true);
        let via_summary = format_summary_line(
            SegmentLevel::L2,
            glyphs,
            &ActivityCounts::default(),
            Some("2m"),
        );
        assert!(collapsed.contains("Worked for 2m"), "{collapsed}");
        assert!(expanded.contains("Worked for 2m"), "{expanded}");
        assert_eq!(expanded, via_summary);
        assert_ne!(collapsed, expanded);
    }

    #[test]
    fn sealed_edits_upgrade_not_explored() {
        let entries = vec![tool("edit", Some("b.rs")), tool("read", Some("a.rs"))];
        let c = count_middles(&entries, &[0, 1]);
        let s = format_l2_body(&c);
        assert!(s.contains("Edited b.rs"), "{s}");
        assert!(!s.to_ascii_lowercase().contains("explored"), "{s}");
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Editing b.rs"), "{live}");
        assert!(!live.contains("Edited"), "{live}");
        assert!(!live.contains("Explor"), "{live}");
    }

    #[test]
    fn thinking_only_is_thought_not_explored() {
        let entries = vec![thinking()];
        let c = count_middles(&entries, &[0]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Thought");
        assert!(!s.contains("Explored"));
        assert_eq!(thought_header_body(Some("17s")), "Thought 17s");
        assert_eq!(format_elapsed_secs(17), "17s");
        assert_eq!(format_elapsed_secs(0), "0s");
        crate::app::tui::keybindings::ensure_product_catalog();
        let glyphs = GlyphSet::from_env();
        let streaming = format_cluster_header(glyphs, &c, false, true, None, true);
        assert!(streaming.contains("Thinking"), "{streaming}");
        assert!(!streaming.contains("Thought"), "{streaming}");
        let flushed = format_cluster_header(glyphs, &c, false, false, Some("17s"), false);
        assert!(flushed.contains("Thought 17s"), "{flushed}");
        assert!(!flushed.contains("Thinking"), "{flushed}");
    }

    #[test]
    fn thinking_plus_todo_tools_is_used_not_thought() {
        let entries = vec![
            thinking(),
            tool("todo_list", None),
            tool("todo_update", None),
        ];
        let c = count_middles(&entries, &[0, 1, 2]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Used 2 tools");
        assert_eq!(c.used_calls, 2);
        assert!(c.thinking > 0, "thinking stays a kid, not the header");
        assert!(!c.is_thought_only());
    }

    #[test]
    fn four_same_unknown_tools_count_calls_not_unique_names() {
        let entries = vec![
            thinking(),
            tool("todo_update", None),
            tool("todo_update", None),
            tool("todo_update", None),
            tool("todo_update", None),
            UiEntry::Todo {
                summary: "Todo · 10/10".into(),
                detail_lines: vec![],
            },
        ];
        let c = count_middles(&entries, &[0, 1, 2, 3, 4, 5]);
        assert_eq!(format_l2_body(&c), "Used 4 tools");
        assert_eq!(c.used_calls, 4);
        assert_eq!(c.used_names.as_slice(), ["todo_update"]);
        assert!(!c.is_thought_only());
    }

    #[test]
    fn thinking_plus_ask_is_asking_not_thought() {
        let entries = vec![
            thinking(),
            UiEntry::Ask {
                id: "a1".into(),
                summary: "Ask · pick".into(),
                detail_lines: vec![],
                phase: crate::app::tui::bridge::AskPhase::Waiting,
                expanded: false,
            },
        ];
        let c = count_middles(&entries, &[0, 1]);
        assert_eq!(format_l2_body(&c), "Asking questions");
        assert!(!c.is_thought_only());
    }

    #[test]
    fn compaction_only_omits_header() {
        let entries = vec![compaction()];
        let c = count_middles(&entries, &[0]);
        assert!(c.omits_cluster_header());
        assert!(format_l2_body(&c).is_empty());
    }

    #[test]
    fn mcp_is_used_not_explored() {
        let entries = vec![tool("mcp:lspz:get_symbols", None)];
        let c = count_middles(&entries, &[0]);
        let s = format_l2_body(&c);
        assert!(s.contains("Used get_symbols"), "{s}");
        assert!(!s.contains("Explored"), "{s}");
    }

    #[test]
    fn bash_only_is_ran_not_explored() {
        let entries = vec![tool("bash", None)];
        let c = count_middles(&entries, &[0]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Ran 1 command");
        let live = format_cluster_body(&c, true);
        assert_eq!(live, "Running 1 command");
    }

    #[test]
    fn write_placeholder_path_is_not_dots() {
        let entries = vec![tool("write", Some("..."))];
        let c = count_middles(&entries, &[0]);
        let live = format_cluster_body(&c, true);
        assert_eq!(live, "Editing 1 file");
        assert!(!live.contains("..."), "{live}");
        let sealed = format_l2_body(&c);
        assert_eq!(sealed, "Edited 1 file");
    }

    #[test]
    fn cwd_dot_path_is_not_explored_dot() {
        let entries = vec![tool("ls", Some(".")), tool("bash", None)];
        let c = count_middles(&entries, &[0, 1]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Explored 1 file, Ran 1 command");
        assert!(!s.contains("Explored ."), "{s}");
        assert!(!s.contains("Explored ..."), "{s}");

        let abs = tool("ls", Some("/home/l8ng/Projects/__straydragon__/xylitol/."));
        let c = count_middles(&[abs], &[0]);
        assert_eq!(format_l2_body(&c), "Explored xylitol");

        let dotfile = tool("read", Some(".gitignore"));
        let c = count_middles(&[dotfile], &[0]);
        assert_eq!(format_l2_body(&c), "Explored .gitignore");
    }

    #[test]
    fn two_edits_unique_files() {
        let entries = vec![tool("edit", Some("a.rs")), tool("write", Some("b.rs"))];
        let c = count_middles(&entries, &[0, 1]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Edited 2 files");
    }

    #[test]
    fn l2_omits_pm_without_diff() {
        let c = ActivityCounts {
            explore_paths: vec!["a.rs".into(), "b.rs".into()],
            search_no_path: true,
            ..Default::default()
        };
        let s = format_l2_body(&c);
        assert!(s.contains("Explored 2 files"), "{s}");
        assert!(!s.contains('+'));
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Exploring 2 files"), "{live}");
        assert!(!live.contains("Explored"));
    }
}
