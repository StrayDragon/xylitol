use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

pub const CUSTOM_TYPE_PLAN_MODE: &str = "plan_mode";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanModeKind {
    #[default]
    Execute,
    Plan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMode {
    #[serde(default)]
    pub mode: PlanModeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

impl PlanMode {
    pub fn execute() -> Self {
        Self {
            mode: PlanModeKind::Execute,
            plan_id: None,
            updated_at: 0,
        }
    }

    pub fn plan(plan_id: Option<String>) -> Self {
        Self {
            mode: PlanModeKind::Plan,
            plan_id,
            updated_at: 0,
        }
    }

    pub fn to_data_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn from_data_value(data: &Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }

    pub fn to_custom_entry_shell(&self) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_PLAN_MODE.into(),
            data: self.to_data_value(),
        })
    }
}

pub fn latest_plan_mode(entries: &[SessionEntry]) -> PlanMode {
    entries
        .iter()
        .rev()
        .find_map(|entry| match entry {
            SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_MODE => {
                PlanMode::from_data_value(&custom.data)
            }
            _ => None,
        })
        .unwrap_or_else(PlanMode::execute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, fixture_message_json};

    fn message(id: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            message: fixture_message_json("user", id),
        })
    }

    #[test]
    fn missing_mode_defaults_to_execute() {
        assert_eq!(
            latest_plan_mode(&[message("m1")]).mode,
            PlanModeKind::Execute
        );
    }

    #[test]
    fn latest_mode_wins() {
        let mut older = PlanMode::plan(Some("p1".into()));
        older.updated_at = 1;
        let mut newer = PlanMode::execute();
        newer.updated_at = 2;
        let entries = vec![
            older.to_custom_entry_shell(),
            message("m1"),
            newer.to_custom_entry_shell(),
        ];
        assert_eq!(latest_plan_mode(&entries).mode, PlanModeKind::Execute);
    }

    #[test]
    fn mode_round_trips() {
        let original = PlanMode::plan(Some("p2".into()));
        let restored = latest_plan_mode(&[original.to_custom_entry_shell()]);
        assert_eq!(restored.mode, PlanModeKind::Plan);
        assert_eq!(restored.plan_id.as_deref(), Some("p2"));
    }
}
