//! SessionTreeNode → package `TreeNode` mapping (c615 / ast6) + product FilterMode (c635).
//! Tree slot Search/Help helpers (c685) — layout, not browser chrome.

use xylitol_tui::{TreeNode, truncate_to_width, visible_width, with_keybindings};

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

    /// Reverse of [`Self::cycle`] (pi `filter.cycleBackward` / Ctrl+Shift+O).
    pub fn cycle_backward(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
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

/// Product filter chords (not yet registered as `app.tree.filter.*` in the package).
const TREE_FILTER_KEY_IDS: &[&str] = &["ctrl+d", "ctrl+t", "ctrl+u", "ctrl+l", "ctrl+a"];
const TREE_CYCLE_KEY_IDS: &[&str] = &["ctrl+o", "ctrl+shift+o"];

/// Search line above the tree list (pi `SearchLine` / demo morphology).
pub(crate) fn tree_search_line(query: &str) -> String {
    if query.is_empty() {
        " Type to search:".into()
    } else {
        format!(" Search: {query}")
    }
}

/// Dynamic TreeHelp from KeybindingsManager + product filter/cycle chords (c685).
pub(crate) fn tree_help_line() -> String {
    let move_keys = binding_first_keys(&["tui.select.up", "tui.select.down"]);
    let page_keys = binding_first_keys(&["tui.select.pageUp", "tui.select.pageDown"]);
    let branch_keys = binding_first_keys(&["tui.tree.foldOrUp", "tui.tree.unfoldOrDown"]);
    let label_keys = binding_first_keys(&["tui.tree.editLabel"]);
    let label_time_keys = binding_first_keys(&["tui.tree.toggleLabelTimestamp"]);

    let mut parts = Vec::new();
    // Purpose-first labels; ⊞/⊟ match tree fold markers (pi connector indicators).
    push_help_item(&mut parts, &move_keys, "move", false);
    push_help_item(&mut parts, &page_keys, "page", false);
    push_help_item(&mut parts, &branch_keys, "fold/unfold", false);
    push_help_item(&mut parts, &label_keys, "edit label", false);
    push_help_item(&mut parts, &label_time_keys, "timestamps", false);
    push_help_item(
        &mut parts,
        &TREE_FILTER_KEY_IDS
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>(),
        "filters",
        true,
    );
    push_help_item(
        &mut parts,
        &TREE_CYCLE_KEY_IDS
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>(),
        "cycle filter",
        true,
    );

    format!(" {}", parts.join(" · "))
}

fn binding_first_keys(ids: &[&'static str]) -> Vec<String> {
    with_keybindings(|kb| {
        ids.iter()
            .filter_map(|id| kb.get_keys(id).into_iter().next().map(str::to_string))
            .collect()
    })
}

fn push_help_item(parts: &mut Vec<String>, keys: &[String], label: &str, label_first: bool) {
    let text = format_help_keys(keys);
    if text.is_empty() {
        parts.push(label.to_string());
        return;
    }
    if label_first {
        parts.push(format!("{label} {text}"));
    } else {
        parts.push(format!("{text} {label}"));
    }
}

fn format_help_keys(keys: &[String]) -> String {
    if keys.is_empty() {
        return String::new();
    }
    let pretty: Vec<String> = keys.iter().map(|k| pretty_key_id(k)).collect();
    compact_raw_keys(&pretty)
}

fn pretty_key_id(key_id: &str) -> String {
    let (prefix, suffix) = match key_id.rfind('+') {
        Some(i) => (&key_id[..=i], &key_id[i + 1..]),
        None => ("", key_id),
    };
    let mapped = match suffix {
        "up" => "↑",
        "down" => "↓",
        "left" => "←",
        "right" => "→",
        "pageUp" => "pgup",
        "pageDown" => "pgdn",
        other => other,
    };
    format!("{prefix}{mapped}")
}

fn compact_raw_keys(keys: &[String]) -> String {
    if keys.len() == 1 {
        return keys[0].clone();
    }
    let parts: Vec<(String, String)> = keys
        .iter()
        .map(|key| match key.rfind('+') {
            Some(i) => (key[..=i].to_string(), key[i + 1..].to_string()),
            None => (String::new(), key.clone()),
        })
        .collect();
    let prefix = parts[0].0.clone();
    if !prefix.is_empty() && parts.iter().all(|p| p.0 == prefix) {
        format!(
            "{prefix}{}",
            parts
                .iter()
                .map(|p| p.1.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )
    } else {
        keys.join("/")
    }
}

/// Wrap a muted help line to `width` on ` · ` boundaries (pi TreeHelp).
pub(crate) fn wrap_help_line(line: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    if visible_width(line) <= width {
        return vec![line.to_string()];
    }
    let indent = "  ";
    let sep = " · ";
    let items: Vec<&str> = line
        .trim_start()
        .split(" · ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut out = Vec::new();
    let mut current = String::new();
    for item in items {
        let candidate = if current.is_empty() {
            let with_indent = format!("{indent}{item}");
            if visible_width(&with_indent) <= width {
                with_indent
            } else {
                item.to_string()
            }
        } else {
            format!("{current}{sep}{item}")
        };
        if current.is_empty() || visible_width(&candidate) <= width {
            current = candidate;
            continue;
        }
        out.push(current);
        let next = format!("{indent}{item}");
        current = if visible_width(&next) <= width {
            next
        } else {
            item.to_string()
        };
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![truncate_to_width(line, width, "", false)]
    } else {
        out
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

    #[test]
    fn filter_mode_cycle_backward_from_default_is_all() {
        assert_eq!(FilterMode::Default.cycle_backward(), FilterMode::All);
        assert_eq!(FilterMode::NoTools.cycle_backward(), FilterMode::Default);
        assert_eq!(FilterMode::All.cycle(), FilterMode::Default);
    }

    #[test]
    fn tree_search_line_echoes_query() {
        assert!(tree_search_line("").contains("Type to search"));
        assert!(tree_search_line("foo").contains("Search: foo"));
    }

    #[test]
    fn tree_help_line_includes_filters_and_cycle() {
        let help = tree_help_line();
        assert!(help.contains("filters"), "got: {help}");
        assert!(help.contains("cycle"), "got: {help}");
        assert!(
            help.contains("fold/unfold") || help.contains("fold"),
            "fold/unfold purpose missing; got: {help}"
        );
        assert!(help.contains("move") || help.contains('↑'), "got: {help}");
    }
}
