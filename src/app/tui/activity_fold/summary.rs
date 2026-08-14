//! L2 count summary + L3 Worked-for lines (att24 / att27).

use time::OffsetDateTime;

use crate::app::tui::bridge::UiEntry;
use crate::app::tui::keybindings::with_keybindings;
use crate::app::tui::widgets::GlyphSet;

#[cfg(test)]
use super::segment::SegmentLevel;
use super::segment::{
    ActivityCluster, ActivitySegment, cluster_middle_indices, middle_entry_indices,
};

/// Observable activity counters for an L2 line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    /// Read / explore file tools (sealed header: Explored).
    pub files: u32,
    /// Edit / write / Diff (sealed header: Edited; progressive still folds into Editing).
    pub edits: u32,
    pub searches: u32,
    pub commands: u32,
    /// Reliable +/- from Diff / edit display_diff only.
    pub diff_plus: Option<u32>,
    pub diff_minus: Option<u32>,
}

impl ActivityCounts {
    pub fn is_empty(&self) -> bool {
        self.files == 0 && self.edits == 0 && self.searches == 0 && self.commands == 0
    }
}

pub fn count_segment(entries: &[UiEntry], seg: &ActivitySegment) -> ActivityCounts {
    count_middles(entries, &middle_entry_indices(entries, seg))
}

/// Counts for one cluster only (open-cluster -2 vs frozen -3).
pub fn count_cluster(entries: &[UiEntry], cluster: &ActivityCluster) -> ActivityCounts {
    count_middles(entries, &cluster_middle_indices(entries, cluster))
}

fn count_middles(entries: &[UiEntry], indices: &[usize]) -> ActivityCounts {
    let mut c = ActivityCounts::default();
    let mut plus = 0u32;
    let mut minus = 0u32;
    let mut saw_diff_stats = false;

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
                    c.searches += 1;
                } else if is_command_tool(&n) {
                    c.commands += 1;
                } else if is_edit_tool(&n) {
                    c.edits += 1;
                } else {
                    // read/ls and unknown path-ish tools → explored files.
                    let _ = (tool_path, args_preview);
                    c.files += 1;
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
                c.edits += 1;
                if let Some((p, m)) = count_diff_pm(display_diff) {
                    plus = plus.saturating_add(p);
                    minus = minus.saturating_add(m);
                    saw_diff_stats = true;
                }
            }
            UiEntry::Bash { .. } => c.commands += 1,
            UiEntry::Thinking { .. } | UiEntry::Ask { .. } => {}
            _ => {}
        }
    }

    // Thinking/Ask-only clusters: keep a non-empty header.
    if c.is_empty() && !indices.is_empty() {
        c.files = 1;
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

#[cfg(test)]
pub fn format_l2_body(counts: &ActivityCounts) -> String {
    format_cluster_body(counts, false)
}

/// Open-cluster progressive (`Editing`) vs sealed (`Explored` / `Edited`).
pub fn format_cluster_body(counts: &ActivityCounts, progressive: bool) -> String {
    let mut parts = Vec::new();
    let file_total = counts.edits.saturating_add(counts.files);
    if progressive {
        if file_total > 0 {
            parts.push(format!("Editing {file_total} {}", files_word(file_total)));
        }
    } else {
        if counts.edits > 0 {
            parts.push(format!(
                "Edited {} {}",
                counts.edits,
                files_word(counts.edits)
            ));
        }
        if counts.files > 0 {
            let n = format!("{} {}", counts.files, files_word(counts.files));
            if parts.is_empty() {
                parts.push(format!("Explored {n}"));
            } else {
                parts.push(format!("explored {n}"));
            }
        }
    }
    if counts.searches > 0 {
        parts.push(format!(
            "{} {}",
            counts.searches,
            if counts.searches == 1 {
                "search"
            } else {
                "searches"
            }
        ));
    }
    if counts.commands > 0 {
        parts.push(format!(
            "ran {} {}",
            counts.commands,
            if counts.commands == 1 {
                "command"
            } else {
                "commands"
            }
        ));
    }
    let mut body = if parts.is_empty() {
        "Activity".to_string()
    } else {
        parts.join(", ")
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
    let secs = secs as u64;
    if secs < 60 {
        Some(format!("{secs}s"))
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            Some(format!("{m}m"))
        } else {
            Some(format!("{m}m {s}s"))
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 {
            Some(format!("{h}h"))
        } else {
            Some(format!("{h}h {m}m"))
        }
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

/// Cluster header (L2/L0). `progressive` is the live open cluster (`Editing`).
pub fn format_cluster_header(
    glyphs: GlyphSet,
    counts: &ActivityCounts,
    expanded: bool,
    progressive: bool,
) -> String {
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let body = format_cluster_body(counts, progressive);
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

    #[test]
    fn diff_pm_counts_reliable_lines() {
        let diff = "@@ -1,2 +1,3 @@\n context\n-old\n+new\n+extra\n";
        assert_eq!(count_diff_pm(diff), Some((2, 1)));
    }

    #[test]
    fn l2_omits_pm_without_diff() {
        let c = ActivityCounts {
            files: 2,
            edits: 0,
            searches: 1,
            commands: 0,
            diff_plus: None,
            diff_minus: None,
        };
        let s = format_l2_body(&c);
        assert!(s.contains("Explored 2 files"));
        assert!(s.contains("1 search"));
        assert!(!s.contains('+'));
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Editing 2 files"));
        assert!(!live.contains("Explored"));
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
    fn sealed_edits_use_edited_not_explored() {
        let c = ActivityCounts {
            files: 1,
            edits: 2,
            searches: 0,
            commands: 0,
            diff_plus: None,
            diff_minus: None,
        };
        let s = format_l2_body(&c);
        assert!(s.contains("Edited 2 files"), "{s}");
        assert!(s.contains("explored 1 file"), "{s}");
        assert!(!s.contains("Explored 2"), "{s}");
        let live = format_cluster_body(&c, true);
        assert!(live.contains("Editing 3 files"), "{live}");
        assert!(!live.contains("Edited"), "{live}");
    }
}
