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
    pub profile_enabled: bool,
    pub session_disabled: bool,
    pub effective_enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub entries: Vec<CapabilityEntry>,
    pub pending: bool,
}
