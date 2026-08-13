//! L2 count summary + L3 Worked-for lines (att24 / att27).

use time::OffsetDateTime;

use crate::app::tui::bridge::UiEntry;
use crate::app::tui::keybindings::with_keybindings;
use crate::app::tui::widgets::GlyphSet;

use super::segment::{ActivitySegment, SegmentLevel, middle_entry_indices};

/// Observable activity counters for an L2 line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    pub files: u32,
    pub searches: u32,
    pub commands: u32,
    /// Reliable +/- from Diff / edit display_diff only.
    pub diff_plus: Option<u32>,
    pub diff_minus: Option<u32>,
}

impl ActivityCounts {
    pub fn is_empty(&self) -> bool {
        self.files == 0 && self.searches == 0 && self.commands == 0
    }
}

pub fn count_segment(entries: &[UiEntry], seg: &ActivitySegment) -> ActivityCounts {
    let mut c = ActivityCounts::default();
    let mut plus = 0u32;
    let mut minus = 0u32;
    let mut saw_diff_stats = false;

    for idx in middle_entry_indices(entries, seg) {
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
                } else {
                    // read/write/edit/ls and unknown path-ish tools → files.
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
                c.files += 1;
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

    // Thinking/Ask-only segments: keep a non-empty L2 line.
    if c.is_empty() && !middle_entry_indices(entries, seg).is_empty() {
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

pub fn format_l2_body(counts: &ActivityCounts) -> String {
    let mut parts = Vec::new();
    if counts.files > 0 {
        parts.push(format!(
            "Explored {} {}",
            counts.files,
            if counts.files == 1 { "file" } else { "files" }
        ));
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

/// Paint one L2/L3 summary row (marker + body + chord hint).
///
/// Collapsed L2/L3 rows show the expand chord; MUST NOT write `(Alt+E)`.
pub fn format_summary_line(
    level: SegmentLevel,
    glyphs: GlyphSet,
    counts: &ActivityCounts,
    duration: Option<&str>,
) -> String {
    let marker = match level {
        SegmentLevel::L0 => glyphs.unfold(),
        SegmentLevel::L2 | SegmentLevel::L3 => glyphs.fold(),
    };
    let body = match level {
        SegmentLevel::L3 => match duration {
            Some(d) => format!("Worked for {d}"),
            None => "Worked for".to_string(),
        },
        SegmentLevel::L2 | SegmentLevel::L0 => format_l2_body(counts),
    };
    let hint = binding_chord_hint(match level {
        SegmentLevel::L2 | SegmentLevel::L3 => "app.activity.expandNearest",
        SegmentLevel::L0 => "app.activity.collapseNearest",
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
            searches: 1,
            commands: 0,
            diff_plus: None,
            diff_minus: None,
        };
        let s = format_l2_body(&c);
        assert!(s.contains("Explored 2 files"));
        assert!(s.contains("1 search"));
        assert!(!s.contains('+'));
    }
}
