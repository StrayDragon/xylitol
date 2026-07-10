//! Generic tree selector (pi TreeList / TreeSelector inspired).
//!
//! Flatten keeps single-child chains flat and bumps indent only at branch points.
//! Product FilterMode is not hard-coded — use [`TreeSelectorOptions::include_node`].

use crate::keybindings::with_keybindings;
use crate::tui::{Component, InputEvent};
use crate::utils::{truncate_to_width, visible_width};

/// One node in an application-supplied tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn with_child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_children(mut self, children: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(children);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GutterInfo {
    /// Display-indent level where the ancestor connector was shown.
    pub position: usize,
    /// `true` → draw `│`, `false` → spaces.
    pub show: bool,
}

/// Flattened row used for navigation and paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatNode {
    pub id: String,
    pub label: String,
    pub indent: usize,
    pub show_connector: bool,
    pub is_last: bool,
    pub gutters: Vec<GutterInfo>,
    pub is_virtual_root_child: bool,
    /// Parent id in the original tree (for filter re-parenting).
    pub parent_id: Option<String>,
}

pub type TreeNodePredicate = Box<dyn Fn(&TreeNode) -> bool>;

/// Theme closures for tree rows.
pub struct TreeSelectorTheme {
    pub cursor: Box<dyn Fn(&str) -> String>,
    pub prefix: Box<dyn Fn(&str) -> String>,
    pub label: Box<dyn Fn(&str) -> String>,
    pub selected_row: Box<dyn Fn(&str) -> String>,
    pub active_marker: Box<dyn Fn(&str) -> String>,
    pub scroll_info: Box<dyn Fn(&str) -> String>,
    pub empty: Box<dyn Fn(&str) -> String>,
}

impl Default for TreeSelectorTheme {
    fn default() -> Self {
        Self {
            cursor: Box::new(|s| format!("\x1b[36m{s}\x1b[39m")),
            prefix: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            label: Box::new(|s| s.to_string()),
            selected_row: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
            active_marker: Box::new(|s| format!("\x1b[36m{s}\x1b[39m")),
            scroll_info: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            empty: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
        }
    }
}

pub struct TreeSelectorOptions {
    pub max_visible: usize,
    /// When `false`, use ASCII `|--` / `` `-- `` connectors.
    pub unicode_connectors: bool,
    /// Optional filter; default keeps all nodes.
    pub include_node: Option<TreeNodePredicate>,
    /// Highlight path from root to this id with an active marker.
    pub active_id: Option<String>,
    /// Optional host label appended after `(i/n)` (e.g. `[no-tools]`).
    pub status_suffix: Option<String>,
}

impl Default for TreeSelectorOptions {
    fn default() -> Self {
        Self {
            max_visible: 12,
            unicode_connectors: true,
            include_node: None,
            active_id: None,
            status_suffix: None,
        }
    }
}

pub struct TreeSelector {
    roots: Vec<TreeNode>,
    flat: Vec<FlatNode>,
    filtered: Vec<FlatNode>,
    selected_index: usize,
    multiple_roots: bool,
    active_path_ids: std::collections::HashSet<String>,
    theme: TreeSelectorTheme,
    options: TreeSelectorOptions,
    /// Incremental label search (AND with [`TreeSelectorOptions::include_node`]).
    search_query: String,
    pub on_select: Option<Box<dyn FnMut(String)>>,
    pub on_cancel: Option<Box<dyn FnMut()>>,
}

impl TreeSelector {
    pub fn new(
        roots: Vec<TreeNode>,
        theme: TreeSelectorTheme,
        options: TreeSelectorOptions,
    ) -> Self {
        let mut sel = Self {
            roots,
            flat: Vec::new(),
            filtered: Vec::new(),
            selected_index: 0,
            multiple_roots: false,
            active_path_ids: std::collections::HashSet::new(),
            theme,
            options,
            search_query: String::new(),
            on_select: None,
            on_cancel: None,
        };
        sel.rebuild();
        sel
    }

    pub fn set_roots(&mut self, roots: Vec<TreeNode>) {
        self.roots = roots;
        self.rebuild();
    }

