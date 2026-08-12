//! Segment level map + nearest expand/collapse (att23 / att28).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::app::tui::bridge::UiEntry;

use super::degrade::{AutoTrigger, apply_auto_degrade, in_virgin_recent_window};
use super::segment::{ActivitySegment, SegmentLevel, middle_entry_indices, partition_segments};
use super::settings::ActivityFoldSettings;
use super::summary::{count_segment, format_duration};

/// Reserved hit seam: segment summary → content-relative half-open row range.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentRowSpans {
    /// `segment_id → [line_start, line_end)` in scrollback content rows.
    pub spans: HashMap<String, (usize, usize)>,
}

impl SegmentRowSpans {
    pub fn clear(&mut self) {
        self.spans.clear();
    }

    pub fn insert(&mut self, id: impl Into<String>, start: usize, end: usize) {
        self.spans.insert(id.into(), (start, end));
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentClock {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl SegmentClock {
    pub fn duration_label(&self) -> Option<String> {
        match (self.start, self.end) {
            (Some(s), Some(e)) => format_duration(s, e),
            _ => None,
        }
    }

    pub fn has_both_ends(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }
}

/// UI-root activity fold plane (orthogonal to [`crate::app::tui::widgets::ScrollbackFold`]).
#[derive(Debug, Clone, Default)]
pub struct ActivityFoldState {
    pub settings: ActivityFoldSettings,
    levels: HashMap<String, SegmentLevel>,
    /// Segments that have been auto-folded or user-collapsed at least once.
    entered: HashSet<String>,
    clocks: HashMap<String, SegmentClock>,
    /// Paint-time reservation (not FoldHitTable).
    pub row_spans: SegmentRowSpans,
}

impl ActivityFoldState {
    pub fn level_of(&self, id: &str) -> SegmentLevel {
        if !self.settings.enabled {
            return SegmentLevel::L0;
        }
        self.levels.get(id).copied().unwrap_or(SegmentLevel::L0)
    }

    /// Effective level for paint (respects master switch).
    pub fn effective_level(&self, id: &str) -> SegmentLevel {
        self.level_of(id)
    }

    pub fn set_level(&mut self, id: &str, level: SegmentLevel) -> bool {
        let prev = self.level_of(id);
        if prev == level && self.levels.contains_key(id) {
            return false;
        }
        if level == SegmentLevel::L0 {
            self.levels.remove(id);
        } else {
            self.levels.insert(id.to_string(), level);
        }
        prev != level
    }

    pub fn mark_entered(&mut self, id: &str) {
        self.entered.insert(id.to_string());
    }

    pub fn set_clock(&mut self, id: &str, clock: SegmentClock) {
        self.clocks.insert(id.to_string(), clock);
    }

    pub fn clock_of(&self, id: &str) -> Option<&SegmentClock> {
        self.clocks.get(id)
    }

    pub fn clear_orphans(&mut self, live_ids: &HashSet<String>) {
        self.levels.retain(|k, _| live_ids.contains(k));
        self.entered.retain(|k| live_ids.contains(k));
        self.clocks.retain(|k, _| live_ids.contains(k));
    }

    pub fn sync_segment_ids(&mut self, entries: &[UiEntry]) {
        let segs = partition_segments(entries);
        let ids: HashSet<String> = segs.iter().map(|s| s.id.clone()).collect();
        self.clear_orphans(&ids);
    }

    pub fn auto_degrade(
        &mut self,
        entries: &[UiEntry],
        trigger: AutoTrigger,
        protect_newest: bool,
    ) -> bool {
        self.sync_segment_ids(entries);
        apply_auto_degrade(self, entries, trigger, protect_newest)
    }

    /// One-step toggle for a specific segment (att31 / c2045 Wave B).
    ///
    /// Collapsed (L2/L3) → `expand_one`; L0 → `collapse_one(floor)`.
    /// Does **not** use nearest heuristics.
    pub fn toggle_one_step(&mut self, id: &str) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let cur = self.level_of(id);
        let next = if cur.is_collapsed() {
            cur.expand_one()
        } else {
            self.mark_entered(id);
            cur.collapse_one(self.settings.collapse_floor())
        };
        self.set_level(id, next)
    }

    /// Expand nearest L2/L3 toward L0. Silent if none. Returns whether state changed.
    pub fn expand_nearest(&mut self, entries: &[UiEntry]) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let segs = partition_segments(entries);
        let Some(seg) = segs
            .iter()
            .rev()
            .find(|s| self.level_of(&s.id).is_collapsed())
        else {
            return false;
        };
        let next = self.level_of(&seg.id).expand_one();
        self.set_level(&seg.id, next)
    }

    /// Collapse nearest L0 Activity toward floor. Near-window virgin L0 is not a target.
    pub fn collapse_nearest(&mut self, entries: &[UiEntry]) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let segs = partition_segments(entries);
        let floor = self.settings.collapse_floor();
        let Some(seg) = segs.iter().rev().find(|s| {
            if self.level_of(&s.id) != SegmentLevel::L0 || !s.has_activity() {
                return false;
            }
            // Near-window never-entered → not a target (att26 / att28).
            if in_virgin_recent_window(&self.settings, &segs, s) && !self.entered.contains(&s.id) {
                return false;
            }
            true
        }) else {
            return false;
        };
        self.mark_entered(&seg.id);
        let next = SegmentLevel::L0.collapse_one(floor);
        self.set_level(&seg.id, next)
    }

    /// Lookup collapsed segment covering a middle entry index.
    pub fn collapsed_segment_for_entry<'a>(
        &self,
        entries: &[UiEntry],
        segments: &'a [ActivitySegment],
        entry_idx: usize,
    ) -> Option<&'a ActivitySegment> {
        let seg = segments
            .iter()
            .find(|s| middle_entry_indices(entries, s).contains(&entry_idx))?;
        if self.effective_level(&seg.id).is_collapsed() {
            Some(seg)
        } else {
            None
        }
    }

    pub fn duration_for(&self, id: &str) -> Option<String> {
        self.clock_of(id).and_then(|c| c.duration_label())
    }

    /// Debug/test: force a level and mark entered.
    pub fn force_level(&mut self, id: &str, level: SegmentLevel) {
        self.mark_entered(id);
        self.set_level(id, level);
    }

    pub fn counts_for(
        &self,
        entries: &[UiEntry],
        seg: &ActivitySegment,
    ) -> super::summary::ActivityCounts {
        count_segment(entries, seg)
    }
}
