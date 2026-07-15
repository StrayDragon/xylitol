//! Product TUI root layout — transcript / status / editor|tree / footer.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    CompletionSource, Component, Focusable, Input, InputEvent, InputListenerResult,
    SlashArgCompletionSource, SlashCommand, SlashCommandSource, SystemClock, TUI, Terminal,
    TreeNode, TreeSelector, TreeSelectorOptions, fg_rgb, fuzzy_filter, matches_key_event,
    printable_from_key_event, truncate_to_width,
};

use super::session_tree::{FilterMode, tree_help_line, tree_search_line, wrap_help_line};
use super::slots::EditorSlot;
use super::theme::LayoutTheme;
use crate::app::tui::bridge::{UiModel, UiPhase};
use crate::app::tui::host::{LayoutMode, TOO_SMALL_HINT};
use crate::app::tui::widgets::{
    GlyphSet, ScrollbackFold, format_footer_text, render_queue_strip, render_scrollback,
};

fn empty_tree_selector(theme: LayoutTheme) -> TreeSelector {
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

fn product_slash_commands() -> Vec<SlashCommand> {
    let mut cmds = vec![
        SlashCommand {
            name: "exit".into(),
            description: Some("Quit TUI".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "model".into(),
            description: Some("Switch model: /model [id]".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-tree".into(),
            description: Some("Open session tree (same as double Esc)".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-fork".into(),
            description: Some("Fork session at current leaf".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-compact".into(),
            description: Some("Compact session context".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-export".into(),
            description: Some("Export session (default HTML; .jsonl → JSONL)".into()),
            argument_hint: Some("[path]".into()),
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-import".into(),
            description: Some("Import session from JSONL".into()),
            argument_hint: Some("<path>".into()),
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session".into(),
            description: Some("Show session info and stats".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-resume".into(),
            description: Some("Switch to another session".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
    ];
    // Hand-test only — see `app::debug_fixtures` (delete that module to remove).
    #[cfg(debug_assertions)]
    cmds.push(SlashCommand {
        name: "debug".into(),
        description: Some("Load fixture: /debug <scene>".into()),
        argument_hint: Some("<scene>".into()),
        get_argument_completions: None,
    });
    cmds
}

fn empty_models_list(theme: LayoutTheme) -> SelectList {
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

fn import_confirm_list(theme: LayoutTheme) -> SelectList {
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

fn empty_session_resume_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        10,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(16),
            max_primary_column_width: Some(48),
            truncate_primary: None,
        },
    )
}

/// User choice from `/session-import` confirm slot (c1010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportConfirmDecision {
    Accepted { path: String },
    Rejected,
}

/// Root UI: live scrollback + optional status + bordered editor|tree + footer.
pub struct UiRoot {
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
    /// Mutually exclusive editor-zone face (ati18).
    slot: EditorSlot,
    tree: TreeSelector,
    tree_filter: FilterMode,
    last_esc_at: Option<Instant>,
    /// `!` / `!!` prefix → success border (c492).
    bash_mode: bool,
    /// Ctrl+G stub invocation count (harness).
    external_editor_invocations: u32,
    /// Double Esc while idle → host fetches MessageHistory via Driver (c615).
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
    pending_model_select: Option<String>,
    models_list: SelectList,
    models_items: Vec<SelectItem>,
    models_filter: String,
    /// `(model_id, description)` for [`SlashArgCompletionSource`] (c999).
    model_arg_catalog: Vec<(String, String)>,
    /// `/session-import` confirm (c1010).
    import_confirm_list: SelectList,
    import_confirm_path: Option<String>,
    pending_import_decision: Option<ImportConfirmDecision>,
    /// `/session-resume` picker (c1015).
    session_resume_list: SelectList,
    pending_session_resume_select: Option<String>,
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
            model: "—".into(),
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
            models_filter: String::new(),
            model_arg_catalog: Vec::new(),
            import_confirm_list: import_confirm_list(theme),
            import_confirm_path: None,
            pending_import_decision: None,
            session_resume_list: empty_session_resume_list(theme),
            pending_session_resume_select: None,
        };
        root.install_completion_sources();
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
        sources.push(Box::new(SlashCommandSource::new(product_slash_commands())));
        self.editor.set_completion_sources(sources);
    }

    /// Refresh `/model <id>` inline completion catalog (from `available_models`).
    pub fn set_model_arg_catalog(&mut self, catalog: Vec<(String, String)>) {
        self.model_arg_catalog = catalog;
        self.install_completion_sources();
    }

    /// Inject footer identity (cwd · model). Call before first render when known.
    pub fn set_layout_meta(&mut self, cwd: impl Into<String>, model: impl Into<String>) {
        self.cwd = cwd.into();
        self.model = model.into();
        self.refresh_footer_from_queue(0, 0);
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
        self.editor.get_text()
    }

    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
        self.sync_editor_border();
    }

    /// Whether the editor is in bash accent mode (`!` / `!!` prefix).
    pub fn bash_mode(&self) -> bool {
        self.bash_mode
    }

    pub fn external_editor_invocations(&self) -> u32 {
        self.external_editor_invocations
    }

    /// Sync operation-zone border for `!` / `!!` (c492 / ati15).
    pub fn sync_editor_border(&mut self) {
        let bash = self.editor.get_text().trim_start().starts_with('!');
        if bash == self.bash_mode {
            return;
        }
        self.bash_mode = bash;
        if bash {
            self.editor.set_border_color(self.theme.bash_border_color());
        } else {
            self.editor
                .set_border_color(self.theme.muted_border_color());
        }
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

    pub fn take_pending_model_select(&mut self) -> Option<String> {
        self.pending_model_select.take()
    }

    pub fn models_open(&self) -> bool {
        self.slot == EditorSlot::Models
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

    /// Mount session resume SelectList in the editor slot (c1015).
    pub fn mount_session_resume_picker(
        &mut self,
        items: Vec<SelectItem>,
        current_id: Option<&str>,
    ) {
        let mut list = empty_session_resume_list(self.theme);
        list.filtered_items = items;
        if let Some(cur) = current_id
            && let Some(idx) = list
                .filtered_items
                .iter()
                .position(|item| item.value == cur)
        {
            list.selected_index = idx;
        }
        self.session_resume_list = list;
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

    /// Mount fuzzy model picker in the editor slot (c630).
    pub fn mount_models_picker(&mut self, items: Vec<SelectItem>) {
        self.models_filter.clear();
        self.models_items = items;
        self.models_list = empty_models_list(self.theme);
        self.apply_models_filter();
        self.slot = EditorSlot::Models;
    }

    fn apply_models_filter(&mut self) {
        let filter = self.models_filter.as_str();
        self.models_list.filtered_items = if filter.is_empty() {
            self.models_items.clone()
        } else {
            fuzzy_filter(&self.models_items, filter, |item| item.value.as_str())
        };
        self.models_list.selected_index = 0;
    }

    fn models_filter_line(&self) -> String {
        if self.models_filter.is_empty() {
            self.theme.paint_muted(" models")
        } else {
            self.theme
                .paint_muted(&format!(" filter: {}", self.models_filter))
        }
    }

    fn apply_tree_filter(&mut self, mode: FilterMode) {
        self.tree_filter = mode;
        let filter = mode;
        self.tree
            .set_include_node(Some(Box::new(move |n| filter.include(n))));
        self.tree
            .set_status_suffix(mode.status_suffix().map(str::to_string));
    }

    /// Mount MessageHistory rows fetched via Driver and open the Tree slot.
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
    pub fn on_escape(&mut self) -> bool {
        if self.slot.is_tree() {
            if self.tree_label_edit.take().is_some() {
                return true;
            }
            if self.tree.clear_search_if_any() {
                return true;
            }
            self.close_slot();
            return true;
        }
        if self.slot == EditorSlot::ImportConfirm {
            self.pending_import_decision = Some(ImportConfirmDecision::Rejected);
            self.close_import_confirm();
            return true;
        }
        if self.slot.is_overlay() {
            self.close_slot();
            return true;
        }
        if self.ui_model.phase == UiPhase::Busy {
            self.last_esc_at = None;
            return false;
        }
        if self.editor.get_text().is_empty() {
            let now = Instant::now();
            if let Some(prev) = self.last_esc_at
                && now.duration_since(prev) < Duration::from_millis(500)
            {
                self.last_esc_at = None;
                self.pending_tree_open = true;
                return true;
            }
            self.last_esc_at = Some(now);
            return false;
        }
        self.last_esc_at = None;
        false
    }

    pub fn close_slot(&mut self) {
        self.slot = EditorSlot::Editor;
        self.models_filter.clear();
        self.models_items.clear();
        self.models_list = empty_models_list(self.theme);
        self.import_confirm_path = None;
        self.import_confirm_list = import_confirm_list(self.theme);
        self.session_resume_list = empty_session_resume_list(self.theme);
    }

    pub fn close_session_tree(&mut self) {
        if self.slot.is_tree() {
            self.close_slot();
        }
    }

    /// Open a non-Editor slot (replaces any current overlay).
    pub fn open_slot(&mut self, slot: EditorSlot) {
        match slot {
            EditorSlot::Editor => self.close_slot(),
            EditorSlot::Tree => self.pending_tree_open = true,
            EditorSlot::Plate | EditorSlot::Settings | EditorSlot::Choice => {
                self.slot = slot;
            }
            EditorSlot::Models | EditorSlot::ImportConfirm | EditorSlot::SessionResume => {
                // Opened via mount_* after slash dispatch.
            }
        }
    }

    #[cfg(test)]
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

    fn refresh_footer_from_queue(&mut self, steer: usize, follow_up: usize) {
        let base = format_footer_text(&self.cwd, &self.model, steer, follow_up);
        self.footer.set_text(self.theme.paint_muted(&base));
    }

    /// Pending steer / follow-up strip above status (pi `pendingMessagesContainer`).
    fn render_queue_slot(&mut self, width: usize) -> Vec<String> {
        render_queue_strip(
            self.theme,
            &self.ui_model.pending_steer,
            &self.ui_model.pending_follow_up,
            width,
        )
    }

    fn render_status_slot(&mut self, width: usize) -> Vec<String> {
        if !self.status_busy {
            // Idle breathing room above editor (status.md / agent_demo status_lines).
            return vec![String::new()];
        }
        // Keep Loader leading blank + spinner row (do not strip empties).
        self.status_loader.render(width)
    }

    fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        match self.slot {
            EditorSlot::Editor => self.editor.render(width.max(1)),
            EditorSlot::Tree => {
                let mut lines = Vec::new();
                lines.push(" Session tree".to_string());
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    lines.push(
                        self.theme
                            .paint_muted(" Label edit · Enter save · Esc cancel"),
                    );
                    lines.extend(input.render(width.max(1)));
                    return lines;
                }
                // pi order: TreeHelp then SearchLine.
                for help in wrap_help_line(&tree_help_line(), width.max(1)) {
                    lines.push(self.theme.paint_muted(&help));
                }
                lines.push(
                    self.theme
                        .paint_muted(&tree_search_line(self.tree.search_query())),
                );
                lines.extend(self.tree.render(width.max(1)));
                lines
            }
            EditorSlot::Plate => vec![
                " Command Plate".to_string(),
                " (stub) Esc close".to_string(),
            ],
            EditorSlot::Settings => vec![" Settings".to_string(), " (stub) Esc close".to_string()],
            EditorSlot::Choice => vec![" Choice".to_string(), " (stub) Esc close".to_string()],
            EditorSlot::Models => {
                let mut lines = Vec::new();
                lines.push(self.models_filter_line());
                lines.extend(self.models_list.render(width.max(1)));
                lines
            }
            EditorSlot::ImportConfirm => {
                let mut lines = Vec::new();
                let path = self.import_confirm_path.as_deref().unwrap_or("?");
                lines.push(
                    self.theme
                        .paint_muted(&format!(" Replace current session with {path}?")),
                );
                lines.extend(self.import_confirm_list.render(width.max(1)));
                lines
            }
            EditorSlot::SessionResume => {
                let mut lines = Vec::new();
                lines.push(self.theme.paint_muted(" sessions"));
                lines.extend(self.session_resume_list.render(width.max(1)));
                lines
            }
        }
    }

    fn render_scrollback_slot(&mut self, width: usize) -> Vec<String> {
        // Idle empty: 0 rows (DESIGN editor.md — no loud placeholder wall).
        render_scrollback(&self.ui_model, self.glyphs, self.theme, self.fold, width)
    }

    #[cfg(test)]
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
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.extend(self.render_scrollback_slot(width));
        // Queue strip sits between transcript and status (pi morphology).
        lines.extend(self.render_queue_slot(width));
        lines.extend(self.render_status_slot(width));
        // Editor owns the operation-zone ─ borders (DESIGN editor.md / agent_demo).
        // Do NOT wrap with a second outer border pair.
        lines.extend(self.render_editor_slot(width));
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        lines.push(footer);
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        match self.slot {
            EditorSlot::Tree => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    if matches_key_event(key, "enter") {
                        if let Some((id, input)) = self.tree_label_edit.take() {
                            let text = input.value().trim().to_string();
                            let ann = if text.is_empty() { None } else { Some(text) };
                            self.pending_tree_label = Some((id, ann));
                        }
                        return;
                    }
                    input.handle_input(event);
                    return;
                }
                if matches_key_event(key, "ctrl+d") {
                    self.apply_tree_filter(FilterMode::Default);
                    return;
                }
                if matches_key_event(key, "ctrl+t") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::NoTools));
                    return;
                }
                if matches_key_event(key, "ctrl+u") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::UserOnly));
                    return;
                }
                if matches_key_event(key, "ctrl+l") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::LabeledOnly));
                    return;
                }
                if matches_key_event(key, "ctrl+a") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::All));
                    return;
                }
                if matches_key_event(key, "ctrl+shift+o") {
                    self.apply_tree_filter(self.tree_filter.cycle_backward());
                    return;
                }
                if matches_key_event(key, "ctrl+o") {
                    self.apply_tree_filter(self.tree_filter.cycle());
                    return;
                }
                if matches_key_event(key, "enter") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_travel = Some(id);
                    return;
                }
                if matches_key_event(key, "shift+f") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_fork = Some(id);
                    return;
                }
                if matches_key_event(key, "shift+l") {
                    let Some(id) = self.tree.selected_id().map(str::to_string) else {
                        return;
                    };
                    let current = self.tree.annotation_of(&id).unwrap_or("").to_string();
                    let mut input = Input::new();
                    input.set_value(current);
                    self.tree_label_edit = Some((id, input));
                    return;
                }
                if matches_key_event(key, "shift+t") {
                    self.tree.toggle_annotation_timestamps();
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                    || matches_key_event(key, "left")
                    || matches_key_event(key, "right")
                    || matches_key_event(key, "ctrl+left")
                    || matches_key_event(key, "alt+left")
                    || matches_key_event(key, "ctrl+right")
                    || matches_key_event(key, "alt+right")
                    || matches_key_event(key, "backspace")
                    || printable_from_key_event(key).is_some()
                {
                    self.tree.handle_input(event);
                }
                return;
            }
            EditorSlot::Plate | EditorSlot::Settings | EditorSlot::Choice => {
                // Empty shells: Esc is handled by InputListener; ignore other keys.
                return;
            }
            EditorSlot::Models => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_key_event(key, "enter") {
                    if let Some(item) = self.models_list.get_selected_item() {
                        self.pending_model_select = Some(item.value.clone());
                    }
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                {
                    self.models_list.handle_input(event);
                    return;
                }
                if matches_key_event(key, "backspace") {
                    self.models_filter.pop();
                    self.apply_models_filter();
                    return;
                }
                if let Some(text) = printable_from_key_event(key) {
                    self.models_filter.push_str(&text);
                    self.apply_models_filter();
                }
                return;
            }
            EditorSlot::ImportConfirm => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_key_event(key, "enter") {
                    let Some(path) = self.import_confirm_path.clone() else {
                        return;
                    };
                    let accepted = self
                        .import_confirm_list
                        .get_selected_item()
                        .is_some_and(|item| item.value == "yes");
                    self.pending_import_decision = Some(if accepted {
                        ImportConfirmDecision::Accepted { path }
                    } else {
                        ImportConfirmDecision::Rejected
                    });
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                {
                    self.import_confirm_list.handle_input(event);
                }
                return;
            }
            EditorSlot::SessionResume => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_key_event(key, "enter") {
                    if let Some(item) = self.session_resume_list.get_selected_item() {
                        self.pending_session_resume_select = Some(item.value.clone());
                    }
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                {
                    self.session_resume_list.handle_input(event);
                }
                return;
            }
            EditorSlot::Editor => {}
        }

        if let InputEvent::Key(ref key) = event {
            if matches_key_event(key, "ctrl+t") {
                self.fold.thinking_expanded = !self.fold.thinking_expanded;
                return;
            }
            if matches_key_event(key, "alt+e") {
                self.fold.tools_expanded = !self.fold.tools_expanded;
                return;
            }
            if matches_key_event(key, "ctrl+o") {
                self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
                return;
            }
            // MAY: Ctrl+P opens Plate stub (Esc closes).
            if matches_key_event(key, "ctrl+p") {
                self.open_slot(EditorSlot::Plate);
                return;
            }
        }

        self.editor.handle_input(event);
        self.sync_editor_border();
    }

    fn invalidate(&mut self) {
        self.status_loader.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
        self.tree.invalidate();
        self.models_list.invalidate();
        self.import_confirm_list.invalidate();
        self.session_resume_list.invalidate();
    }

    fn tick(&mut self) -> bool {
        let mut dirty = self.editor.tick();
        if self.status_busy {
            let interval = self.status_loader.interval_ms() as u128;
            if self.loader_last_tick.elapsed().as_millis() >= interval {
                dirty = Component::tick(&mut self.status_loader) || dirty;
                self.loader_last_tick = Instant::now();
            }
        }
        dirty
    }
}

