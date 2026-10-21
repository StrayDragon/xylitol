use super::SegmentLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityFoldSettings {
    pub enabled: bool,
    pub keep_recent_turns: u32,
    pub auto_on_rebuild: bool,
    pub auto_on_turn_end: bool,
    pub collapse_floor: SegmentLevel,
}

impl Default for ActivityFoldSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
            collapse_floor: SegmentLevel::L3,
        }
    }
}

impl ActivityFoldSettings {
    pub fn paints_envelope_header(&self) -> bool {
        self.collapse_floor == SegmentLevel::L3
    }
}