    pub fn set_include_node(&mut self, pred: Option<TreeNodePredicate>) {
        self.options.include_node = pred;
        self.apply_filter();
        self.clamp_selection();
    }

    pub fn set_status_suffix(&mut self, suffix: Option<String>) {
        self.options.status_suffix = suffix;
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
        self.apply_filter();
        self.clamp_selection();
    }

    /// Clear search if non-empty. Returns `true` when a query was cleared.
    pub fn clear_search_if_any(&mut self) -> bool {
        if self.search_query.is_empty() {
            return false;
        }
        self.search_query.clear();
        self.apply_filter();
        self.clamp_selection();
        true
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.filtered
            .get(self.selected_index)
            .map(|n| n.id.as_str())
    }

    pub fn filtered_ids(&self) -> Vec<&str> {
        self.filtered.iter().map(|n| n.id.as_str()).collect()
    }

    pub fn filtered_nodes(&self) -> &[FlatNode] {
        &self.filtered
    }

    fn clamp_selection(&mut self) {
        if self.filtered.is_empty() {
            self.selected_index = 0;
            return;
        }
        self.selected_index = self
            .selected_index
            .min(self.filtered.len().saturating_sub(1));
    }

    fn rebuild(&mut self) {
        self.multiple_roots = self.roots.len() > 1;
        self.flat = flatten_tree(&self.roots, self.options.active_id.as_deref());
        self.build_active_path();
        self.apply_filter();
        if let Some(active) = self.options.active_id.as_deref() {
            self.selected_index = find_nearest_visible_index(&self.flat, &self.filtered, active);
        } else {
            self.clamp_selection();
        }
    }

    fn build_active_path(&mut self) {
        self.active_path_ids.clear();
        let Some(leaf) = self.options.active_id.as_deref() else {
            return;
        };
        let by_id: std::collections::HashMap<&str, &FlatNode> =
            self.flat.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut cur = Some(leaf);
        while let Some(id) = cur {
            self.active_path_ids.insert(id.to_string());
            cur = by_id.get(id).and_then(|n| n.parent_id.as_deref());
        }
    }

    fn apply_filter(&mut self) {
        let tokens: Vec<String> = self
            .search_query
            .to_lowercase()
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect();
        self.filtered = self
            .flat
            .iter()
            .filter(|flat| {
                if let Some(ref pred) = self.options.include_node
                    && !find_node(&self.roots, &flat.id).is_some_and(pred)
                {
                    return false;
                }
                if tokens.is_empty() {
                    return true;
                }
                let label = flat.label.to_lowercase();
                tokens.iter().all(|t| label.contains(t))
            })
            .cloned()
            .collect();
        recalculate_visual_structure(&mut self.filtered, &self.flat);
        let visible_roots = self
            .filtered
            .iter()
            .filter(|n| {
                n.parent_id
                    .as_ref()
                    .is_none_or(|p| !self.filtered.iter().any(|o| &o.id == p))
            })
            .count();
        self.multiple_roots = visible_roots > 1;
    }

    fn render_prefix(&self, flat: &FlatNode) -> String {
        let display_indent = if self.multiple_roots {
            flat.indent.saturating_sub(1)
        } else {
            flat.indent
        };
        let (branch_mid, branch_last, pipe) = if self.options.unicode_connectors {
            ("├", "└", "│")
        } else {
            ("|", "`", "|")
        };
        let connector = if flat.show_connector && !flat.is_virtual_root_child {
            if flat.is_last {
                format!("{branch_last}─ ")
            } else {
                format!("{branch_mid}─ ")
            }
        } else {
            String::new()
        };
        let connector_position = if connector.is_empty() {
            usize::MAX
        } else {
            display_indent.saturating_sub(1)
        };
        let total_chars = display_indent.saturating_mul(3);
        let mut prefix_chars = Vec::with_capacity(total_chars);
        for i in 0..total_chars {
            let level = i / 3;
            let pos_in_level = i % 3;
            if let Some(gutter) = flat.gutters.iter().find(|g| g.position == level) {
                if pos_in_level == 0 {
                    prefix_chars.push(if gutter.show { pipe } else { " " });
                } else {
                    prefix_chars.push(" ");
                }
            } else if !connector.is_empty() && level == connector_position {
                match pos_in_level {
                    0 => prefix_chars.push(if flat.is_last {
                        branch_last
                    } else {
                        branch_mid
                    }),
                    1 => prefix_chars.push("─"),
                    _ => prefix_chars.push(" "),
                }
            } else {
                prefix_chars.push(" ");
            }
        }
        prefix_chars.concat()
    }
}

