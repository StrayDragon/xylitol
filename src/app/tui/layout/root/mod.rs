//! Product TUI root layout — transcript / status / editor|tree / footer.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.

mod editor_border;
mod models_slot;
mod mount;
mod render;
mod slot_input;
mod slot_nav;
mod theme_apply;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[cfg(test)]
use xylitol_tui::Component;
use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    AtPathSource, CompletionSource, Focusable, Input, SlashArgCompletionSource, SlashCommandSource,
    SystemClock, TreeNode, TreeSelector, TreeSelectorOptions, fg_rgb,
};

use super::dollar_skill_source::DollarSkillSource;
use super::models_picker::{ModelPickerRow, PendingModelChoice};
use super::slash_catalog::product_slash_commands_for_editor;

use super::session_tree::FilterMode;
use super::slots::EditorSlot;
use super::theme::LayoutTheme;
use crate::app::core::driver::LoadedResourcesSnapshot;
use crate::app::tui::bridge::UiModel;
use crate::app::tui::session_resume::SessionResumePanel;
use crate::app::tui::widgets::{
    GlyphSet, ScrollbackFold, footer_thinking_label, format_footer_text,
};
use crate::protocol::types::ThinkingLevel;

pub(super) fn empty_tree_selector(theme: LayoutTheme) -> TreeSelector {
    TreeSelector::new(
        Vec::new(),
        theme.tree_selector_theme(),
        TreeSelectorOptions {
            max_visible: 10,
            unicode_connectors: true,
            include_node: None,
            active_id: None,
            status_suffix: None,
        },
    )
}

pub(super) fn empty_models_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        8,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(24),
            max_primary_column_width: Some(48),
            truncate_primary: None,
        },
    )
}

pub(super) fn empty_themes_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        4,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(12),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    )
}

pub(super) fn import_confirm_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        vec![SelectItem::new("yes", "Yes"), SelectItem::new("no", "No")],
        4,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(8),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    )
}

pub(super) fn empty_session_resume_panel(theme: LayoutTheme) -> SessionResumePanel {
    SessionResumePanel::new(theme)
}

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
    thinking_level: ThinkingLevel,
    /// When true, footer omits the thinking segment (no-thinking active model).
    footer_omit_thinking: bool,
    /// Agent-busy status trail (`Next turn: …`); independent of status short-word.
    status_trail: Option<String>,
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
    /// `/session-resume` picker (c1015 / c1065).
    pub(crate) session_resume: SessionResumePanel,
    pending_session_resume_select: Option<String>,
    pending_session_resume_rename: Option<(String, String)>,
    pending_session_resume_delete: Option<String>,
    /// Generation for loaded+scrollback+queue cache (ath24); bumps on content/theme/fold.
    upper_gen: u64,
    upper_cache_gen: u64,
    upper_cache_width: usize,
    upper_cache_lines: Vec<String>,
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
            thinking_level: ThinkingLevel::Off,
            footer_omit_thinking: false,
            status_trail: None,
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
            import_confirm_list: import_confirm_list(theme),
            import_confirm_path: None,
            pending_import_decision: None,
            session_resume: empty_session_resume_panel(theme),
            pending_session_resume_select: None,
            pending_session_resume_rename: None,
            pending_session_resume_delete: None,
            upper_gen: 0,
            upper_cache_gen: u64::MAX,
            upper_cache_width: usize::MAX,
            upper_cache_lines: Vec::new(),
            #[cfg(test)]
            upper_rebuild_count: 0,
        };
        root.install_completion_sources();
        root.sync_editor_border();
        root.refresh_footer_from_queue(0, 0);
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
        self.loaded_resources = snap;
        self.bump_upper_gen();
    }

    /// Inject footer identity (cwd · model). Call before first render when known.
    pub fn set_layout_meta(&mut self, cwd: impl Into<String>, model: impl Into<String>) {
        self.cwd = cwd.into();
        self.model = model.into();
        self.refresh_footer_from_queue(0, 0);
    }

    /// Set or clear the provenance-honest token usage fragment (c1035).
    pub fn set_footer_token_label(&mut self, label: Option<String>) {
        self.footer_token = label.filter(|s| !s.is_empty());
        self.refresh_footer_from_queue(
            self.ui_model.queue.steer_count,
            self.ui_model.queue.follow_up_count,
        );
    }

    pub fn set_glyphs(&mut self, glyphs: GlyphSet) {
        self.glyphs = glyphs;
    }

    pub fn glyphs(&self) -> GlyphSet {
        self.glyphs
    }

    pub fn fold(&self) -> ScrollbackFold {
        self.fold
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
    pub fn open_external_editor_stub(&mut self) {
        self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
        let text = self.editor.get_text();
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
        self.import_confirm_list = import_confirm_list(self.theme);
        self.import_confirm_list.selected_index = 0;
        self.slot = EditorSlot::ImportConfirm;
    }

    pub fn close_import_confirm(&mut self) {
        if self.slot == EditorSlot::ImportConfirm {
            self.close_slot();
        }
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
        self.ui_model = model.clone();
        self.bump_upper_gen();

        match model.status.as_ref() {
            Some(s) if !s.is_empty() => {
                self.status_loader.set_message(s.clone());
                self.status_busy = true;
            }
            _ => {
                self.status_busy = false;
            }
        }

        self.refresh_footer_from_queue(model.queue.steer_count, model.queue.follow_up_count);
    }

    fn bump_upper_gen(&mut self) {
        self.upper_gen = self.upper_gen.saturating_add(1);
    }

    fn refresh_footer_from_queue(&mut self, steer: usize, follow_up: usize) {
        let thinking = if self.footer_omit_thinking {
            String::new()
        } else {
            footer_thinking_label(self.thinking_level)
        };
        let base = format_footer_text(
            &self.cwd,
            &self.model,
            &thinking,
            steer,
            follow_up,
            self.footer_token.as_deref(),
        );
        self.footer.set_text(self.theme.paint_muted(&base));
    }

    /// Apply active chrome for footer (model + thinking); omit thinking when not adjustable.
    pub fn set_active_chrome(
        &mut self,
        model_label: impl Into<String>,
        thinking: ThinkingLevel,
        omit_thinking: bool,
    ) {
        self.model = model_label.into();
        self.thinking_level = thinking;
        self.footer_omit_thinking = omit_thinking;
        self.sync_editor_border();
        self.refresh_footer_from_queue(
            self.ui_model.queue.steer_count,
            self.ui_model.queue.follow_up_count,
        );
    }

    /// Set or clear status trail (agent-busy NextTurn pending).
    pub fn set_status_trail(&mut self, trail: Option<String>) {
        self.status_trail = trail.filter(|s| !s.is_empty());
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
    pub fn thinking_level_for_test(&self) -> ThinkingLevel {
        self.thinking_level
    }

    #[cfg(test)]
    pub fn upper_rebuild_count_for_test(&self) -> u64 {
        self.upper_rebuild_count
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
