//! Product TUI root layout — transcript / status / editor|tree / footer.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.

mod editor_border;
mod fixed_zone_footprint_apply;
mod mcp_slot;
mod models_slot;
mod mount;
mod pending_slot;
mod render;
mod slot_input;
mod slot_nav;
mod theme_apply;

use pending_slot::PendingSlotOps;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossterm::event::KeyEvent;
use xylitol_tui::Component;
use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    AtPathSource, ChoiceQuestion, CompletionSource, Focusable, InputEvent,
    SlashArgCompletionSource, SlashCommandSource, SystemClock, TreeNode, fg_rgb,
};

use super::dollar_skill_source::DollarSkillSource;
use super::models_picker::PendingModelChoice;
use super::slash_catalog::product_slash_commands_for_editor;

use super::session_tree::FilterMode;
use super::slots::{AskSlot, EditorSlot, EditorSlotKind, ImportSlot, ThemesSlot, TreeSlot};
use super::theme::LayoutTheme;
use crate::app::core::driver::LoadedResourcesSnapshot;
use crate::app::tui::activity_fold::{ActivityFoldState, AutoTrigger, ingest_rebuild_clocks};
use crate::app::tui::bridge::UiModel;
use crate::app::tui::session_resume::SessionResumePanel;
use crate::app::tui::widgets::{
    FoldHitTable, FoldTarget, GlyphSet, ScrollbackFold, ScrollbackPaintCache,
    footer_thinking_label, format_footer_text,
};
use crate::protocol::error::XyToolError;
use crate::protocol::model::THINKING_OFF;
use crate::protocol::session::{SessionEntry, SessionTreeTravel};

pub use super::slots::ImportConfirmDecision;

/// Root UI: loaded-resources + live scrollback + optional status + bordered editor|tree + footer.
pub struct UiRoot {
    /// Brand ASCII + Skills/MCP above scrollback (c1135).
    loaded_resources: LoadedResourcesSnapshot,
    ui_model: UiModel,
    fold: ScrollbackFold,
    /// Triangle-column hit regions for ApplicationOwned mouse (c2040).
    fold_hits: FoldHitTable,
    /// Set when a fold triangle toggle mutates state; host marks AO stale.
    fold_dirty: bool,
    /// Segment L0/L2/L3 plane (c1760); orthogonal to [`ScrollbackFold`].
    activity: ActivityFoldState,
    /// Busy-only; idle leaves this unused so status occupies 0 rows.
    status_loader: Loader,
    status_busy: bool,
    /// Gate loader frames to [`Loader::interval_ms`] (host idle_tick is ~16ms).
    loader_last_tick: Instant,
    editor: Editor,
    footer: Text,
    theme: LayoutTheme,
    glyphs: GlyphSet,
    cwd: String,
    model: String,
    /// Optional `used N|~N|? tokens` fragment (c1035); omitted when unknown/empty.
    footer_token: Option<String>,
    /// Current XyDriver thinking level mirrored for border + footer (c1150).
    thinking_level: String,
    /// When true, footer omits the thinking segment (no-thinking active model).
    footer_omit_thinking: bool,
    /// Agent-busy next-turn cue (`Next turn: …`); independent of status short-word.
    status_next_turn_cue: Option<String>,
    /// c1205: while `/reload` runs, hide mcp pending / next-turn on the right.
    suppress_status_right_cue: bool,
    /// Toast notice body + deadline (atc22); not in `UiModel.entries`.
    toast_notice: Option<(String, Instant)>,
    /// Mutually exclusive editor-zone face (ati18).
    slot: EditorSlot,
    /// Handshake with host `drain_pending_ui` (outlives the live payload).
    pending: PendingSlotOps,
    last_esc_at: Option<Instant>,
    /// `!` / `!!` prefix → success border (c492).
    bash_mode: bool,
    /// Ctrl+G stub invocation count (harness).
    external_editor_invocations: u32,
    /// `(model_id, description)` for [`SlashArgCompletionSource`] (c999).
    model_arg_catalog: Vec<(String, String)>,
    /// Root for [`AtPathSource`] (c1125); default process cwd.
    at_path_base: PathBuf,
    /// `(name, description)` for [`DollarSkillSource`] (c1130).
    dollar_skill_catalog: Vec<(String, String)>,
    /// Generation for loaded+scrollback+queue cache (ath24); bumps on content/theme/fold.
    upper_gen: u64,
    upper_cache_gen: u64,
    upper_cache_width: usize,
    upper_cache_lines: Vec<String>,
    scrollback_paint: ScrollbackPaintCache,
    /// Terminal rows from host (Fixed-Zone Footprint / atc23); soft default until first sync.
    term_rows: usize,
    /// Last paint: toast + status + editor + footer row count (ApplicationOwned dock).
    last_dock_rows: usize,
    /// Rows in toast / status / editor from last paint (ApplicationOwned mouse origin).
    last_toast_rows: usize,
    last_status_rows: usize,
    last_editor_rows: usize,
    /// ApplicationOwned copy-success cue (`Copied`, ~2s). Not toast-notice / ScrollNotice.
    copy_notice_until: Option<Instant>,
    /// Test/obs: how many times upper (loaded+scrollback+queue) was rebuilt.
    #[cfg(test)]
    upper_rebuild_count: u64,
}

