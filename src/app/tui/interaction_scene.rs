//! Headless interaction mounting for BDD scenes (att20–att32 / input keys).
//!
//! Product paths only — no test double between fixture and behavior:
//! - key injection → [`UiRoot::handle_key`] → the product chord router
//!   (`app.thinking.toggle` / `app.tools.blocks` / …);
//! - mouse injection → [`UiRoot::click_fold_at`] (the same function the host
//!   hit-priority wiring calls), so BDD and host share one toggle pipeline;
//! - paint → the same `render` / ANSI-strip chain [`SceneBuilder`] uses.
//!
//! Entries enter by replaying [`XyEvent`]s through [`apply_xy_event`] in a
//! [`SceneBuilder`], then moving the built model here via
//! [`SceneBuilder::into_model`].

use crossterm::event::KeyEvent;
use xylitol_tui::Component;

use crate::app::core::driver::XyEvent;
use crate::app::tui::activity_fold::ActivityFoldSettings;
use crate::app::tui::activity_fold::strip_ansi_live_window;
use crate::app::tui::bridge::{UiEntry, UiModel, apply_xy_event};
use crate::app::tui::layout::{FilterMode, UiRoot};
use crate::app::tui::widgets::{FoldHitTable, ScrollbackFold};
use crate::protocol::session::{SessionEntry, SessionTreeTravel};
use xylitol_tui::TreeNode;

/// Headless keyboard/mouse interaction surface over a product `UiRoot`.
///
/// Build the transcript with [`SceneBuilder`](crate::app::tui::SceneBuilder)
/// first, hand it over with
/// [`SceneBuilder::into_model`](crate::app::tui::SceneBuilder::into_model),
/// then press keys / click triangles / assert frames. Mouse coordinates are screen cells with `scroll_top = 0`
/// (fixtures set a tall transcript pane instead of scrolling).
pub struct InteractionBdd {
    model: UiModel,
    root: UiRoot,
    /// Backing flag for `on_ctrl_c` (idle empty-editor Ctrl+C quits the TUI).
    quit_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl InteractionBdd {
    /// Mount a built model onto a fresh product root.
    pub fn from_model(model: UiModel) -> Self {
        let mut root = UiRoot::new();
        root.apply_ui_model(&model);
        Self {
            model,
            root,
            quit_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Tall transcript pane for hit geometry (`scroll_top = 0`): mouse rows in
    /// fixtures are content rows.
    pub fn open_hit_viewport(&mut self, transcript_rows: u16) -> &mut Self {
        self.root.sync_fold_hit_viewport(0, transcript_rows);
        self
    }

    /// Replay one more Xy event into the live model (mid-scene tool end,
    /// extra thinking flush, …) and re-project onto the root.
    pub fn push_xy(&mut self, event: XyEvent) -> &mut Self {
        apply_xy_event(&mut self.model, &event);
        self.root.apply_ui_model(&self.model);
        self
    }

    // ── activity auto-collapse (att23 / att26 / att28) ──

    /// Replace the auto-collapse knobs (product config seam, `tui.activity_fold`).
    pub fn set_activity_settings(&mut self, settings: ActivityFoldSettings) -> &mut Self {
        self.root.set_activity_settings(settings);
        self
    }

    /// Turn-end auto crush — the host calls the same root method when a turn ends.
    pub fn turn_end_activity(&mut self) -> &mut Self {
        self.root.apply_activity_after_turn_end();
        self
    }

    /// Rebuild-path wall-clock ingest + auto crush (travel / resume / fork).
    pub fn rebuild_activity(
        &mut self,
        session_entries: &[SessionEntry],
        travel: &SessionTreeTravel,
    ) -> &mut Self {
        self.root
            .apply_activity_after_rebuild(session_entries, travel);
        self
    }

    /// Press a decoded key through the product pipeline: pre-focus
    /// `app.clear` / `app.interrupt` listeners first (same order as
    /// `install_ui_root_key_listeners`), then focus/slot routing.
    pub fn handle_key(&mut self, key: KeyEvent) -> &mut Self {
        use crate::app::tui::keybindings::matches_binding;
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

    /// Left click at `(col, row)` through the shared fold-hit pipeline.
    /// Returns whether a registered target consumed the click.
    pub fn left_click(&mut self, col: u16, row: u16) -> bool {
        self.root.click_fold_at(col, row)
    }

    /// Registered fold regions (the product's mouse-geometry truth).
    pub fn fold_hits(&self) -> &FoldHitTable {
        self.root.fold_hits()
    }

    /// Fold state view: defaults, per-block overrides, effective helpers.
    pub fn fold(&self) -> &ScrollbackFold {
        self.root.fold_view()
    }

    /// Effective per-block tools-family fold state (`overrides > default`).
    pub fn tools_effective(&self, id: &str) -> bool {
        self.fold().tools_effective(id)
    }

    /// Effective per-id thinking fold state (`overrides > default`).
    pub fn thinking_effective(&self, id: &str) -> bool {
        self.fold().thinking_effective(id)
    }

    /// Painted frame lines of the current interaction state (product render).
    pub fn render_lines(&mut self, width: usize) -> Vec<String> {
        self.root.render(width)
    }

    /// Plain (ANSI-stripped) frame text of the current interaction state.
    pub fn render_plain(&mut self, width: usize) -> String {
        strip_ansi_live_window(&self.render_lines(width).join("\n"))
    }

    /// Product toast slot (status 之上). Same seam as host `push_toast_notice`.
    pub fn push_toast_notice(&mut self, body: impl Into<String>) -> &mut Self {
        self.root.push_toast_notice(body);
        self
    }

    /// Live model entries for identity checks (`UiEntry` shapes).
    pub fn entries(&self) -> &[UiEntry] {
        &self.model.entries
    }

    // ── session-tree slot scenes (ati22–ati27) ──

    /// Open the product tree slot over a supplied sample (same seam the
    /// effects use: `UiRoot::mount_session_tree`).
    pub fn mount_tree(&mut self, roots: Vec<TreeNode>, active: Option<&str>) -> &mut Self {
        self.root.mount_session_tree(roots, active);
        self
    }

    /// Whether the tree slot currently owns the editor area.
    pub fn is_tree_slot(&self) -> bool {
        self.root.slot_is_tree()
    }

    /// Active `FilterMode` as its binding-table name (`default` / `no-tools` /
    /// `user-only` / `labeled-only` / `all`); `None` outside the tree slot.
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

    /// Select a node by id (`TreeSlot::select_id`).
    pub fn select_tree_node(&mut self, id: &str) -> bool {
        self.root.tree_select(id)
    }

    /// Whether a parent node's children are folded.
    pub fn tree_node_folded(&self, id: &str) -> bool {
        self.root.tree_node_folded(id)
    }

    /// Whether a node-label edit session is open (Shift+L).
    pub fn tree_label_editing(&self) -> bool {
        self.root.tree_label_editing()
    }

    /// Fork request handed to the host pump (`app.session.fork`).
    pub fn pending_tree_fork(&self) -> Option<String> {
        self.root.pending_tree_fork()
    }

    /// Label write-back request handed to the host pump (`tui.select.confirm`
    /// inside a label edit).
    pub fn pending_tree_label(&self) -> Option<(String, Option<String>)> {
        self.root.pending_tree_label()
    }

    /// Editor display buffer (may contain `[paste #N …]` markers).
    pub fn editor_display_text(&self) -> String {
        self.root.editor_display_text()
    }
}
