use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use crate::bridge::UiEntry;

use super::degrade::{AutoTrigger, apply_auto_degrade, in_virgin_recent_window};
use super::segment::{SegmentLevel, partition_segments};
use super::settings::ActivityFoldSettings;
use super::summary::format_duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentRowSpans {
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
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,
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

#[derive(Debug, Clone, Default)]
pub struct ActivityFoldState {
    pub settings: ActivityFoldSettings,
    levels: HashMap<String, SegmentLevel>,
    entered: HashSet<String>,
    cluster_open: HashSet<String>,
    clocks: HashMap<String, SegmentClock>,
    pub row_spans: SegmentRowSpans,
}

impl ActivityFoldState {
    pub fn level_of(&self, id: &str) -> SegmentLevel {
        if !self.settings.enabled {
            return SegmentLevel::L0;
        }
        self.levels.get(id).copied().unwrap_or(SegmentLevel::L0)
    }

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
        if level == SegmentLevel::L3 {
            self.cluster_open
                .retain(|cid| !cid.starts_with(&format!("{id}:")));
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
        self.cluster_open
            .retain(|cid| live_ids.iter().any(|id| cid.starts_with(&format!("{id}:"))));
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

    pub fn toggle_one_step(&mut self, id: &str) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let cur = self.level_of(id);
        let next = match cur {
            SegmentLevel::L3 => SegmentLevel::L2,
            SegmentLevel::L2 | SegmentLevel::L0 => {
                self.mark_entered(id);
                self.settings.collapse_floor
            }
        };
        self.set_level(id, next)
    }

    pub fn expand_nearest(&mut self, entries: &[UiEntry]) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let segs = partition_segments(entries);
        if let Some(seg) = segs.iter().rev().find(|s| {
            self.level_of(&s.id).is_collapsed() && self.level_of(&s.id) == SegmentLevel::L3
        }) {
            let next = self.level_of(&seg.id).expand_one();
            return self.set_level(&seg.id, next);
        }
        for seg in segs.iter().rev() {
            if self.level_of(&seg.id) == SegmentLevel::L3 {
                continue;
            }
            for cl in seg.clusters.iter().rev() {
                if !self.cluster_is_expanded(&seg.id, &cl.id) {
                    return self.toggle_cluster(&cl.id);
                }
            }
        }
        false
    }

    pub fn collapse_nearest(&mut self, entries: &[UiEntry]) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let segs = partition_segments(entries);
        for seg in segs.iter().rev() {
            if self.level_of(&seg.id) == SegmentLevel::L3 {
                continue;
            }
            if in_virgin_recent_window(&self.settings, &segs, seg)
                && !self.entered.contains(&seg.id)
            {
                continue;
            }
            for cl in seg.clusters.iter().rev() {
                if self.cluster_is_expanded(&seg.id, &cl.id) {
                    return self.toggle_cluster(&cl.id);
                }
            }
            if self.level_of(&seg.id) == SegmentLevel::L2 {
                self.mark_entered(&seg.id);
                let next = SegmentLevel::L2.collapse_one(self.settings.collapse_floor);
                return self.set_level(&seg.id, next);
            }
        }
        false
    }

    pub fn cluster_is_expanded(&self, envelope_id: &str, cluster_id: &str) -> bool {
        match self.level_of(envelope_id) {
            SegmentLevel::L3 => false,
            SegmentLevel::L0 | SegmentLevel::L2 => self.cluster_open.contains(cluster_id),
        }
    }

    pub fn cluster_kids_visible(&self, envelope_id: &str, cluster_id: &str) -> bool {
        self.cluster_is_expanded(envelope_id, cluster_id)
    }

    pub fn toggle_cluster(&mut self, cluster_id: &str) -> bool {
        if !self.settings.enabled {
            return false;
        }
        let Some(env_id) = cluster_id.split(':').next() else {
            return false;
        };
        let env_id = env_id.to_string();
        if self.level_of(&env_id) == SegmentLevel::L3 {
            self.set_level(&env_id, SegmentLevel::L2);
            self.cluster_open.insert(cluster_id.to_string());
            self.mark_entered(&env_id);
            return true;
        }
        if self.cluster_open.contains(cluster_id) {
            self.cluster_open.remove(cluster_id);
        } else {
            self.cluster_open.insert(cluster_id.to_string());
        }
        self.mark_entered(&env_id);
        true
    }

    pub fn expand_live_cluster(&mut self, entries: &[UiEntry]) -> bool {
        let segs = partition_segments(entries);
        let Some(seg) = segs.last() else {
            return false;
        };
        if self.level_of(&seg.id) == SegmentLevel::L3 {
            self.set_level(&seg.id, SegmentLevel::L2);
        }
        let Some(cl) = seg.clusters.last() else {
            return false;
        };
        if self.cluster_open.contains(&cl.id) {
            return false;
        }
        self.toggle_cluster(&cl.id)
    }

    pub fn duration_for(&self, id: &str) -> Option<String> {
        self.clock_of(id).and_then(|c| c.duration_label())
    }

    pub fn force_level(&mut self, id: &str, level: SegmentLevel) {
        self.mark_entered(id);
        self.set_level(id, level);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity_fold::settings::ActivityFoldSettings;

    fn tool(id: &str) -> UiEntry {
        UiEntry::Tool {
            id: id.into(),
            name: "read".into(),
            args_preview: "a.rs".into(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: true,
            timeout_secs: None,
        }
    }

    fn turn(n: u8) -> Vec<UiEntry> {
        vec![
            UiEntry::User {
                text: format!("u{n}"),
            },
            tool(&format!("t{n}")),
            UiEntry::Assistant {
                text: format!("a{n}"),
            },
        ]
    }

    fn entries(turns: u8) -> Vec<UiEntry> {
        (1..=turns).flat_map(turn).collect()
    }

    fn state(keep_recent_turns: u32) -> ActivityFoldState {
        ActivityFoldState {
            settings: ActivityFoldSettings {
                keep_recent_turns,
                ..ActivityFoldSettings::default()
            },
            ..ActivityFoldState::default()
        }
    }

    #[test]
    fn collapse_nearest_is_noop_when_disabled() {
        let mut state = state(2);
        state.settings.enabled = false;
        assert!(!state.collapse_nearest(&entries(3)));
    }

    #[test]
    fn collapse_nearest_is_noop_without_segments() {
        let mut state = state(2);
        assert!(!state.collapse_nearest(&[UiEntry::Assistant { text: "a".into() }]));
    }

    #[test]
    fn collapse_nearest_steps_an_l2_segment_to_l3_then_stops() {
        let list = entries(3);
        let mut state = state(0);
        state.force_level("seg-0", SegmentLevel::L2);
        assert!(state.collapse_nearest(&list), "L2 collapses one step");
        assert_eq!(state.level_of("seg-0"), SegmentLevel::L3);
        assert!(
            !state.collapse_nearest(&list),
            "an L3 segment is already fully collapsed"
        );
    }

    #[test]
    fn collapse_nearest_leaves_unentered_l0_segments_alone() {
        let list = entries(2);
        let mut state = state(0);
        assert!(
            !state.collapse_nearest(&list),
            "an L0 segment with no expanded cluster has nothing to fold"
        );
        assert_eq!(state.level_of("seg-0"), SegmentLevel::L0);
        assert_eq!(state.level_of("seg-1"), SegmentLevel::L0);
    }

    #[test]
    fn collapse_nearest_collapses_an_open_cluster_first() {
        let list = entries(3);
        let mut state = state(0);
        state.force_level("seg-0", SegmentLevel::L2);
        assert!(state.toggle_cluster("seg-0:c0"), "cluster starts expanded");
        assert!(state.cluster_is_expanded("seg-0", "seg-0:c0"));
        assert!(
            state.collapse_nearest(&list),
            "open cluster collapses first"
        );
        assert!(!state.cluster_is_expanded("seg-0", "seg-0:c0"));
        assert_eq!(
            state.level_of("seg-0"),
            SegmentLevel::L2,
            "segment level is untouched while a cluster folds"
        );
    }

    #[test]
    fn collapse_nearest_skips_virgin_recent_turns() {
        let list = entries(3);
        let mut state = state(2);
        assert!(
            !state.collapse_nearest(&list),
            "the newest turns inside keep_recent_turns stay untouched"
        );
        assert_eq!(state.level_of("seg-0"), SegmentLevel::L0);
    }
}