impl UiRoot {
    pub fn new() -> Self {
        let theme = LayoutTheme::product_dark();
        let accent = theme.palette().accent;
        let muted = theme.palette().muted;
        let status_loader = Loader::new(
            Box::new(move |s| fg_rgb(accent, s)),
            Box::new(move |s| fg_rgb(muted, s)),
            String::new(),
            Some(LoaderIndicatorOptions::default()),
        );
        let mut editor = Editor::new(
            theme.editor_theme(),
            EditorOptions {
                padding_x: 1,
                // Match agent_demo: compact operation zone (max_vis ≈ 5 lines).
                terminal_rows: 8,
            },
            Box::new(SystemClock),
        );
        editor.set_focused(true);
        let mut root = Self {
            loaded_resources: LoadedResourcesSnapshot::default(),
            ui_model: UiModel::new(),
            fold: ScrollbackFold::default(),
            fold_hits: FoldHitTable::default(),
            fold_dirty: false,
            activity: ActivityFoldState::default(),
            status_loader,
            status_busy: false,
            loader_last_tick: Instant::now(),
            editor,
            footer: Text::new(String::new(), 0, 0),
            theme,
            glyphs: GlyphSet::from_env(),
            cwd: ".".into(),
            model: crate::app::core::bootstrap::UNSET_MODEL_DISPLAY.into(),
            footer_token: None,
            thinking_level: THINKING_OFF.into(),
            footer_omit_thinking: false,
            status_next_turn_cue: None,
            suppress_status_right_cue: false,
            toast_notice: None,
            slot: EditorSlot::Editor,
            pending: PendingSlotOps::default(),
            last_esc_at: None,
            bash_mode: false,
            external_editor_invocations: 0,
            model_arg_catalog: Vec::new(),
            at_path_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dollar_skill_catalog: Vec::new(),
            upper_gen: 0,
            upper_cache_gen: u64::MAX,
            upper_cache_width: usize::MAX,
            upper_cache_lines: Vec::new(),
            scrollback_paint: ScrollbackPaintCache::default(),
            term_rows: 24,
            last_dock_rows: 8,
            last_toast_rows: 0,
            last_status_rows: 1,
            last_editor_rows: 3,
            copy_notice_until: None,
            #[cfg(test)]
            upper_rebuild_count: 0,
        };
        root.install_completion_sources();
        root.sync_editor_border();
        root.refresh_footer();
        root
    }

    fn install_completion_sources(&mut self) {
        // Arg sources before SlashCommandSource so `/model ` / `/debug ` win.
        // Default: no bare — exact `/model` stays for slash list / c630 slot Enter.
        let mut sources: Vec<Box<dyn CompletionSource>> = Vec::new();
        #[cfg(debug_assertions)]
        {
            // Hand-test fixtures — see `app::debug_fixtures`.
            sources.push(Box::new(
                SlashArgCompletionSource::new(
                    "debug",
                    crate::app::debug_fixtures::completion_catalog(),
                )
                .with_id("debug-scene"),
            ));
        }
        sources.push(Box::new(
            SlashArgCompletionSource::new("model", self.model_arg_catalog.clone())
                .with_id("model-id"),
        ));
        // Static `/theme <dark|light|toggle>` args (c1115).
        sources.push(Box::new(
            SlashArgCompletionSource::new(
                "theme",
                vec![
                    ("dark".into(), "Dark palette (default)".into()),
                    ("light".into(), "Light palette".into()),
                    ("toggle".into(), "Toggle dark ↔ light".into()),
                ],
            )
            .with_id("theme-name"),
        ));
        // Static `/trust <self|parent|deny>` args (c1105) — same space-after-cmd probe as `/model `.
        // `self` first so Tab/Enter after `/trust ` defaults to trust cwd (same as bare `/trust`).
        sources.push(Box::new(
            SlashArgCompletionSource::new(
                "trust",
                vec![
                    (
                        "self".into(),
                        "Trust this project directory (default)".into(),
                    ),
                    ("parent".into(), "Trust parent folder".into()),
                    ("deny".into(), "Do not trust this project".into()),
                ],
            )
            .with_id("trust-mode"),
        ));
        sources.push(Box::new(SlashCommandSource::new(
            product_slash_commands_for_editor(),
        )));
        sources.push(Box::new(AtPathSource::new(self.at_path_base.clone())));
        sources.push(Box::new(DollarSkillSource::new(
            self.dollar_skill_catalog.clone(),
        )));
        self.editor.set_completion_sources(sources);
    }

