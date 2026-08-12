//! Code-first ActivityFold defaults (att26). YAML MAY land later.

/// Runtime knobs for segment auto-degrade. Defaults match att26 / design.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityFoldSettings {
    /// Master switch; when false every segment paints as L0 (L1 still works).
    pub enabled: bool,
    /// Newest K activity turns stay L0 under auto-degrade.
    pub keep_recent_turns: u32,
    /// After travel/resume/fork rebuild, crush older segments to ≥L2.
    pub auto_on_rebuild: bool,
    /// After turn end (idle), crush older segments to ≥L2.
    pub auto_on_turn_end: bool,
    /// When true, distant segments may go to L3 and collapse floor is L3.
    pub auto_l3_distant: bool,
}

impl Default for ActivityFoldSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
            auto_l3_distant: false,
        }
    }
}

impl ActivityFoldSettings {
    /// Floor level when collapsing / auto-crushing distant segments.
    pub fn collapse_floor(&self) -> super::SegmentLevel {
        if self.auto_l3_distant {
            super::SegmentLevel::L3
        } else {
            super::SegmentLevel::L2
        }
    }
}
