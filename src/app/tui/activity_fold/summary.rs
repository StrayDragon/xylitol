//! Cluster / envelope summary lines (att24 / att27).

use time::OffsetDateTime;

use crate::app::tui::bridge::{UiEntry, UiModel};
use crate::app::tui::keybindings::with_keybindings;
use crate::app::tui::widgets::GlyphSet;

use super::atom::{ActivityAtom, ExploreKind, STREAMING_THINK_ID, activity_atom};
#[cfg(test)]
use super::segment::SegmentLevel;
use super::segment::{ActivityCluster, cluster_middle_indices};

/// Observable activity for one cluster header (att24).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    /// Unique edit/write paths (first-seen order). Pathless edits use a placeholder.
    pub edit_paths: Vec<String>,
    /// Unique read/ls/search paths.
    pub explore_paths: Vec<String>,
    /// Search ran with no usable path — still qualifies Explored, no fake N.
    pub search_no_path: bool,
    /// Explore invocations by category (c2510/att35 head suffix): invocation
    /// count, not the deduped-path count of `explore_paths`. Pathless
    /// searches count here too.
    pub read_calls: u32,
    pub search_calls: u32,
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
    /// In-flight thinking burst id ([`STREAMING_THINK_ID`]), not a global stream flag.
    pub live_think_id: Option<String>,
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

    /// Merge an in-flight thinking burst into this cluster only.
    ///
    /// Callers MUST pass this only for the open live cluster. Identity is
    /// [`STREAMING_THINK_ID`], not `UiModel.streaming_thinking`.
    pub fn with_live_think(mut self, id: impl Into<String>) -> Self {
        if self.thinking == 0 {
            self.thinking = 1;
        }
        self.live_think_id = Some(id.into());
        self
    }
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

/// Which live thinking burst this cluster owns, if any.
///
/// Keys off [`UiModel::streaming_think_id`], never `streaming_thinking` emptiness.
/// Unflushed bursts use [`STREAMING_THINK_ID`] and only attach to the open live cluster.
pub fn live_think_id_for_cluster(
    model: &UiModel,
    cluster: &ActivityCluster,
    progressive: bool,
) -> Option<String> {
    let live_id = model.streaming_think_id.as_deref()?;
    if progressive && live_id == STREAMING_THINK_ID {
        return Some(live_id.to_string());
    }
    for idx in cluster_middle_indices(&model.entries, cluster) {
        if let Some(UiEntry::Thinking { id, .. }) = model.entries.get(idx)
            && id == live_id
        {
            return Some(live_id.to_string());
        }
    }
    None
}

/// Counts for a live thinking stream before it is flushed to a Thinking entry.
pub fn streaming_thought_counts() -> ActivityCounts {
    counts_from_atoms(std::iter::once(ActivityAtom::streaming_think()))
}

fn push_unique(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|p| p == &path) {
        paths.push(path);
    }
}

fn count_middles(entries: &[UiEntry], indices: &[usize]) -> ActivityCounts {
    counts_from_atoms(indices.iter().map(|&idx| activity_atom(&entries[idx])))
}

fn counts_from_atoms(atoms: impl IntoIterator<Item = ActivityAtom>) -> ActivityCounts {
    let mut c = ActivityCounts::default();
    let mut plus = 0u32;
    let mut minus = 0u32;
    let mut saw_diff_stats = false;
    let mut anon_edits = 0u32;
    let mut anon_explores = 0u32;

    for atom in atoms {
        match atom {
            ActivityAtom::Edit { path, diff_pm } => {
                match path {
                    Some(p) => push_unique(&mut c.edit_paths, p),
                    None => anon_edits += 1,
                }
                if let Some((p, m)) = diff_pm {
                    plus = plus.saturating_add(p);
                    minus = minus.saturating_add(m);
                    saw_diff_stats = true;
                }
            }
            ActivityAtom::Explore { path, kind } => {
                match kind {
                    ExploreKind::File => c.read_calls = c.read_calls.saturating_add(1),
                    ExploreKind::Search => c.search_calls = c.search_calls.saturating_add(1),
                }
                match (path, kind) {
                    (Some(p), _) => push_unique(&mut c.explore_paths, p),
                    (None, ExploreKind::Search) => c.search_no_path = true,
                    (None, ExploreKind::File) => anon_explores += 1,
                }
            }
            ActivityAtom::Run => c.commands += 1,
            ActivityAtom::Used { display_name } => {
                c.used_calls = c.used_calls.saturating_add(1);
                push_unique(&mut c.used_names, display_name);
            }
            ActivityAtom::Think { id } => {
                c.thinking += 1;
                if id == STREAMING_THINK_ID {
                    c.live_think_id = Some(id);
                }
            }
            ActivityAtom::Ask { .. } => c.asks += 1,
            ActivityAtom::Compaction => c.compaction += 1,
            ActivityAtom::Noise | ActivityAtom::Projection => {}
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

/// Shared count pluralization (files / tools / commands / reads / searches).
fn plural_word<'a>(n: u32, single: &'a str, many: &'a str) -> &'a str {
    if n == 1 { single } else { many }
}

fn files_word(n: u32) -> &'static str {
    plural_word(n, "file", "files")
}

fn tool_word(n: u32) -> &'static str {
    plural_word(n, "tool", "tools")
}

fn command_word(n: u32) -> &'static str {
    plural_word(n, "command", "commands")
}