    /// Override `@` path completion root (c1125; harness injects tempdir).
    pub fn set_at_path_base(&mut self, base: impl Into<PathBuf>) {
        let base = base.into();
        if self.at_path_base == base {
            return;
        }
        self.at_path_base = base;
        self.install_completion_sources();
    }

    /// Refresh `/model <id>` inline completion catalog (from `available_models`).
    pub fn set_model_arg_catalog(&mut self, catalog: Vec<(String, String)>) {
        if self.model_arg_catalog == catalog {
            return;
        }
        self.model_arg_catalog = catalog;
        self.install_completion_sources();
    }

    /// Refresh `$skill` completion catalog (from Trust-filtered loaded skills; c1130).
    pub fn set_dollar_skill_catalog(&mut self, catalog: Vec<(String, String)>) {
        if self.dollar_skill_catalog == catalog {
            return;
        }
        self.dollar_skill_catalog = catalog;
        self.install_completion_sources();
    }

    /// Replace the loaded-resources header snapshot (c1135).
    pub fn set_loaded_resources(&mut self, snap: LoadedResourcesSnapshot) {
        if self.loaded_resources == snap {
            return;
        }
        self.loaded_resources = snap;
        self.bump_upper_gen();
        self.refresh_mcp_short_cue();
    }

    /// Sync fixed MCP short cue from loaded-resources snapshot (c1210).
    ///
    /// Always right-aligned in [`render_status_slot`] (idle whole-line or busy trail).
    /// Sticky header may still show `mcp: connecting i/n` — that is the progress
    /// row; the cue only points at `/mcp` and must not sit left under the card.
    pub fn refresh_mcp_short_cue(&mut self) {
        use crate::app::core::driver::MCP_PENDING_CUE;
        if self.suppress_status_right_cue {
            self.status_next_turn_cue = None;
            return;
        }
        if self.loaded_resources.mcp_tools_pending() {
            // Keep an existing model/thinking next-turn cue when busy.
            if self.status_busy && self.status_next_turn_cue.is_some() {
                let cue = self.status_next_turn_cue.as_deref().unwrap_or("");
                if cue.starts_with("Next turn") {
                    return;
                }
            }
            self.status_next_turn_cue = Some(MCP_PENDING_CUE.to_string());
        } else if self
            .status_next_turn_cue
            .as_deref()
            .is_some_and(|c| c == MCP_PENDING_CUE)
        {
            self.status_next_turn_cue = None;
        }
    }

    /// Inject footer identity (cwd · model). Call before first render when known.
    pub fn set_layout_meta(&mut self, cwd: impl Into<String>, model: impl Into<String>) {
        self.cwd = cwd.into();
        self.model = model.into();
        self.refresh_footer();
    }

    /// Set or clear the provenance-honest token usage fragment (c1035).
    pub fn set_footer_token_label(&mut self, label: Option<String>) {
        self.footer_token = label.filter(|s| !s.is_empty());
        self.refresh_footer();
    }

    pub fn set_glyphs(&mut self, glyphs: GlyphSet) {
        self.glyphs = glyphs;
    }

    pub fn glyphs(&self) -> GlyphSet {
        self.glyphs
    }

    pub fn fold(&self) -> ScrollbackFold {
        self.fold.clone()
    }

    pub fn activity(&self) -> &ActivityFoldState {
        &self.activity
    }

    pub fn activity_mut(&mut self) -> &mut ActivityFoldState {
        &mut self.activity
    }

    pub fn set_activity_settings(
        &mut self,
        settings: crate::app::tui::activity_fold::ActivityFoldSettings,
    ) {
        self.activity.settings = settings;
    }

    /// Test/harness: mutate activity then invalidate upper paint cache.
    pub fn touch_activity(&mut self) {
        self.bump_upper_gen();
    }

    /// C1: expand nearest L2/L3 segment one step toward L0 (att28). Silent if none.
    pub fn expand_nearest_activity(&mut self) -> bool {
        let changed = self.activity.expand_nearest(&self.ui_model.entries);
        if changed {
            self.bump_upper_gen();
        }
        changed
    }

    /// C1: collapse nearest eligible L0 Activity one step (att28). Silent if none.
    pub fn collapse_nearest_activity(&mut self) -> bool {
        let changed = self.activity.collapse_nearest(&self.ui_model.entries);
        if changed {
            self.bump_upper_gen();
        }
        changed
    }

