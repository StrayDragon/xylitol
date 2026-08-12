//! Product TUI root layout — transcript / status / editor|tree / footer.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.

mod chrome_footprint_apply;
mod editor_border;
mod empty_widgets;
mod mcp_slot;
mod models_slot;
mod mount;
mod render;
mod slot_input;
mod slot_nav;
mod theme_apply;

use empty_widgets::{
    empty_mcp_list, empty_models_list, empty_session_resume_panel, empty_themes_list,
    empty_tree_selector, import_confirm_list as make_import_confirm_list,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::oneshot;
use xylitol_tui::Component;
use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    AtPathSource, ChoicePrompt, ChoiceQuestion, ChoiceResult, CompletionSource, Focusable, Input,
    SlashArgCompletionSource, SlashCommandSource, SystemClock, TreeNode, TreeSelector,
    TreeSelectorOptions, fg_rgb,
};

use super::dollar_skill_source::DollarSkillSource;
use super::models_picker::{ModelPickerRow, PendingModelChoice};
use super::slash_catalog::product_slash_commands_for_editor;

use super::session_tree::FilterMode;
use super::slots::EditorSlot;
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

/// User choice from `/session-import` confirm slot (c1010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportConfirmDecision {
    Accepted { path: String },
    Rejected,
}

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
    /// Chrome toast body + deadline (atc22); not in `UiModel.entries`.
    chrome_toast: Option<(String, Instant)>,
    /// Mutually exclusive editor-zone face (ati18).
    slot: EditorSlot,
    tree: TreeSelector,
    tree_filter: FilterMode,
    last_esc_at: Option<Instant>,
    /// `!` / `!!` prefix → success border (c492).
    bash_mode: bool,
    /// Ctrl+G stub invocation count (harness).
    external_editor_invocations: u32,
    /// Double Esc while idle → host fetches MessageHistory via XyDriver (c615).
    pending_tree_open: bool,
    /// Tree Enter → host calls `travel_session_tree` (c615).
    pending_tree_travel: Option<String>,
    /// Tree Shift+F → host calls `fork_session` + `switch_session` (c645).
    pending_tree_fork: Option<String>,
    /// Tree Shift+L commit → host `append_entry_label` (c690).
    pending_tree_label: Option<(String, Option<String>)>,
    /// Active label editor for selected tree node (c690).
    tree_label_edit: Option<(String, Input)>,
    /// Models Enter → host calls `SetModel` (c630).
    pending_model_select: Option<PendingModelChoice>,
    models_list: SelectList,
    models_items: Vec<SelectItem>,
    models_rows: Vec<ModelPickerRow>,
    models_filter: String,
    models_last_width: usize,
    /// `(model_id, description)` for [`SlashArgCompletionSource`] (c999).
    model_arg_catalog: Vec<(String, String)>,
    /// Themes Enter → host calls `reload_themes` (c1115).
    pending_theme_select: Option<String>,
    themes_list: SelectList,
    /// Root for [`AtPathSource`] (c1125); default process cwd.
    at_path_base: PathBuf,
    /// `(name, description)` for [`DollarSkillSource`] (c1130).
    dollar_skill_catalog: Vec<(String, String)>,
    /// `/session-import` confirm (c1010).
    import_confirm_list: SelectList,
    import_confirm_path: Option<String>,
    pending_import_decision: Option<ImportConfirmDecision>,
    /// Builtin `ask` ChoicePrompt (c1850).
    choice_prompt: Option<ChoicePrompt>,
    choice_pending: Option<Rc<RefCell<Option<ChoiceResult>>>>,
    ask_reply: Option<oneshot::Sender<Result<String, XyToolError>>>,
    /// `/session-resume` picker (c1015 / c1065).
    pub(crate) session_resume: SessionResumePanel,
    pending_session_resume_select: Option<String>,
    pending_session_resume_rename: Option<(String, String)>,
    pending_session_resume_delete: Option<String>,
    /// `/mcp` SelectList (c1215).
    mcp_list: SelectList,
    /// Summary above the list: `configured N · connected K · armed A`.
    mcp_summary_line: String,
    /// Optional diag lines under the list.
    mcp_diag_lines: Vec<String>,
    /// Generation for loaded+scrollback+queue cache (ath24); bumps on content/theme/fold.
    upper_gen: u64,
    upper_cache_gen: u64,
    upper_cache_width: usize,
    upper_cache_lines: Vec<String>,
    scrollback_paint: ScrollbackPaintCache,
    /// Terminal rows from host (Chrome Footprint / atc23); soft default until first sync.
    term_rows: usize,
    /// Last paint: toast + status + editor + footer row count (ApplicationOwned dock).
    last_dock_rows: usize,
    /// Rows in toast / status / editor from last paint (ApplicationOwned mouse origin).
    last_toast_rows: usize,
    last_status_rows: usize,
    last_editor_rows: usize,
    /// ApplicationOwned copy-success cue (`Copied`, ~2s). Not chrome-toast / ScrollNotice.
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
            chrome_toast: None,
            slot: EditorSlot::Editor,
            tree: empty_tree_selector(theme),
            tree_filter: FilterMode::Default,
            last_esc_at: None,
            bash_mode: false,
            external_editor_invocations: 0,
            pending_tree_open: false,
            pending_tree_travel: None,
            pending_tree_fork: None,
            pending_tree_label: None,
            tree_label_edit: None,
            pending_model_select: None,
            models_list: empty_models_list(theme),
            models_items: Vec::new(),
            models_rows: Vec::new(),
            models_filter: String::new(),
            models_last_width: 80,
            model_arg_catalog: Vec::new(),
            pending_theme_select: None,
            themes_list: empty_themes_list(theme),
            at_path_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dollar_skill_catalog: Vec::new(),
            import_confirm_list: make_import_confirm_list(theme),
            import_confirm_path: None,
            pending_import_decision: None,
            choice_prompt: None,
            choice_pending: None,
            ask_reply: None,
            session_resume: empty_session_resume_panel(theme),
            pending_session_resume_select: None,
            pending_session_resume_rename: None,
            pending_session_resume_delete: None,
            mcp_list: empty_mcp_list(theme),
            mcp_summary_line: String::new(),
            mcp_diag_lines: Vec::new(),
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
        self.at_path_base = base.into();
        self.install_completion_sources();
    }

    /// Refresh `/model <id>` inline completion catalog (from `available_models`).
    pub fn set_model_arg_catalog(&mut self, catalog: Vec<(String, String)>) {
        self.model_arg_catalog = catalog;
        self.install_completion_sources();
    }

    /// Refresh `$skill` completion catalog (from Trust-filtered loaded skills; c1130).
    pub fn set_dollar_skill_catalog(&mut self, catalog: Vec<(String, String)>) {
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

    pub fn sync_fold_hit_viewport(&mut self, scroll_top: usize, transcript_rows: u16) {
        self.fold_hits.scroll_top = scroll_top;
        self.fold_hits.transcript_rows = transcript_rows;
    }

    /// Single-block fold toggle (mouse triangle). Does **not** clear the whole
    /// paint cache — entry fingerprints carry effective fold (ath25).
    pub fn toggle_fold_target(&mut self, target: FoldTarget) {
        match target {
            FoldTarget::Tool(id) | FoldTarget::Diff(id) | FoldTarget::Ask(id) => {
                self.fold.toggle_tools(&id);
            }
            FoldTarget::Thinking(id) => {
                self.fold.toggle_thinking(&id);
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

    /// Arm ApplicationOwned «Copied» chrome cue (~2s). Must not use Error: toast (ath31).
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
        crate::app::tui::layout::reserved_lower_chrome(
            self.status_busy,
            queue_rows,
            self.chrome_toast.is_some(),
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

    pub fn slot(&self) -> EditorSlot {
        self.slot
    }

    pub fn tree_open(&self) -> bool {
        self.slot.is_tree()
    }

    pub fn take_pending_tree_open(&mut self) -> bool {
        std::mem::take(&mut self.pending_tree_open)
    }

    /// Request MessageHistory tree open (double Esc / `/session-tree`, c700/c1005).
    pub fn request_tree_open(&mut self) {
        self.pending_tree_open = true;
    }

    pub fn take_pending_tree_travel(&mut self) -> Option<String> {
        self.pending_tree_travel.take()
    }

    pub fn take_pending_tree_fork(&mut self) -> Option<String> {
        self.pending_tree_fork.take()
    }

    /// Queue fork for `entry_id` (`/session-fork` at leaf or Shift+F).
    pub fn request_tree_fork(&mut self, entry_id: String) {
        self.pending_tree_fork = Some(entry_id);
    }

    pub fn take_pending_tree_label(&mut self) -> Option<(String, Option<String>)> {
        self.pending_tree_label.take()
    }

    pub fn apply_tree_label(&mut self, id: &str, label: Option<String>) {
        self.tree.set_annotation(id, label.clone());
        if label.is_some() {
            self.tree.set_annotation_at(id, Some("just now".into()));
        } else {
            self.tree.set_annotation_at(id, None);
        }
    }

    pub fn take_pending_model_select(&mut self) -> Option<PendingModelChoice> {
        self.pending_model_select.take()
    }

    pub fn models_open(&self) -> bool {
        self.slot == EditorSlot::Models
    }

    pub fn take_pending_theme_select(&mut self) -> Option<String> {
        self.pending_theme_select.take()
    }

    pub fn themes_open(&self) -> bool {
        self.slot == EditorSlot::Themes
    }

    pub fn import_confirm_open(&self) -> bool {
        self.slot == EditorSlot::ImportConfirm
    }

    pub fn session_resume_open(&self) -> bool {
        self.slot == EditorSlot::SessionResume
    }

    pub fn take_pending_session_resume_select(&mut self) -> Option<String> {
        self.pending_session_resume_select.take()
    }

    pub fn take_pending_session_resume_rename(&mut self) -> Option<(String, String)> {
        self.pending_session_resume_rename.take()
    }

    pub fn take_pending_session_resume_delete(&mut self) -> Option<String> {
        self.pending_session_resume_delete.take()
    }

    #[cfg(test)]
    pub fn session_resume_panel_text_for_test(&self, width: usize) -> String {
        self.session_resume.render(width).join("\n")
    }

    /// Mount session resume panel in the editor slot (c1065).
    pub fn mount_session_resume_picker(
        &mut self,
        entries: Vec<crate::app::core::driver::SessionListEntry>,
        current_id: Option<&str>,
    ) {
        self.session_resume.set_current_cwd(&self.cwd);
        self.session_resume
            .load_entries(entries, current_id.map(str::to_string));
        self.slot = EditorSlot::SessionResume;
    }

    /// Placeholder while scanning session jsonl (pi loaded/total).
    pub fn mount_session_resume_loading(&mut self, loaded: usize, total: usize) {
        self.session_resume.set_current_cwd(&self.cwd);
        self.session_resume.set_loading(loaded, total);
        self.slot = EditorSlot::SessionResume;
    }

    pub fn close_session_resume(&mut self) {
        if self.slot == EditorSlot::SessionResume {
            self.close_slot();
        }
    }

    pub fn take_pending_import_decision(&mut self) -> Option<ImportConfirmDecision> {
        self.pending_import_decision.take()
    }

    /// Mount Yes/No import confirm in the editor slot (c1010).
    pub fn mount_import_confirm(&mut self, path: &str) {
        self.import_confirm_path = Some(path.to_string());
        self.import_confirm_list = make_import_confirm_list(self.theme);
        self.import_confirm_list.selected_index = 0;
        self.slot = EditorSlot::ImportConfirm;
    }

    pub fn close_import_confirm(&mut self) {
        if self.slot == EditorSlot::ImportConfirm {
            self.close_slot();
        }
    }

    /// Mount ChoicePrompt for builtin `ask` and park the oneshot reply (c1850).
    pub fn mount_ask_choice(
        &mut self,
        questions: Vec<ChoiceQuestion>,
        reply: oneshot::Sender<Result<String, XyToolError>>,
    ) {
        if questions.is_empty() {
            let _ = reply.send(Err(XyToolError::InvalidArgs(
                "ask requires at least one question".into(),
            )));
            return;
        }
        // Drop any prior unfinished ask (should not overlap under Barrier).
        if let Some(prev) = self.ask_reply.take() {
            let _ = prev.send(Err(XyToolError::Aborted));
        }
        let pending: Rc<RefCell<Option<ChoiceResult>>> = Rc::new(RefCell::new(None));
        let slot = pending.clone();
        let mut theme = self.theme.palette().choice_prompt_theme();
        // Product Ask stays rail-on (demo may wash via /entry-style).
        theme.rail = Some(self.theme.palette().accent);
        let prompt = ChoicePrompt::new(questions, theme, move |r| {
            *slot.borrow_mut() = Some(r);
        });
        self.choice_prompt = Some(prompt);
        self.choice_pending = Some(pending);
        self.ask_reply = Some(reply);
        self.slot = EditorSlot::Choice;
    }

    /// If ChoicePrompt finished, complete oneshot with ask JSON and close the slot.
    ///
    /// Returns true when a result was delivered.
    pub fn complete_ask_if_ready(&mut self) -> bool {
        let Some(pending) = self.choice_pending.as_ref() else {
            return false;
        };
        let Some(result) = pending.borrow_mut().take() else {
            return false;
        };
        let json = result.to_ask_payload_json();
        if let Some(tx) = self.ask_reply.take() {
            let _ = tx.send(Ok(json));
        }
        self.choice_prompt = None;
        self.choice_pending = None;
        if self.slot == EditorSlot::Choice {
            self.slot = EditorSlot::Editor;
        }
        true
    }

    pub fn close_ask_choice(&mut self) {
        if let Some(tx) = self.ask_reply.take() {
            let _ = tx.send(Err(XyToolError::Aborted));
        }
        self.choice_prompt = None;
        self.choice_pending = None;
        if self.slot == EditorSlot::Choice {
            self.slot = EditorSlot::Editor;
        }
    }

    pub fn ask_choice_open(&self) -> bool {
        self.slot == EditorSlot::Choice && self.choice_prompt.is_some()
    }

    /// Mount built-in theme picker in the editor slot (c1115).
    pub fn mount_themes_picker(&mut self, current: Option<&str>) {
        let current = current.unwrap_or("dark");
        let items: Vec<SelectItem> = ["dark", "light"]
            .into_iter()
            .map(|name| {
                let label = if current.eq_ignore_ascii_case(name) {
                    format!("{name} *")
                } else {
                    name.to_string()
                };
                SelectItem::new(name, label)
            })
            .collect();
        self.themes_list = SelectList::new(
            items,
            4,
            self.theme.select_list_theme(),
            SelectListLayoutOptions {
                min_primary_column_width: Some(12),
                max_primary_column_width: Some(24),
                truncate_primary: None,
            },
        );
        self.slot = EditorSlot::Themes;
    }

    fn apply_tree_filter(&mut self, mode: FilterMode) {
        self.tree_filter = mode;
        let filter = mode;
        self.tree
            .set_include_node(Some(Box::new(move |n| filter.include(n))));
        self.tree
            .set_status_suffix(mode.status_suffix().map(str::to_string));
    }

    /// Mount MessageHistory rows fetched via XyDriver and open the Tree slot.
    pub fn mount_session_tree(&mut self, roots: Vec<TreeNode>, active_id: Option<&str>) {
        self.tree_filter = FilterMode::Default;
        self.tree = TreeSelector::new(
            roots,
            self.theme.tree_selector_theme(),
            TreeSelectorOptions {
                max_visible: 10,
                unicode_connectors: true,
                include_node: Some(Box::new(|n| FilterMode::Default.include(n))),
                active_id: active_id.map(str::to_string),
                status_suffix: None,
            },
        );
        self.slot = EditorSlot::Tree;
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
        let _ = self.tree.select_id(id);
    }

    #[cfg(test)]
    pub fn open_slot_for_test(&mut self, slot: EditorSlot) {
        self.editor.set_text(String::new());
        self.open_slot(slot);
    }

    /// Push bridge UI model into status / footer; scrollback re-renders from model (c476).
    pub fn apply_ui_model(&mut self, model: &UiModel) {
        // Queue strip is dock chrome — steer/follow-up alone must not invalidate
        // the transcript upper cache (streaming frames stay cheaper).
        let upper_changed = self.ui_model.entries != model.entries
            || self.ui_model.streaming_assistant != model.streaming_assistant
            || self.ui_model.streaming_thinking != model.streaming_thinking;
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

    /// Apply active chrome for footer (model + thinking); omit thinking when not adjustable.
    pub fn set_active_chrome(
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

    /// Push / replace chrome toast body (TTL from [`crate::app::tui::commands::CHROME_TOAST_TTL`]).
    pub fn push_chrome_toast(&mut self, body: impl Into<String>) {
        self.chrome_toast = Some((
            body.into(),
            Instant::now() + crate::app::tui::commands::CHROME_TOAST_TTL,
        ));
    }

    /// Body only (no `Error: ` prefix); `None` when cleared / expired.
    pub fn chrome_toast_body(&self) -> Option<&str> {
        self.chrome_toast.as_ref().map(|(b, _)| b.as_str())
    }

    pub fn clear_chrome_toast(&mut self) {
        self.chrome_toast = None;
    }

    /// Test/harness: force deadline into the past so the next `tick` clears.
    #[cfg(test)]
    pub fn expire_chrome_toast_now(&mut self) {
        if let Some((body, _)) = self.chrome_toast.take() {
            self.chrome_toast = Some((
                body,
                Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
            ));
        }
    }

    /// Clear toast when past deadline; returns whether state changed.
    pub(super) fn clear_chrome_toast_if_expired(&mut self) -> bool {
        let expired = self
            .chrome_toast
            .as_ref()
            .is_some_and(|(_, d)| Instant::now() >= *d);
        if expired {
            self.chrome_toast = None;
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
        self.tree_filter
    }

    #[cfg(test)]
    pub fn tree_search_query_for_test(&self) -> &str {
        self.tree.search_query()
    }

    #[cfg(test)]
    pub fn tree_panel_text_for_test(&mut self, width: usize) -> String {
        self.tree.render(width).join("\n")
    }

    /// Full Tree slot head (Search / Help) + list for harness asserts.
    #[cfg(test)]
    pub fn tree_slot_text_for_test(&mut self, width: usize) -> String {
        self.render_editor_slot(width).join("\n")
    }

    #[cfg(test)]
    pub fn tree_is_folded_for_test(&self, id: &str) -> bool {
        self.tree.is_folded(id)
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