/// `· 5 reads · 2 searches` body (without the leading separator); empty when
/// the cluster has no explore invocations.
fn explore_calls_clause(counts: &ActivityCounts) -> String {
    let mut parts = Vec::new();
    if counts.read_calls > 0 {
        parts.push(format!(
            "{} {}",
            counts.read_calls,
            plural_word(counts.read_calls, "read", "reads")
        ));
    }
    if counts.search_calls > 0 {
        parts.push(format!(
            "{} {}",
            counts.search_calls,
            plural_word(counts.search_calls, "search", "searches")
        ));
    }
    parts.join(" · ")
}

fn file_clause(verb: &str, paths: &[String]) -> String {
    // c2540: count-only — basenames echoed user content on a metrics row;
    // concrete files live in the cluster's expandable children.
    let n = paths.len() as u32;
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
    } else if !counts.explore_paths.is_empty() || counts.search_no_path {
        let explore_verb = if progressive { "Exploring" } else { "Explored" };
        let mut clause = if !counts.explore_paths.is_empty() {
            file_clause(explore_verb, &counts.explore_paths)
        } else {
            explore_verb.to_string()
        };
        // c2510/att35: invocation-category counts after the file clause —
        // real calls, not another deduped-path figure.
        let calls = explore_calls_clause(counts);
        if !calls.is_empty() {
            clause.push_str(" · ");
            clause.push_str(&calls);
        }
        parts.push(clause);
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
/// Live Thinking vs Thought is bound to [`ActivityCounts::live_think_id`].
pub fn format_cluster_header(
    glyphs: GlyphSet,
    counts: &ActivityCounts,
    expanded: bool,
    progressive: bool,
    thought_dur: Option<&str>,
) -> String {
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mut body = format_cluster_body(counts, progressive);
    if body == "Thought" {
        body = if counts.live_think_id.is_some() {
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
    use super::super::atom::count_diff_pm;
    use super::*;
    use crate::app::tui::bridge::UiEntry;

    fn tool(name: &str, path: Option<&str>) -> UiEntry {
        UiEntry::Tool {
            timeout_secs: None,
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

            tokens_after: None,
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
        assert!(s.contains("Explored 1 file · 2 reads · 1 search"), "{s}");
        assert!(!s.contains("Explored 2"), "{s}");
        // c2510/att35: invocation counts are real, no fake file figure.
        assert!(s.contains("· 2 reads · 1 search"), "{s}");
        let live = format_cluster_body(&c, true);
        assert!(
            live.contains("Exploring 1 file · 2 reads · 1 search"),
            "{live}"
        );
        assert!(!live.contains("Editing"), "{live}");
    }

    #[test]
    fn explore_head_suffix_single_category_and_counts() {
        // three reads, one pathless search on top → both categories listed
        let entries = vec![
            tool("read", Some("a.rs")),
            tool("read", Some("b.rs")),
            tool("read", Some("c.rs")),
            tool("grep", None),
            tool("grep", None),
        ];
        let c = count_middles(&entries, &[0, 1, 2, 3, 4]);
        let s = format_l2_body(&c);
        assert!(s.contains("Explored 3 files · 3 reads · 2 searches"), "{s}");

        // single category → only that category listed, singular form kept
        let reads_only = vec![tool("read", Some("a.rs"))];
        let c = count_middles(&reads_only, &[0]);
        let s = format_l2_body(&c);
        assert!(s.contains("Explored 1 file · 1 read"), "{s}");
        assert!(!s.contains("search"), "{s}");

        // pathless searches only → bare verb carries the search count
        let searches_only = vec![tool("grep", None), tool("grep", None)];
        let c = count_middles(&searches_only, &[0, 1]);
        let s = format_l2_body(&c);
        assert_eq!(s, "Explored · 2 searches");

        // Edited head takes no explore suffix (att24 file-layer mutex)
        let mixed = vec![tool("read", Some("a.rs")), tool("edit", Some("b.rs"))];
        let c = count_middles(&mixed, &[0, 1]);
        let s = format_l2_body(&c);
        assert!(s.starts_with("Edited"), "{s}");
        assert!(!s.contains("reads"), "{s}");
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
        assert!(s.contains("Edited 1 file"), "{s}");
        assert!(!s.to_ascii_lowercase().contains("explored"), "{s}");
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Editing 1 file"), "{live}");
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
        let streaming =
            format_cluster_header(glyphs, &streaming_thought_counts(), false, true, None);
        assert!(streaming.contains("Thinking"), "{streaming}");
        assert!(!streaming.contains("Thought"), "{streaming}");
        let flushed = format_cluster_header(glyphs, &c, false, false, Some("17s"));
        assert!(flushed.contains("Thought 17s"), "{flushed}");
        assert!(!flushed.contains("Thinking"), "{flushed}");
        let live_on_flushed = format_cluster_header(
            glyphs,
            &c.clone().with_live_think(STREAMING_THINK_ID),
            false,
            true,
            None,
        );
        assert!(live_on_flushed.contains("Thinking"), "{live_on_flushed}");
        assert!(!live_on_flushed.contains("Thought"), "{live_on_flushed}");
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
        let entries = vec![tool("mcp__lspz__get_symbols", None)];
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
    fn bang_bash_entry_is_noise_tool_bash_is_ran() {
        let bang = UiEntry::Bash {
            id: "bash-t".into(),
            command: "ls".into(),
            status: crate::app::tui::bridge::BashBlockStatus::Success,
            output: String::new(),
            exclude_from_context: false,
        };
        let c = count_middles(&[bang], &[0]);
        assert_eq!(c.commands, 0);
        assert_eq!(format_l2_body(&c), "Activity");

        let c = count_middles(&[tool("bash", None)], &[0]);
        assert_eq!(format_l2_body(&c), "Ran 1 command");
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
        assert_eq!(s, "Explored 1 file · 1 read, Ran 1 command");
        assert!(!s.contains("Explored ."), "{s}");
        assert!(!s.contains("Explored ..."), "{s}");

        let abs = tool("ls", Some("/home/l8ng/Projects/__straydragon__/xylitol/."));
        let c = count_middles(&[abs], &[0]);
        assert_eq!(format_l2_body(&c), "Explored 1 file · 1 read");

        let dotfile = tool("read", Some(".gitignore"));
        let c = count_middles(&[dotfile], &[0]);
        assert_eq!(format_l2_body(&c), "Explored 1 file · 1 read");
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

    #[test]
    fn unknown_searchish_tool_is_used_not_explored() {
        let entries = vec![tool("my_custom_search", None)];
        let c = count_middles(&entries, &[0]);
        assert_eq!(format_l2_body(&c), "Used my_custom_search");
        assert_eq!(c.used_calls, 1);
        assert!(c.explore_paths.is_empty());
        assert!(!c.search_no_path);
    }

    #[test]
    fn live_think_id_ignores_text_buffer_without_id() {
        use super::super::segment::partition_segments;
        let mut model = UiModel::new();
        model.entries = vec![
            UiEntry::User { text: "hi".into() },
            UiEntry::Thinking {
                id: "th-old".into(),
                text: "first".into(),
                elapsed_secs: Some(17),
            },
            UiEntry::Assistant { text: "mid".into() },
        ];
        model.streaming_thinking = "orphan".into();
        let segs = partition_segments(&model.entries);
        let cl0 = &segs[0].clusters[0];
        assert!(live_think_id_for_cluster(&model, cl0, true).is_none());

        model.streaming_think_id = Some(STREAMING_THINK_ID.into());
        assert!(live_think_id_for_cluster(&model, cl0, false).is_none());
        assert_eq!(
            live_think_id_for_cluster(&model, cl0, true).as_deref(),
            Some(STREAMING_THINK_ID)
        );
    }
}