    /// Rebuild path: ingest wall-clock stamps + auto crush older segments (att26).
    pub fn apply_activity_after_rebuild(
        &mut self,
        session_entries: &[SessionEntry],
        travel: &SessionTreeTravel,
    ) {
        ingest_rebuild_clocks(
            &mut self.activity,
            &self.ui_model.entries,
            session_entries,
            travel,
        );
        if self
            .activity
            .auto_degrade(&self.ui_model.entries, AutoTrigger::Rebuild, false)
        {
            self.bump_upper_gen();
        }
    }

    /// Turn-end auto crush (att26). Caller MUST only invoke when the turn is idle/ended.
    pub fn apply_activity_after_turn_end(&mut self) {
        if self
            .activity
            .auto_degrade(&self.ui_model.entries, AutoTrigger::TurnEnd, false)
        {
            self.bump_upper_gen();
        }
    }

    pub fn fold_hits(&self) -> &FoldHitTable {
        &self.fold_hits
    }

    /// Read-only fold state view (att20–att31 headless asserts; consumed by
    /// [`crate::app::tui::InteractionBdd`]). Borrowing twin of [`Self::fold`]
    /// (which clones) — callers comparing frames between toggles prefer this.
    pub fn fold_view(&self) -> &ScrollbackFold {
        &self.fold
    }

    /// Left click at screen `(col, row)` through the product fold hit table —
    /// the single toggle pipeline shared by the host hit-priority wiring and
    /// BDD mouse injection (att20 / att22 / att29–att32).
    pub fn click_fold_at(&mut self, col: u16, row: u16) -> bool {
        let Some(target) = self.fold_hits.hit(col, row) else {
            return false;
        };
        self.toggle_fold_target(target);
        true
    }

