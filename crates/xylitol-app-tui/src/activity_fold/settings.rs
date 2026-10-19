//! ActivityFold runtime knobs (att26 / rc28). YAML lands on `tui.activity_fold`.

use super::SegmentLevel;

/// Runtime knobs for envelope / cluster auto-collapse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityFoldSettings {
    pub enabled: bool,
    pub keep_recent_turns: u32,
    pub auto_on_rebuild: bool,
    pub auto_on_turn_end: bool,
    /// Ended / distant turns fold floor (`L3` envelope `Worked for`, `L2`
    /// cluster heads only). Config spelling (`stream_collapse`) maps to this in
    /// `app::cli` (owns config) — no infra config DTO import on the TUI side.
    pub collapse_floor: SegmentLevel,
}

impl Default for ActivityFoldSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
            // Matches `TuiActivityFoldConfig::default()` (`envelope` → L3).
            collapse_floor: SegmentLevel::L3,
        }
    }
}

impl ActivityFoldSettings {
    /// `stream_collapse: envelope` paints `Worked for` when the envelope is in
    /// play (L2 expanded / L3 collapsed). Keep-window L0 and live window do not.
    pub fn paints_envelope_header(&self) -> bool {
        self.collapse_floor == SegmentLevel::L3
    }
}