/// Shared root so InputListeners and the focused Component see the same state.
pub struct SharedUiRoot(pub Rc<RefCell<UiRoot>>);

impl Component for SharedUiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.0.borrow_mut().render(width)
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.0.borrow_mut().handle_input(event);
    }

    fn invalidate(&mut self) {
        self.0.borrow_mut().invalidate();
    }

    fn tick(&mut self) -> bool {
        self.0.borrow_mut().tick()
    }
}

/// Full-screen hint when the terminal is too small.
pub struct TooSmallHint;

impl Component for TooSmallHint {
    fn render(&mut self, width: usize) -> Vec<String> {
        let msg = TOO_SMALL_HINT;
        if width == 0 {
            return vec![msg.into()];
        }
        let pad = width.saturating_sub(msg.chars().count()) / 2;
        vec![format!("{}{msg}", " ".repeat(pad))]
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}
}

/// Build root children for a layout mode (no shared state — simple tests).
#[cfg(test)]
pub fn build_root(mode: LayoutMode) -> Vec<Box<dyn Component>> {
    match mode {
        LayoutMode::Ready => vec![Box::new(UiRoot::new())],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Shared root + rebuild closure that keeps the same `UiRoot` across min-size flips.
pub fn shared_ui_root_rebuild(
    root: Rc<RefCell<UiRoot>>,
) -> impl FnMut(LayoutMode) -> Vec<Box<dyn Component>> + 'static {
    move |mode| match mode {
        LayoutMode::Ready => vec![Box::new(SharedUiRoot(root.clone()))],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Register pre-focus Ctrl+C / Esc (editor-slot overlays). Busy Esc abort is host-side (c480).
pub fn install_ui_root_key_listeners<T: Terminal>(
    root: &Rc<RefCell<UiRoot>>,
    quit_flag: &Arc<AtomicBool>,
    tui: &mut TUI<T>,
) {
    let root = root.clone();
    let quit_flag = quit_flag.clone();
    tui.add_input_listener(move |event| {
        let InputEvent::Key(key) = &event else {
            return InputListenerResult::Continue;
        };
        if matches_key_event(key, "ctrl+c") {
            root.borrow_mut().on_ctrl_c(&quit_flag);
            return InputListenerResult::Consumed;
        }
        if matches_key_event(key, "escape") && root.borrow_mut().on_escape() {
            return InputListenerResult::Consumed;
        }
        InputListenerResult::Continue
    });
}

#[cfg(test)]
pub(crate) fn sample_tree_nodes_for_test() -> Vec<TreeNode> {
    vec![
        TreeNode::new("root", "session · product").with_children([
            TreeNode::new("meta1", "fake / model").with_kind("meta"),
            TreeNode::new("u1", "hello")
                .with_kind("user")
                .with_annotation("keep")
                .with_child(
                    TreeNode::new("a1", "plan")
                        .with_kind("assistant")
                        .with_children([
                            TreeNode::new("t1", "read").with_kind("tool"),
                            TreeNode::new("a2", "done")
                                .with_kind("assistant")
                                .with_child(TreeNode::new("u2", "next").with_kind("user")),
                        ]),
                ),
            TreeNode::new("fork", "alternate")
                .with_kind("user")
                .with_child(TreeNode::new("af", "fork leaf").with_kind("assistant")),
        ]),
    ]
}
