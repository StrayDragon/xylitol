//! SessionTreeNode → package `TreeNode` mapping (c615 / ast6).

use xylitol_tui::{TreeNode, truncate_to_width, visible_width};

use crate::domain::session_types::{SessionEntry, SessionTreeNode, message_role, message_text};

const LABEL_PREVIEW_WIDTH: usize = 48;

/// Map Driver MessageHistory nodes to package tree rows (kind + plain-text label).
pub fn map_session_tree_nodes(nodes: &[SessionTreeNode]) -> Vec<TreeNode> {
    nodes.iter().map(map_session_tree_node).collect()
}

fn map_session_tree_node(node: &SessionTreeNode) -> TreeNode {
    let id = node.entry.entry_id().unwrap_or("?").to_string();
    let label = session_tree_display_label(node);
    let mut tree = TreeNode::new(id, label);
    if let Some(kind) = session_tree_node_kind(&node.entry) {
        tree = tree.with_kind(kind);
    }
    if !node.children.is_empty() {
        tree = tree.with_children(node.children.iter().map(map_session_tree_node));
    }
    tree
}

fn session_tree_node_kind(entry: &SessionEntry) -> Option<String> {
    match entry {
        SessionEntry::Message(m) => message_role(&m.message).map(str::to_string),
        SessionEntry::BashExecution(_) => Some("tool".into()),
        _ => None,
    }
}

fn session_tree_display_label(node: &SessionTreeNode) -> String {
    let raw = match &node.entry {
        SessionEntry::Message(m) => message_text(&m.message),
        SessionEntry::BashExecution(b) => b.command.clone(),
        SessionEntry::Compaction(c) => c.summary.clone(),
        SessionEntry::BranchSummary(b) => b.summary.clone(),
        SessionEntry::CustomMessage(c) => c.content.to_string(),
        SessionEntry::Custom(c) => c.data.to_string(),
        SessionEntry::ModelChange(m) => format!("{} / {}", m.provider, m.model_id),
        SessionEntry::ThinkingLevelChange(t) => t.thinking_level.clone(),
        SessionEntry::Label(l) => l.label.clone().unwrap_or_else(|| l.target_id.clone()),
        SessionEntry::SessionInfo(s) => s.name.clone().unwrap_or_else(|| "session".into()),
        SessionEntry::Header(h) => h.id.clone(),
    };
    preview_label_line(&raw)
}

fn preview_label_line(text: &str) -> String {
    let one = text.lines().next().unwrap_or(text).trim();
    if visible_width(one) > LABEL_PREVIEW_WIDTH {
        truncate_to_width(one, LABEL_PREVIEW_WIDTH, "…", false)
    } else {
        one.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session_types::{EntryBase, MessageEntry};

    fn user_node(id: &str, text: &str) -> SessionTreeNode {
        SessionTreeNode {
            entry: SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: None,
                    timestamp: "t".into(),
                },
                message: crate::domain::session_types::fixture_message_json("user", text),
            }),
            children: Vec::new(),
            label: None,
        }
    }

    #[test]
    fn maps_kind_and_plain_label() {
        let mapped = map_session_tree_nodes(&[user_node("u1", "hello")]);
        assert_eq!(mapped[0].id, "u1");
        assert_eq!(mapped[0].label, "hello");
        assert_eq!(mapped[0].kind.as_deref(), Some("user"));
        assert!(!mapped[0].label.contains("user:"));
    }
}