type FlattenStackItem<'a> = (
    &'a TreeNode,
    usize,
    bool,
    bool,
    bool,
    Vec<GutterInfo>,
    bool,
    Option<String>,
);

type RecalcStackItem = (String, usize, bool, bool, bool, Vec<GutterInfo>, bool);

fn child_indent_for(indent: usize, multiple_children: bool, just_branched: bool) -> usize {
    if multiple_children || (just_branched && indent > 0) {
        indent + 1
    } else {
        indent
    }
}

/// Public flatten helper for unit tests.
pub fn flatten_tree(roots: &[TreeNode], active_id: Option<&str>) -> Vec<FlatNode> {
    let mut result = Vec::new();
    let contains_active = mark_contains_active(roots, active_id);

    let multiple_roots = roots.len() > 1;
    let mut ordered_roots: Vec<&TreeNode> = roots.iter().collect();
    ordered_roots.sort_by_key(|n| !contains_active.contains(n.id.as_str()));

    let mut stack: Vec<FlattenStackItem<'_>> = Vec::new();

    for (i, root) in ordered_roots.iter().rev().enumerate() {
        let is_last = i == 0;
        stack.push((
            root,
            if multiple_roots { 1 } else { 0 },
            multiple_roots,
            multiple_roots,
            is_last,
            Vec::new(),
            multiple_roots,
            None,
        ));
    }

    while let Some((
        node,
        indent,
        just_branched,
        show_connector,
        is_last,
        gutters,
        is_virtual,
        parent_id,
    )) = stack.pop()
    {
        result.push(FlatNode {
            id: node.id.clone(),
            label: node.label.clone(),
            indent,
            show_connector,
            is_last,
            gutters: gutters.clone(),
            is_virtual_root_child: is_virtual,
            parent_id,
        });

        let multiple_children = node.children.len() > 1;
        let mut ordered_children: Vec<&TreeNode> = node.children.iter().collect();
        ordered_children.sort_by_key(|n| !contains_active.contains(n.id.as_str()));

        let child_indent = child_indent_for(indent, multiple_children, just_branched);

        let connector_displayed = show_connector && !is_virtual;
        let current_display_indent = if multiple_roots {
            indent.saturating_sub(1)
        } else {
            indent
        };
        let connector_position = current_display_indent.saturating_sub(1);
        let child_gutters: Vec<GutterInfo> = if connector_displayed {
            let mut g = gutters;
            g.push(GutterInfo {
                position: connector_position,
                show: !is_last,
            });
            g
        } else {
            gutters
        };

        for (i, child) in ordered_children.iter().rev().enumerate() {
            let child_is_last = i == 0;
            stack.push((
                child,
                child_indent,
                multiple_children,
                multiple_children,
                child_is_last,
                child_gutters.clone(),
                false,
                Some(node.id.clone()),
            ));
        }
    }

    result
}

fn mark_contains_active(
    roots: &[TreeNode],
    active_id: Option<&str>,
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    let Some(active) = active_id else {
        return set;
    };
    fn walk(node: &TreeNode, active: &str, set: &mut std::collections::HashSet<String>) -> bool {
        let mut has = node.id == active;
        for child in &node.children {
            if walk(child, active, set) {
                has = true;
            }
        }
        if has {
            set.insert(node.id.clone());
        }
        has
    }
    for root in roots {
        walk(root, active, &mut set);
    }
    set
}

fn find_node<'a>(roots: &'a [TreeNode], id: &str) -> Option<&'a TreeNode> {
    for root in roots {
        if root.id == id {
            return Some(root);
        }
        if let Some(n) = find_node(&root.children, id) {
            return Some(n);
        }
    }
    None
}

