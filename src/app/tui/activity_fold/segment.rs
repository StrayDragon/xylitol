//! Activity segment partition — stable ids for live ≡ rebuild (att23).

use std::ops::Range;

use crate::app::tui::bridge::UiEntry;

/// Segment ladder (no segment-level L1 — that is block fold / c2040).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentLevel {
    L0,
    L2,
    L3,
}

impl SegmentLevel {
    pub fn expand_one(self) -> Self {
        match self {
            Self::L3 => Self::L2,
            Self::L2 => Self::L0,
            Self::L0 => Self::L0,
        }
    }

    pub fn collapse_one(self, floor: Self) -> Self {
        let next = match self {
            Self::L0 => Self::L2,
            Self::L2 => Self::L3,
            Self::L3 => Self::L3,
        };
        // Clamp to floor coarseness (L3 coarsest); never below floor when collapsing.
        if next.rank() > floor.rank() {
            floor
        } else {
            next
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::L0 => 0,
            Self::L2 => 1,
            Self::L3 => 2,
        }
    }

    pub fn is_collapsed(self) -> bool {
        matches!(self, Self::L2 | Self::L3)
    }
}

/// One Activity segment: User + foldable middles + optional final Assistant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySegment {
    /// Stable id: `seg-{user_idx}` (entry index of the User row).
    pub id: String,
    pub user_idx: usize,
    /// Inclusive start .. exclusive end of Tool/Thinking/Diff/Ask/Bash rows.
    pub middle: Range<usize>,
    pub assistant_idx: Option<usize>,
    /// 0 = oldest activity segment in the transcript.
    pub turn_ordinal: usize,
}

impl ActivitySegment {
    pub fn has_activity(&self) -> bool {
        !self.middle.is_empty()
    }

    pub fn contains_entry(&self, idx: usize) -> bool {
        self.middle.contains(&idx)
    }
}

fn is_foldable_middle(entry: &UiEntry) -> bool {
    matches!(
        entry,
        UiEntry::Tool { .. }
            | UiEntry::Thinking { .. }
            | UiEntry::Diff { .. }
            | UiEntry::Ask { .. }
            | UiEntry::Bash { .. }
    )
}

/// Partition `entries` into Activity segments (att23).
///
/// ScrollNotice / Error / Compaction are never middles. A segment exists only when
/// the turn has at least one foldable middle op.
pub fn partition_segments(entries: &[UiEntry]) -> Vec<ActivitySegment> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut turn_ordinal = 0usize;
    while i < entries.len() {
        let UiEntry::User { .. } = &entries[i] else {
            i += 1;
            continue;
        };
        let user_idx = i;
        i += 1;
        let turn_end = entries[i..]
            .iter()
            .position(|e| matches!(e, UiEntry::User { .. }))
            .map(|p| i + p)
            .unwrap_or(entries.len());

        let mut middle_idxs = Vec::new();
        let mut last_assistant: Option<usize> = None;
        for (j, entry) in entries.iter().enumerate().take(turn_end).skip(i) {
            match entry {
                e if is_foldable_middle(e) => middle_idxs.push(j),
                UiEntry::Assistant { .. } => last_assistant = Some(j),
                _ => {}
            }
        }

        if !middle_idxs.is_empty() {
            // Contiguous range covering all middles (may include always-visible
            // rows in between — render still only skips foldable idxs).
            let start = *middle_idxs.first().expect("non-empty");
            let end = middle_idxs.last().expect("non-empty") + 1;
            out.push(ActivitySegment {
                id: format!("seg-{user_idx}"),
                user_idx,
                middle: start..end,
                assistant_idx: last_assistant,
                turn_ordinal,
            });
            turn_ordinal += 1;
        }
        i = turn_end;
    }
    out
}

/// Foldable entry indices belonging to `seg` (excludes Compaction etc. in range).
pub fn middle_entry_indices(entries: &[UiEntry], seg: &ActivitySegment) -> Vec<usize> {
    seg.middle
        .clone()
        .filter(|&idx| entries.get(idx).is_some_and(is_foldable_middle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::{BashBlockStatus, UiEntry};

    fn tool(id: &str) -> UiEntry {
        UiEntry::Tool {
            id: id.into(),
            name: "read".into(),
            args_preview: "a.rs".into(),
            tool_path: Some("a.rs".into()),
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: true,
        }
    }

    #[test]
    fn partition_skips_compaction_and_empty_turns() {
        let entries = vec![
            UiEntry::User { text: "u1".into() },
            tool("t1"),
            UiEntry::Assistant { text: "a1".into() },
            UiEntry::Compaction {
                status: crate::app::tui::bridge::CompactionBlockStatus::Complete,
                summary: "c".into(),
                tokens_before: 1,
                detail: None,
            },
            UiEntry::User { text: "u2".into() },
            UiEntry::Assistant { text: "a2".into() },
            UiEntry::ScrollNotice { text: "nav".into() },
        ];
        let segs = partition_segments(&entries);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].id, "seg-0");
        assert_eq!(segs[0].middle, 1..2);
        assert_eq!(segs[0].assistant_idx, Some(2));
    }

    #[test]
    fn partition_stable_ids_match_user_index() {
        let entries = vec![
            UiEntry::User { text: "a".into() },
            tool("t"),
            UiEntry::Assistant { text: "ok".into() },
            UiEntry::User { text: "b".into() },
            UiEntry::Bash {
                command: "ls".into(),
                status: BashBlockStatus::Success,
                output: String::new(),
                exclude_from_context: false,
            },
            UiEntry::Assistant { text: "ok2".into() },
        ];
        let segs = partition_segments(&entries);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].id, "seg-0");
        assert_eq!(segs[1].id, "seg-3");
    }
}
