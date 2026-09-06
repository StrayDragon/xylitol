//! Agent Todo SSOT vocabulary (`custom_type = "agent_todo"`).
//!
//! Persist via [`crate::protocol::session::SessionEntry::Custom`]; never
//! [`crate::protocol::session::CustomMessageEntry`] / session_env (those enter
//! the LLM prefix).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum::IntoStaticStr;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

/// Wire discriminator for Todo Custom snapshots.
pub const CUSTOM_TYPE_AGENT_TODO: &str = "agent_todo";

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
    Cancelled,
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

    /// Completed + cancelled count over total (for TUI summary).
    pub fn done_total(&self) -> (usize, usize) {
        let done = self
            .items
            .iter()
            .filter(|i| matches!(i.status, TodoStatus::Completed | TodoStatus::Cancelled))
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

    pub fn from_data_value(data: &Value) -> Result<Self, TodoValidationError> {
        serde_json::from_value(data.clone())
            .map_err(|e| TodoValidationError(format!("invalid agent_todo payload: {e}")))
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
/// deserialize), at most one `in_progress`.
pub fn validate_todo_list(list: &TodoList) -> Result<(), TodoValidationError> {
    let mut in_progress = 0usize;
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
        if item.status == TodoStatus::InProgress {
            in_progress += 1;
        }
    }
    if in_progress > 1 {
        return Err(TodoValidationError(
            "at most one todo item may be in_progress".into(),
        ));
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

/// Normalize rewrite inputs: generate missing ids; trim content.
pub fn normalize_rewrite_items(items: Vec<TodoItemDraft>) -> Result<TodoList, TodoValidationError> {
    let mut out = Vec::with_capacity(items.len());
    for (idx, draft) in items.into_iter().enumerate() {
        let id = match draft.id {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => format!("todo-{}", idx + 1),
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

/// Apply `todo_update` to a list (validate after).
pub fn apply_todo_update(
    list: &TodoList,
    id: &str,
    status: Option<TodoStatus>,
    content: Option<String>,
) -> Result<TodoList, TodoValidationError> {
    if status.is_none() && content.is_none() {
        return Err(TodoValidationError(
            "todo_update requires status and/or content".into(),
        ));
    }
    let mut items = list.items.clone();
    let Some(item) = items.iter_mut().find(|i| i.id == id) else {
        return Err(TodoValidationError(format!("unknown todo id: {id}")));
    };
    if let Some(s) = status {
        item.status = s;
    }
    if let Some(c) = content {
        item.content = c.trim().to_string();
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
            TodoStatus::Cancelled,
        ] {
            let v = serde_json::to_value(s).unwrap();
            let back: TodoStatus = serde_json::from_value(v).unwrap();
            assert_eq!(back, s);
        }
        assert!(serde_json::from_str::<TodoStatus>(r#""planned""#).is_err());
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
    fn dual_in_progress_rejects() {
        let list = TodoList::new(vec![
            item("1", "a", TodoStatus::InProgress),
            item("2", "b", TodoStatus::InProgress),
        ]);
        let err = validate_todo_list(&list).unwrap_err();
        assert!(err.0.contains("in_progress"));
    }

    #[test]
    fn empty_content_rejects() {
        let list = TodoList::new(vec![item("1", "  ", TodoStatus::Pending)]);
        assert!(validate_todo_list(&list).is_err());
    }

    #[test]
    fn update_unknown_id_rejects() {
        let list = TodoList::new(vec![item("1", "a", TodoStatus::Pending)]);
        let err =
            apply_todo_update(&list, "missing", Some(TodoStatus::Completed), None).unwrap_err();
        assert!(err.0.contains("unknown todo id"));
    }

    #[test]
    fn summary_line_counts_done() {
        let list = TodoList::new(vec![
            item("1", "a", TodoStatus::Completed),
            item("2", "b", TodoStatus::Pending),
            item("3", "c", TodoStatus::Cancelled),
        ]);
        assert_eq!(list.summary_line(), "Todo · 2/3");
    }
}
