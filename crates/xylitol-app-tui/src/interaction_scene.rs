use crossterm::event::KeyEvent;
use xylitol_tui::Component;

use crate::activity_fold::ActivityFoldSettings;
use crate::activity_fold::strip_ansi_live_window;
use crate::bridge::{UiEntry, UiModel, apply_xy_event};
use crate::driver::XyEvent;
use crate::layout::{FilterMode, UiRoot};
use crate::widgets::{FoldHitTable, ScrollbackFold};
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeTravel};
use xylitol_tui::TreeNode;

pub struct InteractionBdd {
    model: UiModel,
    root: UiRoot,
    quit_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl InteractionBdd {
    pub fn from_model(model: UiModel) -> Self {
        let mut root = UiRoot::new();
        root.apply_ui_model(&model);
        Self {
            model,
            root,
            quit_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn open_hit_viewport(&mut self, transcript_rows: u16) -> &mut Self {
        self.root.sync_fold_hit_viewport(0, transcript_rows);
        self
    }

    pub fn push_xy(&mut self, event: XyEvent) -> &mut Self {
        apply_xy_event(&mut self.model, &event);
        self.root.apply_ui_model(&self.model);
        self
    }

    pub fn set_activity_settings(&mut self, settings: ActivityFoldSettings) -> &mut Self {
        self.root.set_activity_settings(settings);
        self
    }

    pub fn turn_end_activity(&mut self) -> &mut Self {
        self.root.apply_activity_after_turn_end();
        self
    }

    pub fn rebuild_activity(
        &mut self,
        session_entries: &[SessionEntry],
        travel: &SessionTreeTravel,
    ) -> &mut Self {
        self.root
            .apply_activity_after_rebuild(session_entries, travel);
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> &mut Self {
        use crate::keybindings::matches_binding;
        if matches_binding(&key, "app.clear") {
            let flag = self.quit_flag.clone();
            self.root.on_ctrl_c(&flag);
            return self;
        }
        if matches_binding(&key, "app.interrupt") && self.root.on_escape() {
            return self;
        }
        self.root.handle_key(key);
        self
    }

    pub fn reset_tools_blocks(&mut self) -> &mut Self {
        self.root.reset_tools_blocks();
        self
    }

    pub fn toggle_thinking_default(&mut self) -> &mut Self {
        self.root.toggle_thinking_default();
        self
    }

    pub fn toggle_tools_output_default(&mut self) -> &mut Self {
        self.root.toggle_tools_output_default();
        self
    }

    pub fn expand_nearest_activity(&mut self) -> &mut Self {
        self.root.expand_nearest_activity();
        self
    }

    pub fn collapse_nearest_activity(&mut self) -> &mut Self {
        self.root.collapse_nearest_activity();
        self
    }

    pub fn click_todo_summary(&mut self) -> bool {
        let _ = self.root.render(80);
        self.root.click_todo_bar_local(0, 0)
    }

    pub fn todo_expanded(&self) -> bool {
        self.root.todo_bar_expanded()
    }

    pub fn render_lines_raw(&mut self, width: usize) -> Vec<String> {
        self.root.render(width)
    }

    pub fn left_click(&mut self, col: u16, row: u16) -> bool {
        self.root.click_fold_at(col, row)
    }

    pub fn fold_hits(&self) -> &FoldHitTable {
        self.root.fold_hits()
    }

    pub fn fold(&self) -> &ScrollbackFold {
        self.root.fold_view()
    }

    pub fn tools_effective(&self, id: &str) -> bool {
        self.fold().tools_effective(id)
    }

    pub fn thinking_effective(&self, id: &str) -> bool {
        self.fold().thinking_effective(id)
    }

    pub fn render_lines(&mut self, width: usize) -> Vec<String> {
        self.root.render(width)
    }

    pub fn render_plain(&mut self, width: usize) -> String {
        strip_ansi_live_window(&self.render_lines(width).join("\n"))
    }

    pub fn push_toast_notice(&mut self, body: impl Into<String>) -> &mut Self {
        self.root.push_toast_notice(body);
        self
    }

    pub fn entries(&self) -> &[UiEntry] {
        &self.model.entries
    }

    pub fn mount_tree(&mut self, roots: Vec<TreeNode>, active: Option<&str>) -> &mut Self {
        self.root.mount_session_tree(roots, active);
        self
    }

    pub fn is_tree_slot(&self) -> bool {
        self.root.slot_is_tree()
    }

    pub fn tree_filter_name(&self) -> Option<&'static str> {
        if !self.is_tree_slot() {
            return None;
        }
        Some(match self.root.tree_filter_for_test() {
            FilterMode::Default => "default",
            FilterMode::NoTools => "no-tools",
            FilterMode::UserOnly => "user-only",
            FilterMode::LabeledOnly => "labeled-only",
            FilterMode::All => "all",
        })
    }

    pub fn select_tree_node(&mut self, id: &str) -> bool {
        self.root.tree_select(id)
    }

    pub fn tree_node_folded(&self, id: &str) -> bool {
        self.root.tree_node_folded(id)
    }

    pub fn tree_label_editing(&self) -> bool {
        self.root.tree_label_editing()
    }

    pub fn pending_tree_fork(&self) -> Option<String> {
        self.root.pending_tree_fork()
    }

    pub fn pending_tree_label(&self) -> Option<(String, Option<String>)> {
        self.root.pending_tree_label()
    }

    pub fn editor_display_text(&self) -> String {
        self.root.editor_display_text()
    }
}
