//! Activity segment partition — stable ids for live ≡ rebuild (att23).

use std::ops::Range;

use crate::app::tui::bridge::UiEntry;

use super::atom::activity_atom;

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

    pub(crate) fn rank_public(self) -> u8 {
        self.rank()
    }
}

/// One cluster inside an envelope (split by displayable assistant body, att34).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityCluster {
    /// `seg-{user_idx}:c{ord}`
    pub id: String,
    pub middle: Range<usize>,
    /// Assistant row that sealed this cluster, if any.
    pub seal_assistant_idx: Option<usize>,
}

/// One Activity envelope: User + foldable middles (possibly several clusters) + last Assistant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySegment {
    /// Stable id: `seg-{user_idx}` (entry index of the User row).
    pub id: String,
    pub user_idx: usize,
    /// Inclusive start .. exclusive end covering all cluster middles.
    pub middle: Range<usize>,
    pub clusters: Vec<ActivityCluster>,
    pub assistant_idx: Option<usize>,
    /// Assistant rows in this turn that are not the last one (hidden when envelope collapsed).
    pub mid_assistant_idxs: Vec<usize>,
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
    activity_atom(entry).is_foldable_middle()
}

fn assistant_displayable(text: &str) -> bool {
    text.chars().any(|c| !c.is_whitespace())
}

/// Partition `entries` into Activity envelopes (att23 / att34).
///
/// ScrollNotice / Error / bang Bash are never middles. Compaction / Todo are middles (in envelope).
/// Cluster boundaries = displayable assistant body (thinking does not split).
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

        let mut clusters = Vec::new();
        let mut open_mids: Vec<usize> = Vec::new();
        let mut assistants: Vec<usize> = Vec::new();
        let mut cluster_ord = 0usize;

        let seal_open = |clusters: &mut Vec<ActivityCluster>,
                         open_mids: &mut Vec<usize>,
                         cluster_ord: &mut usize,
                         seal: Option<usize>| {
            if open_mids.is_empty() {
                return;
            }
            let start = *open_mids.first().expect("non-empty");
            let end = *open_mids.last().expect("non-empty") + 1;
            clusters.push(ActivityCluster {
                id: format!("seg-{user_idx}:c{cluster_ord}"),
                middle: start..end,
                seal_assistant_idx: seal,
            });
            *cluster_ord += 1;
            open_mids.clear();
        };

        for (j, entry) in entries.iter().enumerate().take(turn_end).skip(i) {
            match entry {
                e if is_foldable_middle(e) => open_mids.push(j),
                UiEntry::Assistant { text } if assistant_displayable(text) => {
                    assistants.push(j);
                    seal_open(&mut clusters, &mut open_mids, &mut cluster_ord, Some(j));
                }
                _ => {}
            }
        }
        seal_open(&mut clusters, &mut open_mids, &mut cluster_ord, None);

        if !clusters.is_empty() {
            let start = clusters.first().expect("non-empty").middle.start;
            let end = clusters.last().expect("non-empty").middle.end;
            let last_asst = assistants.last().copied();
            let mid_assistant_idxs = if assistants.len() > 1 {
                assistants[..assistants.len() - 1].to_vec()
            } else {
                Vec::new()
            };
            out.push(ActivitySegment {
                id: format!("seg-{user_idx}"),
                user_idx,
                middle: start..end,
                clusters,
                assistant_idx: last_asst,
                mid_assistant_idxs,
                turn_ordinal,
            });
            turn_ordinal += 1;
        }
        i = turn_end;
    }
    out
}

/// Foldable entry indices belonging to `seg` (includes Compaction / Todo).
pub fn middle_entry_indices(entries: &[UiEntry], seg: &ActivitySegment) -> Vec<usize> {
    seg.middle
        .clone()
        .filter(|&idx| entries.get(idx).is_some_and(is_foldable_middle))
        .collect()
}

/// Foldable entry indices belonging to one cluster.
pub fn cluster_middle_indices(entries: &[UiEntry], cluster: &ActivityCluster) -> Vec<usize> {
    cluster
        .middle
        .clone()
        .filter(|&idx| entries.get(idx).is_some_and(is_foldable_middle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::UiEntry;

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
    fn partition_includes_compaction_skips_empty_and_scrollnotice() {
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
        assert_eq!(segs[0].clusters.len(), 2);
        assert_eq!(segs[0].clusters[0].middle, 1..2);
        assert_eq!(segs[0].assistant_idx, Some(2));
        assert!(
            middle_entry_indices(&entries, &segs[0]).contains(&3),
            "compaction is an envelope middle"
        );
    }

    #[test]
    fn partition_splits_clusters_on_displayable_assistant() {
        let entries = vec![
            UiEntry::User { text: "u".into() },
            tool("t1"),
            UiEntry::Assistant { text: "mid".into() },
            tool("t2"),
            UiEntry::Assistant {
                text: "last".into(),
            },
        ];
        let segs = partition_segments(&entries);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].clusters.len(), 2);
        assert_eq!(segs[0].clusters[0].id, "seg-0:c0");
        assert_eq!(segs[0].clusters[1].id, "seg-0:c1");
        assert_eq!(segs[0].mid_assistant_idxs, vec![2]);
        assert_eq!(segs[0].assistant_idx, Some(4));
    }

    #[test]
    fn whitespace_assistant_does_not_split() {
        let entries = vec![
            UiEntry::User { text: "u".into() },
            tool("t1"),
            UiEntry::Assistant {
                text: "  \n".into(),
            },
            tool("t2"),
            UiEntry::Assistant { text: "ok".into() },
        ];
        let segs = partition_segments(&entries);
        assert_eq!(segs[0].clusters.len(), 1);
        assert_eq!(segs[0].clusters[0].middle, 1..4);
    }

    #[test]
    fn partition_stable_ids_match_user_index() {
        let entries = vec![
            UiEntry::User { text: "a".into() },
            tool("t"),
            UiEntry::Assistant { text: "ok".into() },
            UiEntry::User { text: "b".into() },
            tool("t2"),
            UiEntry::Assistant { text: "ok2".into() },
        ];
        let segs = partition_segments(&entries);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].id, "seg-0");
        assert_eq!(segs[1].id, "seg-3");
    }
}
