//! Headless interaction mounting for BDD scenes (att20–att32 / input keys).
//!
//! Product paths only — no test double between fixture and behavior:
//! - key injection → [`UiRoot::handle_key`] → the product chord router
//!   (`app.thinking.toggle` / `app.tools.blocks` / …);
//! - mouse injection → [`UiRoot::click_fold_at`] (the same function the host
//!   hit-priority wiring calls), so BDD and host share one toggle pipeline;
//! - paint → the same `render` / ANSI-strip / [`SemanticDump`] chain
//!   [`SceneBuilder`] uses.
//!
//! Entries enter by replaying [`XyEvent`]s through [`apply_xy_event`] in a
//! [`SceneBuilder`], then moving the built model here via
//! [`SceneBuilder::into_model`].

use crossterm::event::KeyEvent;
use xylitol_tui::Component;

use crate::app::core::driver::XyEvent;
use crate::app::tui::activity_fold::scene::SemanticDump;
use crate::app::tui::activity_fold::strip_ansi_live_window;
use crate::app::tui::bridge::{UiEntry, UiModel, apply_xy_event};
use crate::app::tui::layout::UiRoot;
use crate::app::tui::widgets::{FoldHitTable, ScrollbackFold};

/// Headless keyboard/mouse interaction surface over a product `UiRoot`.
///
/// Build the transcript with [`SceneBuilder`] first, hand it over with
/// [`SceneBuilder::into_model`], then press keys / click triangles /
/// assert frames. Mouse coordinates are screen cells with `scroll_top = 0`
/// (fixtures set a tall transcript pane instead of scrolling).
pub struct InteractionBdd {
    model: UiModel,
    root: UiRoot,
}

impl InteractionBdd {
    /// Mount a built model onto a fresh product root.
    pub fn from_model(model: UiModel) -> Self {
        let mut root = UiRoot::new();
        root.apply_ui_model(&model);
        Self { model, root }
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

    /// Press a decoded key through the product chord router.
    pub fn handle_key(&mut self, key: KeyEvent) -> &mut Self {
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

    /// Plain frame + semantic dump (row chords attributed to scene entries).
    pub fn render_semantic(&mut self, width: usize) -> (String, SemanticDump) {
        let plain = self.render_plain(width);
        let dump = SemanticDump::from_product_frame(&plain, &self.model.entries);
        (plain, dump)
    }

    /// Row count of live model entries (transcript size asserts).
    pub fn entries_len(&self) -> usize {
        self.model.entries.len()
    }

    /// Live model entries for identity checks (`UiEntry` shapes).
    pub fn entries(&self) -> &[UiEntry] {
        &self.model.entries
    }
}
