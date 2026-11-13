use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    McpServer,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEntry {
    pub id: String,
    pub kind: CapabilityKind,
    pub label: String,
    pub session_disabled: bool,
    profile_enabled: bool,
    effective_enabled: bool,
    pending: bool,
}

impl CapabilityEntry {
    pub fn new(
        id: String,
        kind: CapabilityKind,
        label: String,
        session_disabled: bool,
        applied: bool,
    ) -> Self {
        Self {
            id,
            kind,
            label,
            profile_enabled: true,
            session_disabled,
            effective_enabled: !session_disabled,
            pending: session_disabled != applied,
        }
    }

    pub fn profile_enabled(&self) -> bool {
        self.profile_enabled
    }

    pub fn effective_enabled(&self) -> bool {
        self.effective_enabled
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub entries: Vec<CapabilityEntry>,
    pub pending: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_bools_hold_the_construction_identity() {
        for (session_disabled, applied) in
            [(false, false), (false, true), (true, false), (true, true)]
        {
            let entry = CapabilityEntry::new(
                "tool:read".into(),
                CapabilityKind::Tool,
                "read".into(),
                session_disabled,
                applied,
            );
            assert!(entry.profile_enabled());
            assert_eq!(entry.session_disabled, session_disabled);
            assert_eq!(entry.effective_enabled(), !session_disabled);
            assert_eq!(entry.pending(), session_disabled != applied);
        }
    }
}