fn find_nearest_visible_index(flat: &[FlatNode], filtered: &[FlatNode], target: &str) -> usize {
    if filtered.is_empty() {
        return 0;
    }
    let visible: std::collections::HashMap<&str, usize> = filtered
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let by_id: std::collections::HashMap<&str, &FlatNode> =
        flat.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut cur = Some(target);
    while let Some(id) = cur {
        if let Some(&idx) = visible.get(id) {
            return idx;
        }
        cur = by_id.get(id).and_then(|n| n.parent_id.as_deref());
    }
    filtered.len() - 1
}

fn recalculate_visual_structure(filtered: &mut [FlatNode], flat: &[FlatNode]) {
    if filtered.is_empty() {
        return;
    }
    let visible_ids: std::collections::HashSet<&str> =
        filtered.iter().map(|n| n.id.as_str()).collect();
    let entry_map: std::collections::HashMap<&str, &FlatNode> =
        flat.iter().map(|n| (n.id.as_str(), n)).collect();

    let find_visible_ancestor = |node_id: &str| -> Option<String> {
        let mut current = entry_map.get(node_id).and_then(|n| n.parent_id.clone());
        while let Some(id) = current {
            if visible_ids.contains(id.as_str()) {
                return Some(id);
            }
            current = entry_map.get(id.as_str()).and_then(|n| n.parent_id.clone());
        }
        None
    };

    let mut visible_parent: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut visible_children: std::collections::HashMap<Option<String>, Vec<String>> =
        std::collections::HashMap::new();
    visible_children.insert(None, Vec::new());

    for node in filtered.iter() {
        let ancestor = find_visible_ancestor(&node.id);
        visible_parent.insert(node.id.clone(), ancestor.clone());
        visible_children
            .entry(ancestor)
            .or_default()
            .push(node.id.clone());
    }

    let visible_root_ids = visible_children.get(&None).cloned().unwrap_or_default();
    let multiple_roots = visible_root_ids.len() > 1;

    let filtered_map: std::collections::HashMap<String, usize> = filtered
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    // Stack: id, indent, just_branched, show_connector, is_last, gutters, is_virtual
    let mut stack: Vec<RecalcStackItem> = Vec::new();
    for (i, id) in visible_root_ids.iter().rev().enumerate() {
        let is_last = i == 0;
        stack.push((
            id.clone(),
            if multiple_roots { 1 } else { 0 },
            multiple_roots,
            multiple_roots,
            is_last,
            Vec::new(),
            multiple_roots,
        ));
    }

    while let Some((node_id, indent, just_branched, show_connector, is_last, gutters, is_virtual)) =
        stack.pop()
    {
        let Some(&idx) = filtered_map.get(&node_id) else {
            continue;
        };
        {
            let flat_node = &mut filtered[idx];
            flat_node.indent = indent;
            flat_node.show_connector = show_connector;
            flat_node.is_last = is_last;
            flat_node.gutters = gutters.clone();
            flat_node.is_virtual_root_child = is_virtual;
        }

        let children = visible_children
            .get(&Some(node_id.clone()))
            .cloned()
            .unwrap_or_default();
        let multiple_children = children.len() > 1;
        let child_indent = child_indent_for(indent, multiple_children, just_branched);
        let connector_displayed = show_connector && !is_virtual;
        let current_display_indent = if multiple_roots {
            indent.saturating_sub(1)
        } else {
            indent
        };
        let connector_position = current_display_indent.saturating_sub(1);
        let child_gutters = if connector_displayed {
            let mut g = gutters;
            g.push(GutterInfo {
                position: connector_position,
                show: !is_last,
            });
            g
        } else {
            gutters
        };

        for (i, child_id) in children.iter().rev().enumerate() {
            let child_is_last = i == 0;
            stack.push((
                child_id.clone(),
                child_indent,
                multiple_children,
                multiple_children,
                child_is_last,
                child_gutters.clone(),
                false,
            ));
        }
    }
}

impl Component for TreeSelector {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.filtered.is_empty() {
            lines.push(truncate_to_width(
                &(self.theme.empty)("  No entries found"),
                width,
                "",
                false,
            ));
            let mut scroll = "  (0/0)".to_string();
            if let Some(ref suffix) = self.options.status_suffix {
                scroll.push(' ');
                scroll.push_str(suffix);
            }
            lines.push(truncate_to_width(
                &(self.theme.scroll_info)(&scroll),
                width,
                "",
                false,
            ));
            return lines;
        }

