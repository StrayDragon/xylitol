//! Agent Todo SSOT vocabulary (`custom_type = "agent_todo"`).
//!
//! Persist via [`crate::protocol::session::SessionEntry::Custom`]; never
//! [`crate::protocol::session::CustomMessageEntry`] / session_env (those enter
//! Request-time AgentStatusBar inject is a projection, not SSOT.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum::IntoStaticStr;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

/// Wire discriminator for Todo Custom snapshots.
pub const CUSTOM_TYPE_AGENT_TODO: &str = "agent_todo";

/// Max Unicode scalars in a trimmed item `content` (write reject, no clip).
pub const TODO_CONTENT_MAX_CHARS: usize = 80;

/// Closed status set for Todo items.
///
/// `IntoStaticStr` keeps one snake_case SSOT shared by [`Self::as_str`] and the
/// serde wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoStatus {
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

/// One Todo row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

/// Ordered Todo list (empty is legal).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

impl TodoList {
    pub fn new(items: Vec<TodoItem>) -> Self {
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Completed count over total (for TUI summary).
    pub fn done_total(&self) -> (usize, usize) {
        let done = self
            .items
            .iter()
            .filter(|i| i.status == TodoStatus::Completed)
            .count();
        (done, self.items.len())
    }

    pub fn summary_line(&self) -> String {
        let (done, total) = self.done_total();
        format!("Todo · {done}/{total}")
    }

    pub fn to_data_value(&self) -> Value {
        json!({ "items": self.items })
    }

    /// Load a snapshot. Rows with status `cancelled` are dropped so old JSONL
    /// still yields a list; other invalid rows fail the whole payload.
    pub fn from_data_value(data: &Value) -> Result<Self, TodoValidationError> {
        let Some(items) = data.get("items").and_then(Value::as_array) else {
            return Err(TodoValidationError(
                "invalid agent_todo payload: missing items array".into(),
            ));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            if item.get("status").and_then(Value::as_str) == Some("cancelled") {
                continue;
            }
            let parsed: TodoItem = serde_json::from_value(item.clone())
                .map_err(|e| TodoValidationError(format!("invalid agent_todo payload: {e}")))?;
            out.push(parsed);
        }
        Ok(TodoList::new(out))
    }

    pub fn to_custom_entry_shell(&self) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AGENT_TODO.into(),
            data: self.to_data_value(),
        })
    }
}

/// Validation error for Todo mutate paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoValidationError(pub String);

impl std::fmt::Display for TodoValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TodoValidationError {}

/// Validate domain rules: non-empty trimmed content, closed status set (via
/// deserialize), content length cap.
pub fn validate_todo_list(list: &TodoList) -> Result<(), TodoValidationError> {
    for (idx, item) in list.items.iter().enumerate() {
        if item.id.trim().is_empty() {
            return Err(TodoValidationError(format!(
                "todo item at index {idx}: id must be non-empty"
            )));
        }
        if item.content.trim().is_empty() {
            return Err(TodoValidationError(format!(
                "todo item '{}': content must be non-empty after trim",
                item.id
            )));
        }
        if item.content.trim().chars().count() > TODO_CONTENT_MAX_CHARS {
            return Err(TodoValidationError(format!(
                "todo item '{}': content must be at most {TODO_CONTENT_MAX_CHARS} Unicode scalars after trim",
                item.id
            )));
        }
    }
    Ok(())
}

/// Latest-wins fold over a **full leaf** branch (not the LLM context window).
pub fn latest_agent_todo(entries: &[SessionEntry]) -> Option<TodoList> {
    entries.iter().rev().find_map(|e| match e {
        SessionEntry::Custom(c) if c.custom_type == CUSTOM_TYPE_AGENT_TODO => {
            TodoList::from_data_value(&c.data).ok()
        }
        _ => None,
    })
}

