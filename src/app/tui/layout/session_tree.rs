//! SessionTreeNode → package `TreeNode` mapping (c615 / ast6) + product FilterMode (c635).

use xylitol_tui::{TreeNode, truncate_to_width, visible_width};

use crate::domain::session_types::{SessionEntry, SessionTreeNode, message_role, message_text};

const LABEL_PREVIEW_WIDTH: usize = 48;

/// Product session-tree filter modes (aligned with pi; wired via `include_node` + `status_suffix`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    Default,
    NoTools,
    UserOnly,
    LabeledOnly,
    All,
}

impl FilterMode {
    const ALL: [Self; 5] = [
        Self::Default,
        Self::NoTools,
        Self::UserOnly,
        Self::LabeledOnly,
        Self::All,
    ];

    pub fn include(self, node: &TreeNode) -> bool {
        let kind = node.kind.as_deref().unwrap_or("");
        match self {
            Self::Default => kind != "meta",
            Self::NoTools => kind != "meta" && kind != "tool",
            Self::UserOnly => kind == "user",
            Self::LabeledOnly => node.annotation.is_some(),
            Self::All => true,
        }
    }

    pub fn status_suffix(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::NoTools => Some("[no-tools]"),
            Self::UserOnly => Some("[user]"),
            Self::LabeledOnly => Some("[labeled]"),
            Self::All => Some("[all]"),
        }
    }

    pub fn cycle(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Toggle `target` ↔ `default` (pi semantics for Ctrl+T/U/L/A).
    pub fn toggle(self, target: Self) -> Self {
        if self == target {
            Self::Default
        } else {
            target
        }
    }
}

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
    if let Some(annotation) = node.label.as_ref() {
        tree = tree.with_annotation(annotation.clone());
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
        SessionEntry::ModelChange(_)
        | SessionEntry::ThinkingLevelChange(_)
        | SessionEntry::Label(_)
        | SessionEntry::SessionInfo(_)
        | SessionEntry::Custom(_)
        | SessionEntry::CustomMessage(_)
        | SessionEntry::Header(_) => Some("meta".into()),
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
    use crate::domain::session_types::{EntryBase, MessageEntry, ModelChangeEntry};

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

    #[test]
    fn maps_domain_label_to_annotation() {
        let mut node = user_node("u1", "hello");
        node.label = Some("keep".into());
        let mapped = map_session_tree_nodes(&[node]);
        assert_eq!(mapped[0].annotation.as_deref(), Some("keep"));
    }

    #[test]
    fn maps_bookkeeping_to_meta_kind() {
        let node = SessionTreeNode {
            entry: SessionEntry::ModelChange(ModelChangeEntry {
                base: EntryBase {
                    entry_type: "model_change".into(),
                    id: "mc1".into(),
                    parent_id: None,
                    timestamp: "t".into(),
                },
                provider: "fake".into(),
                model_id: "model-a".into(),
            }),
            children: Vec::new(),
            label: None,
        };
        let mapped = map_session_tree_nodes(&[node]);
        assert_eq!(mapped[0].kind.as_deref(), Some("meta"));
    }

    #[test]
    fn filter_mode_default_hides_meta() {
        let node = TreeNode::new("m", "meta row").with_kind("meta");
        assert!(!FilterMode::Default.include(&node));
        assert!(FilterMode::All.include(&node));
    }
}