        let max_visible = self.options.max_visible.max(1);
        let start = self
            .selected_index
            .saturating_sub(max_visible / 2)
            .min(self.filtered.len().saturating_sub(max_visible));
        let end = (start + max_visible).min(self.filtered.len());

        for i in start..end {
            let flat = &self.filtered[i];
            let is_selected = i == self.selected_index;
            let cursor = if is_selected {
                (self.theme.cursor)("› ")
            } else {
                "  ".to_string()
            };
            let prefix = (self.theme.prefix)(&self.render_prefix(flat));
            let marker = if self.active_path_ids.contains(&flat.id) {
                (self.theme.active_marker)("• ")
            } else {
                String::new()
            };
            let label = (self.theme.label)(&flat.label);
            let mut line = format!("{cursor}{prefix}{marker}{label}");
            if is_selected {
                line = (self.theme.selected_row)(&line);
            }
            // Pad then truncate so reverse selection spans useful width.
            let pad = width.saturating_sub(visible_width(&line));
            if pad > 0 && is_selected {
                line = (self.theme.selected_row)(&format!("{line}{}", " ".repeat(pad)));
            } else if pad > 0 {
                line = format!("{line}{}", " ".repeat(pad));
            }
            lines.push(truncate_to_width(&line, width, "", false));
        }

        let mut scroll = format!("  ({}/{})", self.selected_index + 1, self.filtered.len());
        if let Some(ref suffix) = self.options.status_suffix {
            scroll.push(' ');
            scroll.push_str(suffix);
        }
        lines.push(truncate_to_width(
            &(self.theme.scroll_info)(&scroll),
            width,
            "",
            false,
        ));
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        let InputEvent::Key(ref key) = event else {
            return;
        };
        let up = with_keybindings(|kb| kb.matches_event(key, "tui.select.up"));
        let down = with_keybindings(|kb| kb.matches_event(key, "tui.select.down"));
        let page_up = with_keybindings(|kb| kb.matches_event(key, "tui.select.pageUp"));
        let page_down = with_keybindings(|kb| kb.matches_event(key, "tui.select.pageDown"));
        let confirm = with_keybindings(|kb| kb.matches_event(key, "tui.select.confirm"));
        let cancel = with_keybindings(|kb| kb.matches_event(key, "tui.select.cancel"));
        let backspace = crate::keys::matches_key_event(key, "backspace");

        if cancel {
            if self.clear_search_if_any() {
                return;
            }
            if let Some(ref mut cb) = self.on_cancel {
                cb();
            }
            return;
        }

        if backspace {
            if !self.search_query.is_empty() {
                self.search_query.pop();
                self.apply_filter();
                self.clamp_selection();
            }
            return;
        }

        if let Some(ch) = crate::keys::printable_from_key_event(key) {
            // Ignore bare space-only spam at start; still allow spaces inside query.
            if ch == " " && self.search_query.is_empty() {
                return;
            }
            self.search_query.push_str(&ch);
            self.apply_filter();
            self.clamp_selection();
            return;
        }

        let len = self.filtered.len();
        if len == 0 {
            return;
        }

