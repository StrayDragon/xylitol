//! ActivityFold runtime knobs (att26 / rc28). YAML lands on `tui.activity_fold`.

use crate::infra::config::types::{ActivityFoldStreamCollapse, TuiActivityFoldConfig};

use super::SegmentLevel;

/// Runtime knobs for envelope / cluster auto-collapse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityFoldSettings {
    pub enabled: bool,
    pub keep_recent_turns: u32,
    pub auto_on_rebuild: bool,
    pub auto_on_turn_end: bool,
    /// Ended / distant turns: envelope (`Worked for`) or cluster heads only.
    pub stream_collapse: ActivityFoldStreamCollapse,
}

impl Default for ActivityFoldSettings {
    fn default() -> Self {
        TuiActivityFoldConfig::default().into()
    }
}

impl From<TuiActivityFoldConfig> for ActivityFoldSettings {
    fn from(c: TuiActivityFoldConfig) -> Self {
        Self {
            enabled: c.enabled,
            keep_recent_turns: c.keep_recent_turns,
            auto_on_rebuild: c.auto_on_rebuild,
            auto_on_turn_end: c.auto_on_turn_end,
            stream_collapse: c.stream_collapse,
        }
    }
}

impl ActivityFoldSettings {
    /// Envelope collapsed → L3 (`Worked for`); clusters-only → L2 (cluster heads).
    pub fn collapse_floor(&self) -> super::SegmentLevel {
        match self.stream_collapse {
            ActivityFoldStreamCollapse::Envelope => SegmentLevel::L3,
            ActivityFoldStreamCollapse::Clusters => SegmentLevel::L2,
        }
    }

    /// `stream_collapse: envelope` paints `Worked for` when the envelope is in
    /// play (L2 expanded / L3 collapsed). Keep-window L0 and live window do not.
    pub fn paints_envelope_header(&self) -> bool {
        matches!(self.stream_collapse, ActivityFoldStreamCollapse::Envelope)
    }
}