fn mint_todo_id(existing: &[TodoItem]) -> String {
    loop {
        let b = uuid::Uuid::new_v4();
        let bytes = b.as_bytes();
        let id = format!(
            "t_{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        );
        if !existing.iter().any(|i| i.id == id) {
            return id;
        }
    }
}

/// Normalize rewrite inputs: generate missing ids; trim content.
pub fn normalize_rewrite_items(items: Vec<TodoItemDraft>) -> Result<TodoList, TodoValidationError> {
    let mut out = Vec::with_capacity(items.len());
    for draft in items {
        let id = match draft.id {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => mint_todo_id(&out),
        };
        out.push(TodoItem {
            id,
            content: draft.content.trim().to_string(),
            status: draft.status,
        });
    }
    let list = TodoList::new(out);
    validate_todo_list(&list)?;
    Ok(list)
}

/// Rewrite draft row (`id` optional).
#[derive(Debug, Clone, Deserialize)]
pub struct TodoItemDraft {
    pub id: Option<String>,
    pub content: String,
    #[serde(default = "default_pending")]
    pub status: TodoStatus,
}

fn default_pending() -> TodoStatus {
    TodoStatus::Pending
}

/// One `todo_update` patch row.
#[derive(Debug, Clone, Deserialize)]
pub struct TodoItemPatch {
    pub id: String,
    pub status: Option<TodoStatus>,
    pub content: Option<String>,
    pub after_id: Option<String>,
}

/// Apply `todo_update` patches in order (validate after).
pub fn apply_todo_patches(
    list: &TodoList,
    patches: &[TodoItemPatch],
) -> Result<TodoList, TodoValidationError> {
    if patches.is_empty() {
        return Err(TodoValidationError(
            "todo_update requires a non-empty items array".into(),
        ));
    }
    let mut items = list.items.clone();
    for patch in patches {
        if patch.status.is_none() && patch.content.is_none() && patch.after_id.is_none() {
            return Err(TodoValidationError(format!(
                "todo item '{}': patch must set status, content, or after_id",
                patch.id
            )));
        }
        let idx = items
            .iter()
            .position(|i| i.id == patch.id)
            .ok_or_else(|| TodoValidationError(format!("unknown todo id: {}", patch.id)))?;
        if let Some(s) = patch.status {
            items[idx].status = s;
        }
        if let Some(ref c) = patch.content {
            items[idx].content = c.trim().to_string();
        }
        if let Some(ref after) = patch.after_id {
            if after == &patch.id {
                return Err(TodoValidationError(format!(
                    "todo item '{}': after_id must not be the same id",
                    patch.id
                )));
            }
            let item = items.remove(idx);
            let insert_at = items
                .iter()
                .position(|i| i.id == *after)
                .ok_or_else(|| TodoValidationError(format!("unknown todo id: {after}")))?;
            items.insert(insert_at + 1, item);
        }
    }
    let next = TodoList::new(items);
    validate_todo_list(&next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, content: &str, status: TodoStatus) -> TodoItem {
        TodoItem {
            id: id.into(),
            content: content.into(),
            status,
        }
    }

    fn custom(data: Value) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: "c".into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AGENT_TODO.into(),
            data,
        })
    }

    #[test]
    fn domain_status_closed_set_roundtrip() {
        for s in [
            TodoStatus::Pending,
            TodoStatus::InProgress,
            TodoStatus::Completed,
        ] {
            let v = serde_json::to_value(s).unwrap();
            let back: TodoStatus = serde_json::from_value(v).unwrap();
            assert_eq!(back, s);
        }
        assert!(serde_json::from_str::<TodoStatus>(r#""planned""#).is_err());
        assert!(serde_json::from_str::<TodoStatus>(r#""cancelled""#).is_err());
    }

    #[test]
    fn from_data_value_skips_cancelled_rows() {
        let data = json!({
            "items": [
                {"id": "1", "content": "keep", "status": "pending"},
                {"id": "2", "content": "drop", "status": "cancelled"},
                {"id": "3", "content": "also", "status": "completed"}
            ]
        });
        let list = TodoList::from_data_value(&data).expect("cancelled rows skipped");
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].id, "1");
        assert_eq!(list.items[1].id, "3");
    }

    #[test]
    fn latest_wins_reads_snapshot_with_cancelled_row() {
        let data = json!({
            "items": [
                {"id": "1", "content": "keep", "status": "in_progress"},
                {"id": "x", "content": "old", "status": "cancelled"}
            ]
        });
        assert_eq!(
            latest_agent_todo(&[custom(data.clone())])
                .expect("load")
                .items
                .len(),
            1
        );
    }

    #[test]
    fn latest_wins_full_scan() {
        let a = TodoList::new(vec![item("1", "a", TodoStatus::Pending)]);
        let b = TodoList::new(vec![
            item("1", "a", TodoStatus::Completed),
            item("2", "b", TodoStatus::InProgress),
        ]);
        let branch = vec![custom(a.to_data_value()), custom(b.to_data_value())];
        assert_eq!(latest_agent_todo(&branch), Some(b));
    }

    #[test]
    fn dual_in_progress_is_allowed() {
        let list = TodoList::new(vec![
            item("1", "a", TodoStatus::InProgress),
            item("2", "b", TodoStatus::InProgress),
        ]);
        assert!(validate_todo_list(&list).is_ok());
    }

    #[test]
    fn empty_content_rejects() {
        let list = TodoList::new(vec![item("1", "  ", TodoStatus::Pending)]);
        assert!(validate_todo_list(&list).is_err());
    }

    #[test]
    fn content_over_eighty_scalars_rejects() {
        let too_long = "a".repeat(TODO_CONTENT_MAX_CHARS + 1);
        let list = TodoList::new(vec![item("1", &too_long, TodoStatus::Pending)]);
        let err = validate_todo_list(&list).unwrap_err();
        assert!(err.0.contains("80"), "{err}");
    }

    #[test]
    fn content_exactly_eighty_scalars_ok() {
        let exact = "你".repeat(TODO_CONTENT_MAX_CHARS);
        let list = TodoList::new(vec![item("1", &exact, TodoStatus::Pending)]);
        assert!(validate_todo_list(&list).is_ok());
    }

    #[test]
    fn rewrite_omitted_id_is_t_hex() {
        let list = normalize_rewrite_items(vec![TodoItemDraft {
            id: None,
            content: "x".into(),
            status: TodoStatus::Pending,
        }])
        .unwrap();
        assert!(
            list.items[0].id.starts_with("t_") && list.items[0].id.len() == 10,
            "id={}",
            list.items[0].id
        );
        assert!(!list.items[0].id.starts_with("todo-"));
    }

    #[test]
    fn update_unknown_id_rejects() {
        let list = TodoList::new(vec![item("1", "a", TodoStatus::Pending)]);
        let err = apply_todo_patches(
            &list,
            &[TodoItemPatch {
                id: "missing".into(),
                status: Some(TodoStatus::Completed),
                content: None,
                after_id: None,
            }],
        )
        .unwrap_err();
        assert!(err.0.contains("unknown todo id"));
    }

    #[test]
    fn update_batch_ticks_and_reorders() {
        let list = TodoList::new(vec![
            item("a", "one", TodoStatus::InProgress),
            item("b", "two", TodoStatus::Pending),
            item("c", "three", TodoStatus::Pending),
        ]);
        let next = apply_todo_patches(
            &list,
            &[
                TodoItemPatch {
                    id: "a".into(),
                    status: Some(TodoStatus::Completed),
                    content: None,
                    after_id: None,
                },
                TodoItemPatch {
                    id: "b".into(),
                    status: Some(TodoStatus::InProgress),
                    content: None,
                    after_id: Some("c".into()),
                },
            ],
        )
        .unwrap();
        assert_eq!(next.items[0].id, "a");
        assert_eq!(next.items[0].status, TodoStatus::Completed);
        assert_eq!(next.items[1].id, "c");
        assert_eq!(next.items[2].id, "b");
        assert_eq!(next.items[2].status, TodoStatus::InProgress);
    }

    #[test]
    fn summary_line_counts_done() {
        let list = TodoList::new(vec![
            item("1", "a", TodoStatus::Completed),
            item("2", "b", TodoStatus::Pending),
            item("3", "c", TodoStatus::InProgress),
        ]);
        assert_eq!(list.summary_line(), "Todo · 1/3");
    }
}