        if up {
            self.selected_index = if self.selected_index == 0 {
                len - 1
            } else {
                self.selected_index - 1
            };
        } else if down {
            self.selected_index = if self.selected_index + 1 >= len {
                0
            } else {
                self.selected_index + 1
            };
        } else if page_up {
            self.selected_index = self
                .selected_index
                .saturating_sub(self.options.max_visible.max(1));
        } else if page_down {
            self.selected_index =
                (self.selected_index + self.options.max_visible.max(1)).min(len - 1);
        } else if confirm
            && let Some(id) = self.selected_id().map(str::to_string)
            && let Some(ref mut cb) = self.on_select
        {
            cb(id);
        }
    }

    fn invalidate(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_branch() -> Vec<TreeNode> {
        vec![
            TreeNode::new("r", "root").with_children([
                TreeNode::new("a", "A").with_child(TreeNode::new("a1", "A1")),
                TreeNode::new("b", "B")
                    .with_children([TreeNode::new("b1", "B1"), TreeNode::new("b2", "B2")]),
            ]),
        ]
    }

    #[test]
    fn flatten_keeps_single_child_chain_flat_until_branch() {
        let flat = flatten_tree(&sample_branch(), Some("b2"));
        let by_id: std::collections::HashMap<_, _> =
            flat.iter().map(|n| (n.id.as_str(), n)).collect();
        assert_eq!(by_id["r"].indent, 0);
        // r has multiple children → a and b at indent 1 with connectors
        assert_eq!(by_id["a"].indent, 1);
        assert!(by_id["a"].show_connector);
        assert_eq!(by_id["b"].indent, 1);
        // a→a1: after branch, first generation +1 then single-child may stay
        assert!(by_id["a1"].indent >= by_id["a"].indent);
        assert_eq!(by_id["b1"].indent, by_id["b2"].indent);
        assert!(by_id["b1"].show_connector || by_id["b2"].show_connector);
    }

    #[test]
    fn filter_recalculates_sibling_indent() {
        let roots = sample_branch();
        let sel = TreeSelector::new(
            roots,
            TreeSelectorTheme::default(),
            TreeSelectorOptions {
                include_node: Some(Box::new(|n| n.id != "a" && n.id != "a1")),
                active_id: Some("b2".into()),
                ..TreeSelectorOptions::default()
            },
        );
        let ids = sel.filtered_ids();
        assert!(!ids.contains(&"a"));
        assert!(ids.contains(&"b1"));
        assert!(ids.contains(&"b2"));
        let b1 = sel.filtered_nodes().iter().find(|n| n.id == "b1").unwrap();
        let b2 = sel.filtered_nodes().iter().find(|n| n.id == "b2").unwrap();
        assert_eq!(b1.indent, b2.indent);
    }

    #[test]
    fn navigation_confirm_fires_on_select() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use std::cell::RefCell;
        use std::rc::Rc;

        let picked = Rc::new(RefCell::new(None));
        let picked_cb = picked.clone();
        let mut sel = TreeSelector::new(
            sample_branch(),
            TreeSelectorTheme::default(),
            TreeSelectorOptions::default(),
        );
        sel.on_select = Some(Box::new(move |id| {
            *picked_cb.borrow_mut() = Some(id);
        }));
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        )));
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert!(picked.borrow().is_some());
    }

    #[test]
    fn search_filters_labels_and_esc_clears() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut sel = TreeSelector::new(
            sample_branch(),
            TreeSelectorTheme::default(),
            TreeSelectorOptions::default(),
        );
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::NONE,
        )));
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('1'),
            KeyModifiers::NONE,
        )));
        assert_eq!(sel.search_query(), "b1");
        let ids = sel.filtered_ids();
        assert!(ids.contains(&"b1"));
        assert!(!ids.contains(&"a1"));
        assert!(!ids.contains(&"b2"));
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        )));
        assert!(sel.search_query().is_empty());
        assert!(sel.filtered_ids().contains(&"a1"));
    }

    #[test]
    fn left_right_page_by_max_visible() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let roots =
            vec![TreeNode::new("r", "root").with_children(
                (0..20).map(|i| TreeNode::new(format!("n{i}"), format!("node-{i}"))),
            )];
        let mut sel = TreeSelector::new(
            roots,
            TreeSelectorTheme::default(),
            TreeSelectorOptions {
                max_visible: 5,
                ..TreeSelectorOptions::default()
            },
        );
        assert_eq!(sel.selected_index, 0);
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )));
        assert_eq!(sel.selected_index, 5);
        sel.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::NONE,
        )));
        assert_eq!(sel.selected_index, 0);
    }

    #[test]
    fn status_suffix_appears_in_render() {
        let mut sel = TreeSelector::new(
            sample_branch(),
            TreeSelectorTheme::default(),
            TreeSelectorOptions {
                status_suffix: Some("[no-tools]".into()),
                ..TreeSelectorOptions::default()
            },
        );
        let lines = sel.render(80);
        let joined = lines.join("\n");
        assert!(
            joined.contains("[no-tools]") && joined.contains("/"),
            "status suffix missing: {joined}"
        );
    }
}
