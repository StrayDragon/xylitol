//! Auto-degrade helpers (att26).

use crate::app::tui::bridge::UiEntry;

use super::segment::{ActivitySegment, SegmentLevel, partition_segments};
use super::settings::ActivityFoldSettings;
use super::state::ActivityFoldState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoTrigger {
    Rebuild,
    TurnEnd,
}

/// Apply keep_recent_turns / auto_l3_distant crush. Returns whether any level changed.
///
/// When `protect_newest` is true (streaming busy), the newest activity segment is
/// forced to stay L0 for this pass.
pub fn apply_auto_degrade(
    state: &mut ActivityFoldState,
    entries: &[UiEntry],
    trigger: AutoTrigger,
    protect_newest: bool,
) -> bool {
    if !state.settings.enabled {
        return false;
    }
    let enabled = match trigger {
        AutoTrigger::Rebuild => state.settings.auto_on_rebuild,
        AutoTrigger::TurnEnd => state.settings.auto_on_turn_end,
    };
    if !enabled {
        return false;
    }

    let segments = partition_segments(entries);
    if segments.is_empty() {
        return false;
    }

    let keep = state.settings.keep_recent_turns as usize;
    let floor = state.settings.collapse_floor();
    let target = if state.settings.auto_l3_distant {
        SegmentLevel::L3
    } else {
        SegmentLevel::L2
    };

    let newest_ord = segments.iter().map(|s| s.turn_ordinal).max().unwrap_or(0);
    let mut changed = false;

    for seg in &segments {
        let from_newest = newest_ord.saturating_sub(seg.turn_ordinal);
        let in_recent_window = from_newest < keep;
        if in_recent_window {
            continue;
        }
        if protect_newest && seg.turn_ordinal == newest_ord {
            continue;
        }
        // Mark as having entered ActivityFold (eligible for later collapseNearest).
        state.mark_entered(&seg.id);
        let cur = state.level_of(&seg.id);
        let want =
            if cur == SegmentLevel::L0 || (target == SegmentLevel::L3 && cur == SegmentLevel::L2) {
                Some(target)
            } else if matches!(floor, SegmentLevel::L3) && cur == SegmentLevel::L2 {
                Some(SegmentLevel::L3)
            } else {
                None
            };
        if let Some(level) = want
            && state.set_level(&seg.id, level)
        {
            changed = true;
        }
    }
    changed
}

/// Whether `seg` is inside the keep_recent_turns virgin window (never collapse target).
pub fn in_virgin_recent_window(
    settings: &ActivityFoldSettings,
    segments: &[ActivitySegment],
    seg: &ActivitySegment,
) -> bool {
    let keep = settings.keep_recent_turns as usize;
    let newest_ord = segments.iter().map(|s| s.turn_ordinal).max().unwrap_or(0);
    let from_newest = newest_ord.saturating_sub(seg.turn_ordinal);
    from_newest < keep
}
