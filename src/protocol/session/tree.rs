//! Session tree shapes and MessageHistory travel planner.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::entries::SessionEntry;
use super::helpers::{is_user_message, message_text};
use crate::protocol::error::XySessionError;

/// Kind of session tree exposed via [`crate::app::core::driver::XyDriver`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionTreeKind {
    MessageHistory,
    /// Reserved; callers MUST receive an explicit error until implemented.
    FileBrowser,
}

/// Result of travelling a session tree — leaf position plus optional editor prefill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTreeTravel {
    pub kind: SessionTreeKind,
    pub selected_id: String,
    pub leaf_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_text: Option<String>,
}

/// Tree node for getTree() - defensive copy of session structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTreeNode {
    /// The session entry at this node.
    pub entry: SessionEntry,
    /// Child nodes in timestamp order.
    pub children: Vec<SessionTreeNode>,
    /// Resolved label for this entry, if any.
    pub label: Option<String>,
}

// ── Session tree ────────────────────────────────────────────────────

/// Build a parent/child tree from flat session entries (labels resolved).
pub fn build_session_tree(entries: &[SessionEntry]) -> Vec<SessionTreeNode> {
    use std::collections::HashMap;

    let mut labels: HashMap<String, String> = HashMap::new();
    for entry in entries {
        if let SessionEntry::Label(l) = entry {
            if let Some(ref label) = l.label {
                labels.insert(l.target_id.clone(), label.clone());
            } else {
                labels.remove(&l.target_id);
            }
        }
    }

    let mut node_map: HashMap<String, SessionTreeNode> = HashMap::new();
    for entry in entries {
        if matches!(entry, SessionEntry::Label(_) | SessionEntry::Header(_)) {
            continue;
        }
        if let Some(id) = entry.entry_id() {
            let label = labels.get(id).cloned();
            node_map.insert(
                id.to_string(),
                SessionTreeNode {
                    entry: entry.clone(),
                    children: Vec::new(),
                    label,
                },
            );
        }
    }

    let mut roots: Vec<SessionTreeNode> = Vec::new();

    // Link children to parents (reverse order so parents stay in node_map).
    for entry in entries.iter().rev() {
        if matches!(entry, SessionEntry::Label(_) | SessionEntry::Header(_)) {
            continue;
        }
        let Some(id) = entry.entry_id() else {
            continue;
        };
        let Some(node) = node_map.remove(id) else {
            continue;
        };

        if let Some(parent_id) = entry.parent_id() {
            if let Some(parent) = node_map.get_mut(parent_id) {
                parent.children.push(node);
            } else {
                roots.push(node);
            }
        } else {
            roots.push(node);
        }
    }

    fn sort_children(nodes: &mut [SessionTreeNode]) {
        for node in nodes.iter_mut() {
            node.children.sort_by(|a, b| {
                let ta = a.entry.base().map(|b| b.timestamp);
                let tb = b.entry.base().map(|b| b.timestamp);
                ta.cmp(&tb)
            });
            sort_children(&mut node.children);
        }
    }
    sort_children(&mut roots);

    roots
}

/// Pure planner for MessageHistory travel (pi / c600 semantics).
pub fn plan_message_history_travel(
    entries: &[SessionEntry],
    selected_id: &str,
) -> Result<SessionTreeTravel, XySessionError> {
    let selected = entries
        .iter()
        .find(|e| e.entry_id() == Some(selected_id))
        .ok_or_else(|| XySessionError::entry_not_found(selected_id))?;

    if is_user_message(selected) {
        let SessionEntry::Message(m) = selected else {
            return Err(XySessionError::entry_not_found(selected_id));
        };
        Ok(SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: selected_id.to_string(),
            leaf_id: selected.parent_id().map(str::to_string),
            editor_text: Some(message_text(&m.message)),
        })
    } else {
        Ok(SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: selected_id.to_string(),
            leaf_id: Some(selected_id.to_string()),
            editor_text: None,
        })
    }
}
