use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};
use super::todo::TodoStatus;

pub const CUSTOM_TYPE_PLAN_CHECKPOINT: &str = "plan_checkpoint";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanCheckpointStep {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanResumeCursor {
    pub step_id: String,
    #[serde(default)]
    pub attempt: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanCheckpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default)]
    pub steps: Vec<PlanCheckpointStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verified_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<PlanResumeCursor>,
    #[serde(default)]
    pub updated_at: u64,
}

impl PlanCheckpoint {
    pub fn unfinished_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status != TodoStatus::Completed)
            .count()
    }

    pub fn next_step(&self) -> Option<&PlanCheckpointStep> {
        self.steps
            .iter()
            .find(|step| step.status != TodoStatus::Completed)
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
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
            custom_type: CUSTOM_TYPE_PLAN_CHECKPOINT.into(),
            data: self.to_data_value(),
        })
    }
}

pub fn latest_plan_checkpoint(entries: &[SessionEntry]) -> Option<PlanCheckpoint> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_CHECKPOINT => {
            PlanCheckpoint::from_data_value(&custom.data)
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::fixture_message_json;
    use crate::session::{EntryBase, MessageEntry};

    fn checkpoint(steps: Vec<PlanCheckpointStep>) -> PlanCheckpoint {
        PlanCheckpoint {
            goal: Some("ship it".into()),
            steps,
            last_verified_artifact: Some("bash:call-1".into()),
            last_failure: Some("bash: tests failed".into()),
            resume_cursor: Some(PlanResumeCursor {
                step_id: "s2".into(),
                attempt: 2,
            }),
            updated_at: 1_700_000_000_000,
        }
    }

    fn step(id: &str, status: TodoStatus) -> PlanCheckpointStep {
        PlanCheckpointStep {
            id: id.into(),
            content: format!("step {id}"),
            status,
            verification: Some(format!("cargo test -p {id}")),
            output_ref: None,
        }
    }

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
    fn checkpoint_round_trips_through_custom_entry() {
        let original = checkpoint(vec![
            step("s1", TodoStatus::Completed),
            step("s2", TodoStatus::InProgress),
        ]);
        let entry = original.to_custom_entry_shell();
        let entries = vec![entry];
        let restored = latest_plan_checkpoint(&entries).expect("checkpoint");
        assert_eq!(restored, original);
    }

    #[test]
    fn latest_checkpoint_wins_and_ignores_other_entries() {
        let older = checkpoint(vec![step("s1", TodoStatus::Pending)]);
        let mut newer = checkpoint(vec![step("s1", TodoStatus::Completed)]);
        newer.updated_at = older.updated_at + 1;
        let entries = vec![
            older.to_custom_entry_shell(),
            message("m1"),
            newer.to_custom_entry_shell(),
        ];
        let restored = latest_plan_checkpoint(&entries).expect("checkpoint");
        assert_eq!(restored.updated_at, newer.updated_at);
        assert_eq!(restored.unfinished_count(), 0);
    }

    #[test]
    fn malformed_checkpoint_is_ignored() {
        let entry = SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_PLAN_CHECKPOINT.into(),
            data: serde_json::json!({ "steps": "not-an-array" }),
        });
        assert!(latest_plan_checkpoint(&[entry]).is_none());
    }

    #[test]
    fn next_step_skips_completed() {
        let checkpoint = checkpoint(vec![
            step("s1", TodoStatus::Completed),
            step("s2", TodoStatus::InProgress),
            step("s3", TodoStatus::Pending),
        ]);
        assert_eq!(checkpoint.next_step().map(|s| s.id.as_str()), Some("s2"));
        assert_eq!(checkpoint.unfinished_count(), 2);
    }
}