    /// Product input routing for a decoded key event (chord →
    /// [`crate::app::tui::keybindings`] ids → fold/slot effects). Kept off the
    /// external API surface; the headless interaction fixture is in-tree.
    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        self.handle_slot_input(InputEvent::Key(key));
    }

    pub fn sync_fold_hit_viewport(&mut self, scroll_top: usize, transcript_rows: u16) {
        self.fold_hits.scroll_top = scroll_top;
        self.fold_hits.transcript_rows = transcript_rows;
    }

    /// Fold toggle from mouse hit (L1 / Compaction / OutputViewport / Segment).
    /// Does **not** clear the whole paint cache — entry fingerprints carry
    /// effective fold (ath25). Compaction MUST NOT clear tools overrides.
    /// Segment is one-step level toggle (att31), not nearest expand/collapse.
    pub fn toggle_fold_target(&mut self, target: FoldTarget) {
        match target {
            FoldTarget::Tool(id) | FoldTarget::Diff(id) | FoldTarget::Ask(id) => {
                self.fold.toggle_tools(&id);
            }
            FoldTarget::Thinking(id) => {
                self.fold.toggle_thinking(&id);
            }
            FoldTarget::Todo => {
                self.fold.todo_expanded = !self.fold.todo_expanded;
            }
            FoldTarget::Compaction => {
                self.fold.compaction_expanded = !self.fold.compaction_expanded;
            }
            FoldTarget::OutputViewport => {
                self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
            }
            FoldTarget::Segment(id) => {
                let _ = self.activity.toggle_one_step(&id);
            }
            FoldTarget::Cluster(id) => {
                let _ = self.activity.toggle_cluster(&id);
            }
            FoldTarget::LiveTail => {
                let _ = self.activity.expand_live_cluster(&self.ui_model.entries);
            }
        }
        self.fold_dirty = true;
        self.bump_upper_gen();
    }

    /// Consume fold-dirty edge so the host can `mark_ao_components_stale`.
    pub fn take_fold_dirty(&mut self) -> bool {
        std::mem::take(&mut self.fold_dirty)
    }

    /// ApplicationOwned dock rows from the last [`Component::render`].
    pub(crate) fn last_dock_rows(&self) -> usize {
        self.last_dock_rows.max(1)
    }

    /// Mouse/key paint policy for the focused editor (ApplicationOwned selection / typing).
    pub(crate) fn editor_wants_rerender(&self, event: &xylitol_tui::InputEvent) -> bool {
        Component::input_wants_rerender(&self.editor, event)
    }

    /// Arm ApplicationOwned «Copied» fixed-zone cue (~2s). Must not use Error: toast (ath31).
    pub fn arm_copy_notice(&mut self) {
        self.copy_notice_until = Some(Instant::now() + xylitol_tui::COPY_NOTICE_TTL);
    }

    /// Whether the ApplicationOwned copy cue is still within TTL.
    pub fn copy_notice_visible(&self) -> bool {
        self.copy_notice_until
            .is_some_and(|until| Instant::now() < until)
    }

    /// Test helper: visible copy-notice body when armed.
    #[cfg(test)]
    pub fn copy_notice_body_for_test(&self) -> Option<&'static str> {
        self.copy_notice_visible().then_some("Copied")
    }

    /// Test helper: Editor absolute screen origin (ApplicationOwned mouse hit-test).
    #[cfg(test)]
    pub fn editor_screen_origin_for_test(&self) -> (u16, u16) {
        self.editor.screen_origin()
    }

    /// Pre-paint estimate when no frame has measured dock yet.
    pub(crate) fn estimate_dock_rows(&self) -> usize {
        let queue_rows = if self.ui_model.pending_steer.is_empty()
            && self.ui_model.pending_follow_up.is_empty()
        {
            0
        } else {
            // Steering/Follow-up lines + Alt+Up hint (see render_queue_strip).
            self.ui_model.pending_steer.len() + self.ui_model.pending_follow_up.len() + 1
        };
        crate::app::tui::layout::reserved_lower_fixed_zone(
            self.status_busy,
            queue_rows,
            self.toast_notice.is_some(),
        )
        .saturating_add(4) // editor borders + body floor
        .max(4)
    }

    pub fn editor_text(&self) -> String {
        // Send / history / Ctrl+G paths: expand [paste #N …] markers (c1160 / ati34).
        self.editor.get_expanded_text()
    }

    /// Display buffer (may contain `[paste #N …]` collapse markers).
    pub fn editor_display_text(&self) -> String {
        self.editor.get_text()
    }

    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
        self.sync_editor_border();
    }

    /// Insert text at the editor cursor (clipboard image path paste, c1155).
    pub fn insert_editor_text_at_cursor(&mut self, text: &str) {
        self.editor.insert_text_at_cursor(text);
        self.sync_editor_border();
    }

    /// Autocomplete popup open (host must not steal Enter before confirm).
    pub fn editor_autocomplete_open(&self) -> bool {
        self.editor.is_showing_autocomplete()
    }

    /// Apply highlighted completion into the editor buffer (no submit).
    pub fn confirm_editor_autocomplete(&mut self) -> bool {
        let ok = self.editor.confirm_autocomplete_selection();
        if ok {
            self.sync_editor_border();
        }
        ok
    }

    /// Whether the editor is in bash accent mode (`!` / `!!` prefix).
    pub fn bash_mode(&self) -> bool {
        self.bash_mode
    }

    pub fn external_editor_invocations(&self) -> u32 {
        self.external_editor_invocations
    }

    /// Ctrl+G stub: count + optional `# $EDITOR stub` marker (harness-safe).
    /// Uses expanded text so `[paste #N …]` markers become real content (ati34 / pi getExpandedText).
    pub fn open_external_editor_stub(&mut self) {
        self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
        let text = self.editor.get_expanded_text();
        if text.is_empty() {
            self.editor.set_text("# $EDITOR stub\n".to_string());
        } else if !text.contains("$EDITOR stub") {
            self.editor
                .set_text(format!("{}\n# $EDITOR stub", text.trim_end()));
        }
        self.sync_editor_border();
    }

    /// Record a sent prompt / steer / follow-up for ↑/↓ history (pi `addToHistory`).
    pub fn remember_editor_send(&mut self, text: impl Into<String>) {
        self.editor.add_to_history(text.into());
    }

    /// Replace ↑/↓ send history from session seed (c1560).
    pub fn replace_editor_send_history(&mut self, texts: impl IntoIterator<Item = String>) {
        self.editor.replace_history(texts);
    }

    pub fn slot(&self) -> EditorSlotKind {
        self.slot.kind()
    }

    pub fn tree_open(&self) -> bool {
        self.slot.is_tree()
    }

    pub fn take_pending_tree_open(&mut self) -> bool {
        std::mem::take(&mut self.pending.tree_open)
    }

    /// Request MessageHistory tree open (double Esc / `/session-tree`, c700/c1005).
    pub fn request_tree_open(&mut self) {
        self.pending.tree_open = true;
    }

    pub fn take_pending_tree_travel(&mut self) -> Option<String> {
        self.pending.tree_travel.take()
    }

    pub fn take_pending_tree_fork(&mut self) -> Option<String> {
        self.pending.tree_fork.take()
    }

    /// Queue fork for `entry_id` (`/session-fork` at leaf or Shift+F).
    pub fn request_tree_fork(&mut self, entry_id: String) {
        self.pending.tree_fork = Some(entry_id);
    }

    pub fn take_pending_tree_label(&mut self) -> Option<(String, Option<String>)> {
        self.pending.tree_label.take()
    }

    pub fn apply_tree_label(&mut self, id: &str, label: Option<String>) {
        if let EditorSlot::Tree(tree) = &mut self.slot {
            tree.apply_label(id, label);
        }
    }

    pub fn take_pending_model_select(&mut self) -> Option<PendingModelChoice> {
        self.pending.model_select.take()
    }

    pub fn models_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Models(_))
    }

    pub fn take_pending_theme_select(&mut self) -> Option<String> {
        self.pending.theme_select.take()
    }

    pub fn themes_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Themes(_))
    }

    pub fn import_confirm_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::ImportConfirm(_))
    }

    pub fn session_resume_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::SessionResume(_))
    }

    pub fn take_pending_session_resume_select(&mut self) -> Option<String> {
        self.pending.session_resume_select.take()
    }

    pub fn take_pending_session_resume_rename(&mut self) -> Option<(String, String)> {
        self.pending.session_resume_rename.take()
    }

    pub fn take_pending_session_resume_delete(&mut self) -> Option<String> {
        self.pending.session_resume_delete.take()
    }

    pub fn session_resume_apply_rename(&mut self, id: &str, name: &str) {
        if let EditorSlot::SessionResume(panel) = &mut self.slot {
            panel.apply_rename(id, name);
        }
    }

    pub fn session_resume_remove_entry(&mut self, id: &str) {
        if let EditorSlot::SessionResume(panel) = &mut self.slot {
            panel.remove_entry(id);
        }
    }

    pub fn session_resume_set_status(&mut self, msg: impl Into<String>) {
        if let EditorSlot::SessionResume(panel) = &mut self.slot {
            panel.set_status(msg);
        }
    }

    #[cfg(test)]
    pub fn session_resume_panel_text_for_test(&self, width: usize) -> String {
        match &self.slot {
            EditorSlot::SessionResume(panel) => panel.render(width).join("\n"),
            _ => String::new(),
        }
    }

    /// Mount session resume panel in the editor slot (c1065).
    pub fn mount_session_resume_picker(
        &mut self,
        entries: Vec<crate::app::core::driver::SessionListEntry>,
        current_id: Option<&str>,
    ) {
        let mut panel = SessionResumePanel::new(self.theme);
        panel.set_current_cwd(&self.cwd);
        panel.load_entries(entries, current_id.map(str::to_string));
        self.slot = EditorSlot::SessionResume(panel);
    }

    /// Placeholder while scanning session jsonl (pi loaded/total).
    pub fn mount_session_resume_loading(&mut self, loaded: usize, total: usize) {
        if let EditorSlot::SessionResume(panel) = &mut self.slot {
            panel.set_current_cwd(&self.cwd);
            panel.set_loading(loaded, total);
            return;
        }
        let mut panel = SessionResumePanel::new(self.theme);
        panel.set_current_cwd(&self.cwd);
        panel.set_loading(loaded, total);
        self.slot = EditorSlot::SessionResume(panel);
    }

    pub fn close_session_resume(&mut self) {
        if matches!(&self.slot, EditorSlot::SessionResume(_)) {
            self.close_slot();
        }
    }

    pub fn take_pending_import_decision(&mut self) -> Option<ImportConfirmDecision> {
        self.pending.import_decision.take()
    }

    /// Mount Yes/No import confirm in the editor slot (c1010).
    pub fn mount_import_confirm(&mut self, path: &str) {
        self.slot = EditorSlot::ImportConfirm(ImportSlot::mount(self.theme, path));
    }

    pub fn close_import_confirm(&mut self) {
        if matches!(&self.slot, EditorSlot::ImportConfirm(_)) {
            self.close_slot();
        }
    }

    /// Mount ChoicePrompt for builtin `ask` and park the oneshot reply (c1850).
    pub fn mount_ask_choice(
        &mut self,
        questions: Vec<ChoiceQuestion>,
        reply: tokio::sync::oneshot::Sender<Result<String, XyToolError>>,
    ) {
        if questions.is_empty() {
            let _ = reply.send(Err(XyToolError::InvalidArgs(
                "ask requires at least one question".into(),
            )));
            return;
        }
        if let EditorSlot::Choice(ref mut prev) = self.slot {
            prev.abort();
        }
        self.slot = EditorSlot::Choice(AskSlot::mount(self.theme, questions, reply));
    }

    /// If ChoicePrompt finished, complete oneshot with ask JSON and close the slot.
    ///
    /// Returns the ask payload JSON when a result was delivered.
    pub fn complete_ask_if_ready(&mut self) -> Option<String> {
        let json = {
            let EditorSlot::Choice(ask) = &mut self.slot else {
                return None;
            };
            ask.complete_if_ready()?
        };
        self.slot = EditorSlot::Editor;
        Some(json)
    }

    pub fn ask_choice_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Choice(ask) if ask.has_prompt())
    }

    /// Mount built-in theme picker in the editor slot (c1115).
    pub fn mount_themes_picker(&mut self, current: Option<&str>) {
        self.slot = EditorSlot::Themes(ThemesSlot::mount(self.theme, current));
    }

    /// Mount MessageHistory rows fetched via XyDriver and open the Tree slot.
    pub fn mount_session_tree(&mut self, roots: Vec<TreeNode>, active_id: Option<&str>) {
        self.slot = EditorSlot::Tree(TreeSlot::mount(self.theme, roots, active_id));
    }

    pub fn on_ctrl_c(&mut self, quit_flag: &AtomicBool) {
        if self.slot.is_overlay() {
            self.close_slot();
            return;
        }
        if !self.editor.get_text().is_empty() {
            self.editor.set_text(String::new());
            return;
        }
        quit_flag.store(true, Ordering::SeqCst);
    }

    /// Esc: label edit cancel → tree clears search → other overlays close; else idle empty double-Esc.
    pub fn open_session_tree_for_test(&mut self, roots: Vec<TreeNode>, active_id: Option<&str>) {
        self.editor.set_text(String::new());
        self.mount_session_tree(roots, active_id);
    }

    #[cfg(test)]
    pub fn open_session_tree_at_for_test(&mut self, roots: Vec<TreeNode>, id: &str) {
        self.editor.set_text(String::new());
        self.mount_session_tree(roots, Some(id));
        if let EditorSlot::Tree(tree) = &mut self.slot {
            let _ = tree.select_id(id);
        }
    }

    #[cfg(test)]
    pub fn open_slot_for_test(&mut self, slot: EditorSlot) {
        self.editor.set_text(String::new());
        self.open_slot(slot);
    }

    /// Push bridge UI model into status / footer; scrollback re-renders from model (c476).
    pub fn apply_ui_model(&mut self, model: &UiModel) {
        // Queue strip is part of the dock — steer/follow-up alone must not invalidate
        // the transcript upper cache (streaming frames stay cheaper).
        let upper_changed = self.ui_model.entries != model.entries
            || self.ui_model.streaming_assistant != model.streaming_assistant
            || self.ui_model.streaming_thinking != model.streaming_thinking
            || self.ui_model.streaming_think_id != model.streaming_think_id
            || self.ui_model.phase != model.phase;
        self.ui_model = model.clone();
        if upper_changed {
            self.bump_upper_gen();
        }

        match model.status.as_ref() {
            Some(s) if !s.is_empty() => {
                self.status_loader.set_message(s.clone());
                self.status_busy = true;
            }
            _ => {
                self.status_busy = false;
            }
        }
        self.refresh_mcp_short_cue();

        self.refresh_footer();
    }

    fn bump_upper_gen(&mut self) {
        self.upper_gen = self.upper_gen.saturating_add(1);
    }

    fn refresh_footer(&mut self) {
        let thinking = if self.footer_omit_thinking {
            String::new()
        } else {
            footer_thinking_label(&self.thinking_level)
        };
        let base = format_footer_text(
            &self.cwd,
            &self.model,
            &thinking,
            self.footer_token.as_deref(),
        );
        self.footer.set_text(self.theme.paint_muted(&base));
    }

    /// Apply active fixed zone for footer (model + thinking); omit thinking when not adjustable.
    pub fn set_active_fixed_zone(
        &mut self,
        model_label: impl Into<String>,
        thinking: String,
        omit_thinking: bool,
    ) {
        self.model = model_label.into();
        self.thinking_level = thinking;
        self.footer_omit_thinking = omit_thinking;
        self.sync_editor_border();
        self.refresh_footer();
    }

    /// Set or clear next-turn cue (agent-busy NextTurn pending).
    pub fn set_status_next_turn_cue(&mut self, cue: Option<String>) {
        self.status_next_turn_cue = cue.filter(|s| !s.is_empty());
        // If model/thinking cue cleared, restore MCP short cue when pending.
        if self.status_next_turn_cue.is_none() {
            self.refresh_mcp_short_cue();
        }
    }

    /// c1205: suppress right-side status cues while Reloading.
    pub fn set_suppress_status_right_cue(&mut self, suppress: bool) {
        self.suppress_status_right_cue = suppress;
        if suppress {
            self.status_next_turn_cue = None;
        } else {
            self.refresh_mcp_short_cue();
        }
    }

    /// Push / replace toast notice body (TTL from [`crate::app::tui::commands::TOAST_NOTICE_TTL`]).
    pub fn push_toast_notice(&mut self, body: impl Into<String>) {
        self.toast_notice = Some((
            body.into(),
            Instant::now() + crate::app::tui::commands::TOAST_NOTICE_TTL,
        ));
    }

    /// Body only (no `Error: ` prefix); `None` when cleared / expired.
    pub fn toast_notice_body(&self) -> Option<&str> {
        self.toast_notice.as_ref().map(|(b, _)| b.as_str())
    }

    /// Test/harness: force deadline into the past so the next `tick` clears.
    #[cfg(test)]
    pub fn expire_toast_notice_now(&mut self) {
        if let Some((body, _)) = self.toast_notice.take() {
            self.toast_notice = Some((
                body,
                Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
            ));
        }
    }

    /// Clear toast when past deadline; returns whether state changed.
    pub(super) fn clear_toast_notice_if_expired(&mut self) -> bool {
        let expired = self
            .toast_notice
            .as_ref()
            .is_some_and(|(_, d)| Instant::now() >= *d);
        if expired {
            self.toast_notice = None;
            true
        } else {
            false
        }
    }

    pub(super) fn clear_copy_notice_if_expired(&mut self) -> bool {
        let expired = self
            .copy_notice_until
            .is_some_and(|until| Instant::now() >= until);
        if expired {
            self.copy_notice_until = None;
            true
        } else {
            false
        }
    }

    /// Update Editor screen origin from the last ApplicationOwned dock measure.
    pub(crate) fn sync_editor_screen_origin(&mut self) {
        let (row, col) = xylitol_tui::editor_screen_origin(
            self.term_rows.min(u16::MAX as usize) as u16,
            self.last_dock_rows,
            self.last_toast_rows.saturating_add(self.last_status_rows),
        );
        self.editor.set_screen_origin(row, col);
    }

    /// Pending steer / follow-up strip above status (pi `pendingMessagesContainer`).
    pub fn tree_filter_for_test(&self) -> FilterMode {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.filter(),
            _ => FilterMode::Default,
        }
    }

    // ── headless interaction fixture surface (`InteractionBdd`) ──

    pub(crate) fn slot_is_tree(&self) -> bool {
        matches!(&self.slot, EditorSlot::Tree(_))
    }

    pub(crate) fn tree_select(&mut self, id: &str) -> bool {
        match &mut self.slot {
            EditorSlot::Tree(tree) => tree.select_id(id),
            _ => false,
        }
    }

    pub(crate) fn tree_label_editing(&self) -> bool {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.is_label_editing(),
            _ => false,
        }
    }

    pub(crate) fn tree_node_folded(&self, id: &str) -> bool {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.is_folded(id),
            _ => false,
        }
    }

    pub(crate) fn pending_tree_fork(&self) -> Option<String> {
        self.pending.tree_fork.clone()
    }

    pub(crate) fn pending_tree_label(&self) -> Option<(String, Option<String>)> {
        self.pending.tree_label.clone()
    }

    #[cfg(test)]
    pub fn tree_search_query_for_test(&self) -> &str {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.search_query(),
            _ => "",
        }
    }

    #[cfg(test)]
    pub fn tree_panel_text_for_test(&mut self, width: usize) -> String {
        match &mut self.slot {
            EditorSlot::Tree(tree) => tree.panel_text(width),
            _ => String::new(),
        }
    }

    /// Full Tree slot head (Search / Help) + list for harness asserts.
    #[cfg(test)]
    pub fn tree_slot_text_for_test(&mut self, width: usize) -> String {
        self.render_editor_slot(width).join("\n")
    }

    #[cfg(test)]
    pub fn tree_is_folded_for_test(&self, id: &str) -> bool {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.is_folded(id),
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn ui_model_entries_len_for_test(&self) -> usize {
        self.ui_model.entries.len()
    }

    /// Editor slot lines (incl. border SGR) for harness border asserts (c1150).
    #[cfg(test)]
    pub fn editor_render_for_test(&mut self, width: usize) -> Vec<String> {
        self.render_editor_slot(width)
    }

    #[cfg(test)]
    pub fn thinking_level_for_test(&self) -> String {
        self.thinking_level.clone()
    }

    #[cfg(test)]
    pub fn upper_rebuild_count_for_test(&self) -> u64 {
        self.upper_rebuild_count
    }

    #[cfg(test)]
    pub fn scrollback_entry_misses_for_test(&self) -> u64 {
        self.scrollback_paint.entry_misses
    }

    #[cfg(test)]
    pub fn clear_scrollback_entry_misses_for_test(&mut self) {
        self.scrollback_paint.clear_misses();
    }

    #[cfg(test)]
    pub fn streaming_assistant_full_parses_for_test(&self) -> u64 {
        self.scrollback_paint.streaming_assistant.full_parses
    }

    #[cfg(test)]
    pub fn clear_streaming_assistant_parse_counts_for_test(&mut self) {
        self.scrollback_paint.streaming_assistant.clear_counts();
    }
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub use mount::build_root;
#[cfg(test)]
pub(crate) use mount::sample_tree_nodes_for_test;
pub use mount::{install_ui_root_key_listeners, shared_ui_root_rebuild};
