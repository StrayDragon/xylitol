use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};
use super::todo::TodoStatus;

pub const CUSTOM_TYPE_TODO_CHECKPOINT: &str = "todo_checkpoint";
pub(crate) const LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT: &str = "plan_checkpoint";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoCheckpointStep {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoResumeCursor {
    pub task_id: String,
    #[serde(default)]
    pub attempt: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoCheckpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default)]
    pub steps: Vec<TodoCheckpointStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verified_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<TodoResumeCursor>,
    #[serde(default)]
    pub updated_at: u64,
}

impl TodoCheckpoint {
    pub fn unfinished_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status != TodoStatus::Completed)
            .count()
    }

    pub fn next_step(&self) -> Option<&TodoCheckpointStep> {
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
            custom_type: CUSTOM_TYPE_TODO_CHECKPOINT.into(),
            data: self.to_data_value(),
        })
    }
}

pub fn latest_todo_checkpoint(entries: &[SessionEntry]) -> Option<TodoCheckpoint> {
    for entry in entries.iter().rev() {
        let SessionEntry::Custom(custom) = entry else {
            continue;
        };
        if custom.custom_type != CUSTOM_TYPE_TODO_CHECKPOINT
            && custom.custom_type != LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT
        {
            continue;
        }
        return match TodoCheckpoint::from_data_value(&custom.data) {
            Some(checkpoint) => Some(checkpoint),
            None => {
                log::warn!(
                    target: "xylitol::session",
                    "latest {} entry fails to parse; treating session as having no checkpoint",
                    custom.custom_type
                );
                None
            }
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::fixture_message_json;
    use crate::session::{EntryBase, MessageEntry};

    fn checkpoint(steps: Vec<TodoCheckpointStep>) -> TodoCheckpoint {
        TodoCheckpoint {
            goal: Some("ship it".into()),
            steps,
            last_verified_artifact: Some("bash:call-1".into()),
            last_failure: Some("bash: tests failed".into()),
            resume_cursor: Some(TodoResumeCursor {
                task_id: "s2".into(),
                attempt: 2,
            }),
            updated_at: 1_700_000_000_000,
        }
    }

    fn step(id: &str, status: TodoStatus) -> TodoCheckpointStep {
        TodoCheckpointStep {
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
        let restored = latest_todo_checkpoint(&entries).expect("checkpoint");
        assert_eq!(restored, original);
    }

    #[test]
    fn legacy_plan_checkpoint_entry_is_read_as_todo_checkpoint() {
        let mut entry = checkpoint(vec![step("s1", TodoStatus::Pending)]).to_custom_entry_shell();
        if let SessionEntry::Custom(custom) = &mut entry {
            custom.custom_type = "plan_checkpoint".into();
        }
        let restored = latest_todo_checkpoint(&[entry]).expect("legacy checkpoint");
        assert_eq!(restored.goal.as_deref(), Some("ship it"));
    }

    #[test]
    fn new_checkpoint_writes_new_custom_type() {
        let entry = checkpoint(vec![step("s1", TodoStatus::Pending)]).to_custom_entry_shell();
        let SessionEntry::Custom(custom) = entry else {
            panic!("checkpoint must persist as a custom entry");
        };
        assert_eq!(custom.custom_type, "todo_checkpoint");
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
        let restored = latest_todo_checkpoint(&entries).expect("checkpoint");
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
            custom_type: CUSTOM_TYPE_TODO_CHECKPOINT.into(),
            data: serde_json::json!({ "steps": "not-an-array" }),
        });
        assert!(latest_todo_checkpoint(&[entry]).is_none());
    }

    struct CaptureWarn;

    impl log::Log for CaptureWarn {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                captured_warnings()
                    .lock()
                    .expect("capture lock")
                    .push(record.args().to_string());
            }
        }

        fn flush(&self) {}
    }

    static CAPTURE: CaptureWarn = CaptureWarn;

    fn captured_warnings() -> &'static std::sync::Mutex<Vec<String>> {
        static WARNINGS: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> =
            std::sync::OnceLock::new();
        WARNINGS.get_or_init(|| {
            let _ = log::set_logger(&CAPTURE);
            log::set_max_level(log::LevelFilter::Warn);
            std::sync::Mutex::new(Vec::new())
        })
    }

    fn corrupt_entry(custom_type: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: "corrupt".into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: custom_type.into(),
            data: serde_json::json!({ "steps": "not-an-array" }),
        })
    }

    #[test]
    fn corrupt_latest_checkpoint_short_circuits_without_backscan() {
        let warnings = captured_warnings();
        let before = warnings.lock().expect("capture lock").len();
        let older = checkpoint(vec![step("s1", TodoStatus::Pending)]).to_custom_entry_shell();
        let entries = vec![
            older,
            message("m1"),
            corrupt_entry(CUSTOM_TYPE_TODO_CHECKPOINT),
        ];
        assert!(latest_todo_checkpoint(&entries).is_none());
        let lines = warnings.lock().expect("capture lock");
        let fresh = &lines[before.min(lines.len())..];
        assert!(
            fresh.iter().any(|line| line.contains("todo_checkpoint")),
            "expected warn for corrupt latest checkpoint, got {fresh:?}"
        );
    }

    #[test]
    fn corrupt_legacy_latest_checkpoint_short_circuits_without_backscan() {
        let warnings = captured_warnings();
        let before = warnings.lock().expect("capture lock").len();
        let older = checkpoint(vec![step("s1", TodoStatus::Pending)]).to_custom_entry_shell();
        let entries = vec![older, corrupt_entry(LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT)];
        assert!(latest_todo_checkpoint(&entries).is_none());
        let lines = warnings.lock().expect("capture lock");
        let fresh = &lines[before.min(lines.len())..];
        assert!(
            fresh.iter().any(|line| line.contains("plan_checkpoint")),
            "expected warn for corrupt legacy checkpoint, got {fresh:?}"
        );
    }

    #[test]
    fn entries_without_checkpoint_match_yield_none() {
        assert!(latest_todo_checkpoint(&[message("m1"), message("m2")]).is_none());
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
