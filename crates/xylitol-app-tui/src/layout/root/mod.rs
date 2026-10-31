mod capabilities_slot;
mod editor_border;
mod fixed_zone_footprint_apply;
mod footer_chips;
mod mcp_slot;
mod models_slot;
mod mount;
mod pending_slot;
mod render;
mod scenes_slot;
mod slot_input;
mod slot_nav;
mod theme_apply;

use pending_slot::PendingSlotOps;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossterm::event::KeyEvent;
use xylitol_tui::Component;
use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    AtPathSource, ChoiceQuestion, CompletionSource, Focusable, Hit, HitId, HitSpace, InputEvent,
    SlashArgCompletionSource, SlashCommandSource, SystemClock, TreeNode, fg_rgb,
};

use super::dollar_skill_source::DollarSkillSource;
use super::models_picker::PendingModelChoice;
use super::slash_catalog::slash_commands_for_editor;

use super::session_tree::FilterMode;
use super::slots::{AskSlot, EditorSlot, EditorSlotKind, ImportSlot, ThemesSlot, TreeSlot};
use super::theme::LayoutTheme;
use super::{WidthTier, width_tier};
use crate::activity_fold::{ActivityFoldState, AutoTrigger, ingest_rebuild_clocks};
use crate::bridge::UiModel;
use crate::driver::LoadedResourcesSnapshot;
use crate::session_resume::SessionResumePanel;
use crate::widgets::{
    FoldHitTable, FoldTarget, GlyphSet, ScrollbackFold, ScrollbackPaintCache, TodoBarHit,
    footer_thinking_label, format_footer_text,
};
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::model::THINKING_OFF;
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeTravel};

pub use super::slots::ImportConfirmDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientNoticeKind {
    Success,
    Info,
}

#[derive(Debug, Clone)]
struct ClientNotice {
    body: String,
    kind: ClientNoticeKind,
    until: Instant,
}

const CLIENT_NOTICE_MAX: usize = 3;

pub struct UiRoot {
    loaded_resources: LoadedResourcesSnapshot,
    ui_model: UiModel,
    fold: ScrollbackFold,
    fold_hits: FoldHitTable,
    fold_dirty: bool,
    load_earlier_requested: bool,
    activity: ActivityFoldState,
    status_loader: Loader,
    status_busy: bool,
    loader_last_tick: Instant,
    editor: Editor,
    footer: Text,
    theme: LayoutTheme,
    glyphs: GlyphSet,
    cwd: String,
    model: String,
    footer_token: Option<String>,
    thinking_level: String,
    footer_omit_thinking: bool,
    status_next_turn_cue: Option<String>,
    suppress_status_right_cue: bool,
    toast_notice: Option<(String, Instant, ToastKind)>,
    slot: EditorSlot,
    pending: PendingSlotOps,
    designing: bool,
    session_popup_open: bool,
    session_popup_id: String,
    session_sidebar_on: bool,
    session_sidebar_width: usize,
    session_sidebar_scroll: usize,
    last_esc_at: Option<Instant>,
    bash_mode: bool,
    external_editor_invocations: u32,
    model_arg_catalog: Vec<(String, String)>,
    at_path_base: PathBuf,
    dollar_skill_catalog: Vec<(String, String)>,
    plugin_commands: Vec<(String, String)>,
    upper_gen: u64,
    upper_cache_gen: u64,
    upper_cache_width: usize,
    upper_cache_lines: Vec<String>,
    scrollback_paint: ScrollbackPaintCache,
    term_rows: usize,
    term_cols: usize,
    last_dock_rows: usize,
    last_toast_rows: usize,
    last_status_rows: usize,
    last_editor_rows: usize,
    last_queue_rows: usize,
    last_mode_rows: usize,
    queue_strip_row: usize,
    last_footer_hits: Vec<footer_chips::FooterChipHit>,
    last_chip_row: Option<usize>,
    last_chip_width: usize,
    last_slot_top_row: Option<usize>,
    last_slot_close_hit: Option<(usize, usize)>,
    last_todo_rows: usize,
    todo_expanded: bool,
    todo_scroll: usize,
    last_todo_hits: Vec<TodoBarHit>,
    last_todo_plain: Vec<String>,
    todo_selection: xylitol_tui::SelectionController,
    todo_clipboard: Vec<String>,
    todo_pointer_dirty: bool,
    client_notices: VecDeque<ClientNotice>,
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
            load_earlier_requested: false,
            activity: ActivityFoldState::default(),
            status_loader,
            status_busy: false,
            loader_last_tick: Instant::now(),
            editor,
            footer: Text::new(String::new(), 0, 0),
            theme,
            glyphs: GlyphSet::from_env(),
            cwd: ".".into(),
            model: crate::UNSET_MODEL_DISPLAY.into(),
            footer_token: None,
            thinking_level: THINKING_OFF.into(),
            footer_omit_thinking: false,
            status_next_turn_cue: None,
            suppress_status_right_cue: false,
            toast_notice: None,
            slot: EditorSlot::Editor,
            pending: PendingSlotOps::default(),
            designing: false,
            session_popup_open: false,
            session_popup_id: String::new(),
            session_sidebar_on: false,
            session_sidebar_width: Self::SESSION_SIDEBAR_WIDTHS[0],
            session_sidebar_scroll: 0,
            last_esc_at: None,
            bash_mode: false,
            external_editor_invocations: 0,
            model_arg_catalog: Vec::new(),
            at_path_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dollar_skill_catalog: Vec::new(),
            plugin_commands: Vec::new(),
            upper_gen: 0,
            upper_cache_gen: u64::MAX,
            upper_cache_width: usize::MAX,
            upper_cache_lines: Vec::new(),
            scrollback_paint: ScrollbackPaintCache::default(),
            term_rows: 24,
            term_cols: 80,
            last_dock_rows: 8,
            last_toast_rows: 0,
            last_status_rows: 1,
            last_editor_rows: 3,
            last_queue_rows: 0,
            last_mode_rows: 0,
            queue_strip_row: 0,
            last_footer_hits: Vec::new(),
            last_chip_row: None,
            last_chip_width: 0,
            last_slot_top_row: None,
            last_slot_close_hit: None,
            last_todo_rows: 0,
            todo_expanded: false,
            todo_scroll: 0,
            last_todo_hits: Vec::new(),
            last_todo_plain: Vec::new(),
            todo_selection: xylitol_tui::SelectionController::new(),
            todo_clipboard: Vec::new(),
            todo_pointer_dirty: false,
            client_notices: VecDeque::new(),
            #[cfg(test)]
            upper_rebuild_count: 0,
        };
        root.install_completion_sources();
        root.sync_editor_border();
        root.refresh_footer();
        root
    }

    fn install_completion_sources(&mut self) {
        let sources: Vec<Box<dyn CompletionSource>> = vec![
            Box::new(
                SlashArgCompletionSource::new("model", self.model_arg_catalog.clone())
                    .with_id("model-id"),
            ),
            Box::new(
                SlashArgCompletionSource::new(
                    "theme",
                    vec![
                        ("dark".into(), "Dark palette (default)".into()),
                        ("light".into(), "Light palette".into()),
                        ("toggle".into(), "Toggle dark ↔ light".into()),
                    ],
                )
                .with_id("theme-name"),
            ),
            Box::new(
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
            ),
            Box::new({
                let src = SlashCommandSource::new(slash_commands_for_editor(
                    self.designing,
                    &self.plugin_commands,
                ));
                if self.designing {
                    src.show_on_bare_slash()
                } else {
                    src
                }
            }),
            Box::new(AtPathSource::new(self.at_path_base.clone())),
            Box::new(DollarSkillSource::new(self.dollar_skill_catalog.clone())),
        ];
        self.editor.set_completion_sources(sources);
    }

    pub fn set_at_path_base(&mut self, base: impl Into<PathBuf>) {
        let base = base.into();
        if self.at_path_base == base {
            return;
        }
        self.at_path_base = base;
        self.install_completion_sources();
    }

    pub fn set_model_arg_catalog(&mut self, catalog: Vec<(String, String)>) {
        if self.model_arg_catalog == catalog {
            return;
        }
        self.model_arg_catalog = catalog;
        self.install_completion_sources();
    }

    pub fn set_dollar_skill_catalog(&mut self, catalog: Vec<(String, String)>) {
        if self.dollar_skill_catalog == catalog {
            return;
        }
        self.dollar_skill_catalog = catalog;
        self.install_completion_sources();
    }

    pub fn set_plugin_commands(&mut self, commands: Vec<(String, String)>) {
        if self.plugin_commands == commands {
            return;
        }
        self.plugin_commands = commands;
        self.install_completion_sources();
    }

    pub fn set_loaded_resources(&mut self, snap: LoadedResourcesSnapshot) {
        if self.loaded_resources == snap {
            return;
        }
        self.loaded_resources = snap;
        self.bump_upper_gen();
        self.refresh_mcp_short_cue();
    }

    pub fn refresh_mcp_short_cue(&mut self) {
        use crate::driver::MCP_PENDING_CUE;
        if self.suppress_status_right_cue {
            self.status_next_turn_cue = None;
            return;
        }
        if self.loaded_resources.mcp_tools_pending() {
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

    pub fn set_layout_meta(&mut self, cwd: impl Into<String>, model: impl Into<String>) {
        self.cwd = cwd.into();
        self.model = model.into();
        self.refresh_footer();
    }

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

    pub fn set_activity_settings(&mut self, settings: crate::activity_fold::ActivityFoldSettings) {
        self.activity.settings = settings;
    }

    pub fn touch_activity(&mut self) {
        self.bump_upper_gen();
    }

    pub fn expand_nearest_activity(&mut self) -> bool {
        let changed = self.activity.expand_nearest(&self.ui_model.entries);
        if changed {
            self.bump_upper_gen();
        }
        changed
    }

    pub fn collapse_nearest_activity(&mut self) -> bool {
        let changed = self.activity.collapse_nearest(&self.ui_model.entries);
        if changed {
            self.bump_upper_gen();
        }
        changed
    }

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

    pub fn fold_view(&self) -> &ScrollbackFold {
        &self.fold
    }

    pub fn click_fold_at(&mut self, col: u16, row: u16) -> bool {
        if let Some(target) = self.fold_hits.hit(col, row) {
            self.toggle_fold_target(target);
            return true;
        }
        self.click_todo_bar_at(col, row)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        self.handle_slot_input(InputEvent::Key(key));
    }

    pub fn sync_fold_hit_viewport(&mut self, scroll_top: usize, transcript_rows: u16) {
        self.fold_hits.scroll_top = scroll_top;
        self.fold_hits.transcript_rows = transcript_rows;
    }

    pub fn toggle_fold_target(&mut self, target: FoldTarget) {
        match target {
            FoldTarget::Tool(id) | FoldTarget::Diff(id) | FoldTarget::Ask(id) => {
                self.fold.toggle_tools(&id);
            }
            FoldTarget::Thinking(id) => {
                self.fold.toggle_thinking(&id);
            }
            FoldTarget::Compaction => {
                self.fold.compaction_expanded = !self.fold.compaction_expanded;
            }
            FoldTarget::OutputViewport(id) => {
                self.fold.toggle_output(&id);
            }
            FoldTarget::Segment(_) | FoldTarget::Cluster(_) | FoldTarget::LiveTail => {
                self.toggle_activity_fold(target);
            }
            FoldTarget::LoadEarlier => {
                self.load_earlier_requested = true;
                self.fold_dirty = true;
                return;
            }
        }
        self.fold_dirty = true;
        self.bump_upper_gen();
    }

    fn toggle_activity_fold(&mut self, target: FoldTarget) {
        match target {
            FoldTarget::Segment(id) => {
                let _ = self.activity.toggle_one_step(&id);
            }
            FoldTarget::Cluster(id) => {
                let _ = self.activity.toggle_cluster(&id);
            }
            FoldTarget::LiveTail => {
                let _ = self.activity.expand_live_cluster(&self.ui_model.entries);
            }
            _ => unreachable!("activity fold target"),
        }
    }

    fn toggle_todo_expanded(&mut self) {
        self.todo_expanded = !self.todo_expanded;
        self.todo_scroll = 0;
    }

    pub fn click_todo_bar_at(&mut self, col: u16, screen_row: u16) -> bool {
        if self.last_todo_rows == 0 {
            return false;
        }
        let dock = self.last_dock_rows.max(1).min(self.term_rows);
        let dock_top = self.term_rows.saturating_sub(dock);
        let todo_top = dock_top.saturating_add(self.last_queue_rows);
        let todo_end = todo_top.saturating_add(self.last_todo_rows);
        let row = screen_row as usize;
        if row < todo_top || row >= todo_end {
            return false;
        }
        self.click_todo_bar_local(col as usize, row - todo_top)
    }

    fn scroll_todo_bar_at(&mut self, screen_row: u16, delta: isize) -> bool {
        if self.last_todo_rows == 0 || delta == 0 {
            return false;
        }
        let dock = self.last_dock_rows.max(1).min(self.term_rows);
        let dock_top = self.term_rows.saturating_sub(dock);
        let todo_top = dock_top.saturating_add(self.last_queue_rows);
        let todo_end = todo_top.saturating_add(self.last_todo_rows);
        let row = screen_row as usize;
        if row < todo_top || row >= todo_end {
            return false;
        }
        if delta < 0 {
            self.todo_scroll = self.todo_scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.todo_scroll = self.todo_scroll.saturating_add(delta as usize);
        }
        self.fold_dirty = true;
        true
    }

    pub(crate) fn click_todo_bar_local(&mut self, col: usize, row_in_bar: usize) -> bool {
        let Some(hit) = self
            .last_todo_hits
            .iter()
            .find(|h| h.row == row_in_bar && col >= h.col_start && col < h.col_end)
            .copied()
        else {
            return false;
        };
        let _ = hit;
        self.toggle_todo_expanded();
        true
    }

    #[cfg(test)]
    pub fn click_todo_bar_local_for_test(&mut self, col: usize, row_in_bar: usize) -> bool {
        self.click_todo_bar_local(col, row_in_bar)
    }

    #[cfg(test)]
    pub fn todo_bar_hits_for_test(&self) -> &[TodoBarHit] {
        &self.last_todo_hits
    }

    pub fn take_fold_dirty(&mut self) -> bool {
        std::mem::take(&mut self.fold_dirty)
    }

    pub fn take_load_earlier_request(&mut self) -> bool {
        std::mem::take(&mut self.load_earlier_requested)
    }

    pub(crate) fn last_dock_rows(&self) -> usize {
        self.last_dock_rows.max(1)
    }

    pub(crate) fn editor_wants_rerender(&self, event: &xylitol_tui::InputEvent) -> bool {
        self.todo_pointer_dirty || Component::input_wants_rerender(&self.editor, event)
    }

    pub fn arm_copy_notice(&mut self) {
        self.push_client_notice(ClientNoticeKind::Success, "Copied");
    }

    pub fn arm_copy_last_msg_notice(&mut self) {
        self.push_client_notice(ClientNoticeKind::Success, "Copied Last Msg");
    }

    pub fn copy_notice_visible(&self) -> bool {
        self.client_notices
            .iter()
            .any(|n| n.body == "Copied" && Instant::now() < n.until)
    }

    #[cfg(test)]
    pub fn copy_notice_body_for_test(&self) -> Option<&'static str> {
        self.copy_notice_visible().then_some("Copied")
    }

    pub fn client_notice_bodies_for_test(&self) -> Vec<String> {
        self.client_notices.iter().map(|n| n.body.clone()).collect()
    }

    #[cfg(test)]
    pub fn transient_notice_contains_for_test(&self, needle: &str) -> bool {
        self.toast_notice_body().is_some_and(|b| b.contains(needle))
            || self.client_notices.iter().any(|n| n.body.contains(needle))
    }

    pub fn push_client_notice(&mut self, kind: ClientNoticeKind, body: impl Into<String>) {
        let ttl = match kind {
            ClientNoticeKind::Success => xylitol_tui::COPY_NOTICE_TTL,
            ClientNoticeKind::Info => crate::commands::TOAST_NOTICE_TTL,
        };
        self.client_notices.push_front(ClientNotice {
            body: body.into(),
            kind,
            until: Instant::now() + ttl,
        });
        while self.client_notices.len() > CLIENT_NOTICE_MAX {
            self.client_notices.pop_back();
        }
    }

    #[cfg(test)]
    pub fn editor_screen_origin_for_test(&self) -> (u16, u16) {
        self.editor.screen_origin()
    }

    pub(crate) fn estimate_dock_rows(&self) -> usize {
        let queue_rows = if self.ui_model.pending_steer.is_empty()
            && self.ui_model.pending_follow_up.is_empty()
        {
            0
        } else {
            self.ui_model.pending_steer.len() + self.ui_model.pending_follow_up.len() + 1
        };
        let todo_rows = if self.ui_model.todo.is_empty() {
            0
        } else {
            crate::widgets::todo_bar_line_count(
                self.glyphs,
                crate::widgets::TodoBarParams {
                    list: &self.ui_model.todo,
                    expanded: self.todo_expanded,
                    scroll: 0,
                    width: 80,
                    max_rows: crate::widgets::todo_bar_max_rows(self.term_rows),
                },
            )
        };
        crate::layout::reserved_lower_fixed_zone(
            self.status_busy,
            queue_rows,
            todo_rows,
            self.toast_notice.is_some(),
            usize::from(self.ui_model.mode_strip.is_some()),
        )
        .saturating_add(4)
        .max(4)
    }

    pub fn editor_text(&self) -> String {
        self.editor.get_expanded_text()
    }

    pub fn editor_display_text(&self) -> String {
        self.editor.get_text()
    }

    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
        self.sync_editor_border();
    }

    pub fn insert_editor_text_at_cursor(&mut self, text: &str) {
        self.editor.insert_text_at_cursor(text);
        self.sync_editor_border();
    }

    pub fn editor_autocomplete_open(&self) -> bool {
        self.editor.is_showing_autocomplete()
    }

    pub fn confirm_editor_autocomplete(&mut self) -> bool {
        let ok = self.editor.confirm_autocomplete_selection();
        if ok {
            self.sync_editor_border();
        }
        ok
    }

    pub fn bash_mode(&self) -> bool {
        self.bash_mode
    }

    pub fn external_editor_invocations(&self) -> u32 {
        self.external_editor_invocations
    }

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

    pub fn remember_editor_send(&mut self, text: impl Into<String>) {
        self.editor.add_to_history(text.into());
    }

    pub fn replace_editor_send_history(&mut self, texts: impl IntoIterator<Item = String>) {
        self.editor.replace_history(texts);
    }

    pub fn merge_editor_history(&mut self, texts: impl IntoIterator<Item = String>) {
        self.editor.merge_history(texts);
    }

    pub fn editor_history_texts(&self) -> Vec<String> {
        self.editor.history_texts()
    }

    pub fn slot(&self) -> EditorSlotKind {
        self.slot.kind()
    }

    pub fn tree_open(&self) -> bool {
        self.slot.is_tree()
    }

    pub fn session_popup_open(&self) -> bool {
        self.session_popup_open
    }

    pub fn close_session_popup(&mut self) {
        self.session_popup_open = false;
    }

    pub fn request_session_popup(&mut self) {
        self.pending.session_popup = true;
    }

    #[cfg(test)]
    pub fn session_popup_pending_for_test(&self) -> bool {
        self.pending.session_popup
    }

    pub fn take_pending_session_popup(&mut self) -> bool {
        std::mem::take(&mut self.pending.session_popup)
    }

    pub fn show_session_popup(&mut self, session_id: &str) {
        self.session_popup_id = session_id.to_string();
        self.session_popup_open = true;
    }

    pub fn session_sidebar_on(&self) -> bool {
        self.session_sidebar_on
    }

    pub fn close_session_sidebar(&mut self) {
        self.session_sidebar_on = false;
        self.session_sidebar_scroll = 0;
    }

    pub fn request_session_sidebar(&mut self) {
        self.pending.session_sidebar = true;
    }

    #[cfg(test)]
    pub fn has_pending_session_sidebar(&self) -> bool {
        self.pending.session_sidebar
    }

    pub fn take_pending_session_sidebar(&mut self) -> bool {
        std::mem::take(&mut self.pending.session_sidebar)
    }

    pub fn show_session_sidebar(&mut self, session_id: &str) {
        self.session_popup_id = session_id.to_string();
        self.session_sidebar_on = true;
        self.session_sidebar_scroll = 0;
    }

    pub fn set_term_cols(&mut self, cols: usize) {
        self.term_cols = cols;
    }

    pub fn session_sidebar_available(&self) -> bool {
        width_tier(self.term_cols) >= WidthTier::Standard
    }

    pub fn session_sidebar_width(&self) -> usize {
        self.session_sidebar_width
    }

    pub fn session_sidebar_width_cycle(&mut self) -> bool {
        let current = self.session_sidebar_width;
        let idx = Self::SESSION_SIDEBAR_WIDTHS
            .iter()
            .position(|width| *width == current)
            .unwrap_or(0);
        let next = Self::SESSION_SIDEBAR_WIDTHS[(idx + 1) % Self::SESSION_SIDEBAR_WIDTHS.len()];
        if next == current {
            false
        } else {
            self.session_sidebar_width = next;
            true
        }
    }

    pub fn session_sidebar_scroll(&self) -> usize {
        self.session_sidebar_scroll
    }

    pub fn session_sidebar_content_rows(&self) -> usize {
        Self::SESSION_SIDEBAR_CONTENT_ROWS
    }

    pub fn scroll_session_sidebar(&mut self, delta: isize, visible_rows: usize) -> bool {
        let max_scroll = self
            .session_sidebar_content_rows()
            .saturating_sub(visible_rows);
        let next =
            (self.session_sidebar_scroll as isize + delta).clamp(0, max_scroll as isize) as usize;
        if next == self.session_sidebar_scroll {
            false
        } else {
            self.session_sidebar_scroll = next;
            true
        }
    }

    pub fn cycle_session_details(&mut self) {
        if self.session_sidebar_on && self.session_popup_open {
            self.close_session_sidebar();
            self.close_session_popup();
            return;
        }
        if self.session_sidebar_on {
            self.close_session_sidebar();
            return;
        }
        if self.session_popup_open {
            self.close_session_popup();
            if self.session_sidebar_available() {
                self.request_session_sidebar();
            }
            return;
        }
        self.request_session_popup();
    }

    pub const SESSION_SIDEBAR_W: usize = Self::SESSION_SIDEBAR_WIDTHS[0];
    pub const SESSION_SIDEBAR_WIDTHS: [usize; 3] = [24, 32, 40];
    pub const SESSION_SIDEBAR_MIN_CONTENT: usize = 40;
    pub const SESSION_SIDEBAR_CONTENT_ROWS: usize = 3;
    pub const SESSION_POPUP_W: usize = 30;
    pub const SESSION_POPUP_ROWS: usize = 5;
    pub const SESSION_POPUP_HIT_ID: HitId = 1 << 61;
    pub const SESSION_SIDEBAR_HIT_ID: HitId = 1 << 61 | 1;
    pub const SESSION_SIDEBAR_SEPARATOR_HIT_ID: HitId = 1 << 61 | 2;
    pub const QUEUE_STRIP_HIT_ID: HitId = 1 << 61 | 3;

    pub fn session_sidebar_reserved_cols(&self, width: usize) -> usize {
        let (on, sidebar_w, _) = self.session_sidebar_layout(width);
        if on { sidebar_w } else { 0 }
    }

    pub fn session_sidebar_layout(&self, width: usize) -> (bool, usize, usize) {
        if !self.session_sidebar_on || width_tier(width) < WidthTier::Standard {
            return (false, 0, width);
        }
        let sidebar_w = self.sidebar_width_for(width);
        (true, sidebar_w, width - sidebar_w)
    }

    fn sidebar_width_for(&self, width: usize) -> usize {
        self.session_sidebar_width
            .min(width.saturating_sub(Self::SESSION_SIDEBAR_MIN_CONTENT))
    }

    pub fn session_sidebar_contains(&self, col: usize) -> bool {
        let (on, _, content_w) = self.session_sidebar_layout(self.term_cols);
        on && col >= content_w
    }

    pub fn overlay_hits(&self, width: usize, transcript_rows: usize) -> Vec<Hit> {
        let mut hits = Vec::new();
        hits.extend(self.popup_hit(width, transcript_rows));
        hits.extend(self.queue_strip_hit(width));
        hits.extend(self.sidebar_hits(width, transcript_rows));
        hits
    }

    fn popup_hit(&self, width: usize, transcript_rows: usize) -> Option<Hit> {
        let (_, _, content_w) = self.session_sidebar_layout(width);
        if !self.session_popup_open || content_w == 0 {
            return None;
        }
        let box_w = Self::SESSION_POPUP_W.min(content_w.saturating_sub(2));
        if box_w == 0 {
            return None;
        }
        let start = content_w.saturating_sub(box_w);
        Some(
            Hit::at_cell(Self::SESSION_POPUP_HIT_ID, start as u16, 0)
                .cols(start, content_w)
                .tall(transcript_rows.min(Self::SESSION_POPUP_ROWS))
                .capturing()
                .space(HitSpace::Chrome),
        )
    }

    fn queue_strip_hit(&self, width: usize) -> Option<Hit> {
        if self.last_queue_rows == 0 {
            return None;
        }
        Some(
            Hit::at_cell(
                Self::QUEUE_STRIP_HIT_ID,
                0,
                u16::try_from(self.queue_strip_row).unwrap_or(u16::MAX),
            )
            .cols(0, width)
            .tall(self.last_queue_rows)
            .capturing()
            .space(HitSpace::Chrome),
        )
    }

    fn sidebar_hits(&self, width: usize, transcript_rows: usize) -> Vec<Hit> {
        let (sidebar_on, _, content_w) = self.session_sidebar_layout(width);
        if !sidebar_on {
            return Vec::new();
        }
        vec![
            Hit::at_cell(Self::SESSION_SIDEBAR_HIT_ID, content_w as u16, 0)
                .cols(content_w, width)
                .tall(transcript_rows)
                .capturing()
                .space(HitSpace::Chrome),
            Hit::at_cell(Self::SESSION_SIDEBAR_SEPARATOR_HIT_ID, content_w as u16, 0)
                .cols(content_w, content_w + 1)
                .tall(transcript_rows)
                .capturing()
                .space(HitSpace::Chrome),
        ]
    }

    pub fn take_pending_tree_open(&mut self) -> bool {
        std::mem::take(&mut self.pending.tree_open)
    }

    pub fn request_tree_open(&mut self) {
        self.pending.tree_open = true;
    }

    pub fn take_pending_tree_travel(&mut self) -> Option<String> {
        self.pending.tree_travel.take()
    }

    pub fn take_pending_tree_fork(&mut self) -> Option<String> {
        self.pending.tree_fork.take()
    }

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

    pub fn mount_session_resume_picker(
        &mut self,
        entries: Vec<crate::driver::SessionListEntry>,
        current_id: Option<&str>,
    ) {
        let mut panel = SessionResumePanel::new(self.theme);
        panel.set_current_cwd(&self.cwd);
        panel.load_entries(entries, current_id.map(str::to_string));
        self.slot = EditorSlot::SessionResume(panel);
    }

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

    pub fn mount_import_confirm(&mut self, path: &str) {
        self.slot = EditorSlot::ImportConfirm(ImportSlot::mount(self.theme, path));
    }

    pub fn close_import_confirm(&mut self) {
        if matches!(&self.slot, EditorSlot::ImportConfirm(_)) {
            self.close_slot();
        }
    }

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

    pub fn mount_themes_picker(&mut self, current: Option<&str>) {
        self.slot = EditorSlot::Themes(ThemesSlot::mount(self.theme, current));
    }

    pub fn mount_session_tree(&mut self, roots: Vec<TreeNode>, active_id: Option<&str>) {
        self.slot = EditorSlot::Tree(Box::new(TreeSlot::mount(self.theme, roots, active_id)));
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

    pub fn apply_ui_model(&mut self, model: &UiModel) {
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

    pub fn set_status_next_turn_cue(&mut self, cue: Option<String>) {
        self.status_next_turn_cue = cue.filter(|s| !s.is_empty());
        if self.status_next_turn_cue.is_none() {
            self.refresh_mcp_short_cue();
        }
    }

    pub fn set_suppress_status_right_cue(&mut self, suppress: bool) {
        self.suppress_status_right_cue = suppress;
        if suppress {
            self.status_next_turn_cue = None;
        } else {
            self.refresh_mcp_short_cue();
        }
    }

    pub fn push_toast_notice(&mut self, body: impl Into<String>) {
        self.toast_notice = Some((
            body.into(),
            Instant::now() + crate::commands::TOAST_NOTICE_TTL,
            ToastKind::Error,
        ));
    }

    pub fn todo_bar_expanded(&self) -> bool {
        self.todo_expanded
    }

    #[cfg(test)]
    pub fn todo_bar_expanded_for_test(&mut self, expanded: bool) {
        self.todo_expanded = expanded;
        self.todo_scroll = 0;
    }

    pub fn push_toast_info_notice(&mut self, body: impl Into<String>) {
        self.push_client_notice(ClientNoticeKind::Info, body);
    }

    pub fn render_with_viewport_overlay(&mut self, width: usize) -> Vec<String> {
        let mut lines = Component::render(self, width);
        let transcript_rows = lines.len().saturating_sub(self.last_dock_rows);
        Component::overlay_viewport(self, &mut lines, width, transcript_rows);
        lines
    }

    pub fn toast_notice_body(&self) -> Option<&str> {
        self.toast_notice.as_ref().map(|(b, _, _)| b.as_str())
    }

    #[cfg(test)]
    pub fn expire_toast_notice_now(&mut self) {
        if let Some((body, _, kind)) = self.toast_notice.take() {
            self.toast_notice = Some((
                body,
                Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
                kind,
            ));
        }
    }

    pub(super) fn clear_toast_notice_if_expired(&mut self) -> bool {
        let expired = self
            .toast_notice
            .as_ref()
            .is_some_and(|(_, d, _)| Instant::now() >= *d);
        if expired {
            self.toast_notice = None;
            true
        } else {
            false
        }
    }

    pub(super) fn clear_copy_notice_if_expired(&mut self) -> bool {
        let before = self.client_notices.len();
        let now = Instant::now();
        self.client_notices.retain(|n| now < n.until);
        self.client_notices.len() != before
    }

    pub(crate) fn sync_editor_screen_origin(&mut self) {
        let (row, col) = xylitol_tui::editor_screen_origin(
            self.term_rows.min(u16::MAX as usize) as u16,
            self.last_dock_rows,
            self.last_queue_rows
                .saturating_add(self.last_todo_rows)
                .saturating_add(self.last_toast_rows)
                .saturating_add(self.last_status_rows),
        );
        self.editor.set_screen_origin(row, col);
    }

    pub fn tree_filter_for_test(&self) -> FilterMode {
        match &self.slot {
            EditorSlot::Tree(tree) => tree.filter(),
            _ => FilterMode::Default,
        }
    }

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

    pub fn scrollback_entry_misses_for_test(&self) -> u64 {
        self.scrollback_paint.entry_misses
    }

    pub fn clear_scrollback_entry_misses_for_test(&mut self) {
        self.scrollback_paint.clear_misses();
    }

    pub fn streaming_assistant_full_parses_for_test(&self) -> u64 {
        self.scrollback_paint.streaming_assistant.full_parses
    }

    pub fn streaming_assistant_last_lines_for_test(&self) -> usize {
        self.scrollback_paint.streaming_assistant.last_lines
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::models_picker::ModelPickerRow;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn enter() -> crossterm::event::KeyEvent {
        key(KeyCode::Enter)
    }

    fn plain(line: &str) -> String {
        xylitol_tui::utils::strip_ansi_codes(line)
    }

    #[test]
    fn footer_chips_render_details_and_sidebar_at_standard_tier() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let frame = root.render(120);
        let chip_row = frame
            .iter()
            .map(|l| plain(l))
            .position(|l| l.contains("[详情]"))
            .expect("details chip row");
        let line = plain(&frame[chip_row]);
        assert!(line.contains("[详情]") && line.contains("[侧栏]"), "{line}");
        let expected_screen_row = root
            .term_rows
            .saturating_sub(root.last_editor_rows)
            .saturating_sub(1);
        assert_eq!(Some(expected_screen_row), root.last_chip_row);
    }

    #[test]
    fn narrow_terminal_hides_sidebar_chip() {
        let mut root = UiRoot::new();
        root.set_term_cols(70);
        let frame = root.render(70);
        let joined: String = frame.iter().map(|l| plain(l)).collect();
        assert!(joined.contains("[详情]"), "{joined}");
        assert!(
            !joined.contains("[侧栏]"),
            "sidebar chip needs standard tier"
        );
    }

    #[test]
    fn footer_chip_click_toggles_sidebar() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let _ = root.render(120);
        let actions = root.footer_chip_actions_for_test();
        assert!(
            actions
                .iter()
                .any(|(a, _, _)| *a == super::footer_chips::FooterAction::Sidebar),
            "standard tier paints the sidebar chip"
        );
        assert!(
            root.click_footer_action(super::footer_chips::FooterAction::Sidebar),
            "chip must consume"
        );
        assert!(root.take_pending_session_sidebar(), "request queued");
    }

    #[test]
    fn slot_row_click_selects_then_commits_on_second_click() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.mount_models_picker(vec![
            ModelPickerRow {
                id: "m-a".into(),
                label: "A".into(),
                levels: Vec::new(),
                provisional: "off".into(),
            },
            ModelPickerRow {
                id: "m-b".into(),
                label: "B".into(),
                levels: Vec::new(),
                provisional: "off".into(),
            },
        ]);
        let _ = root.render(120);
        let top = root.last_slot_top_row.expect("slot top");
        assert!(root.handle_slot_row_click_for_test(top as u16 + 2));
        assert!(
            root.pending.model_select.is_none(),
            "first click only selects"
        );
        let _ = root.render(120);
        assert!(root.handle_slot_row_click_for_test(top as u16 + 2));
        assert!(
            root.pending.model_select.is_some(),
            "second click commits the row action"
        );
    }

    #[test]
    fn slot_close_chip_closes_overlay() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.slot =
            EditorSlot::Themes(ThemesSlot::mount(LayoutTheme::product_dark(), Some("dark")));
        let _ = root.render(120);
        let row = root.last_slot_top_row.expect("slot top row");
        let (start, _end) = root.last_slot_close_hit.expect("close chip hit");
        assert!(root.handle_close_chip_for_test(start, row));
        assert!(root.slot().is_editor(), "close chip restores editor");
    }

    #[test]
    fn footer_reset_chip_clears_overrides() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.toggle_fold_target(FoldTarget::Tool("t".into()));
        let _ = root.render(120);
        assert!(root.click_footer_action(super::footer_chips::FooterAction::ResetFold));
        assert!(root.fold().tools_overrides.is_empty());
    }

    #[test]
    fn sidebar_layout_gated_on_standard_tier() {
        let mut root = UiRoot::new();
        root.show_session_sidebar("s1");
        assert_eq!(root.session_sidebar_layout(79), (false, 0, 79));
        assert_eq!(root.session_sidebar_layout(80), (true, 24, 56));
        assert_eq!(root.session_sidebar_reserved_cols(79), 0);
        assert_eq!(root.session_sidebar_reserved_cols(80), 24);
        root.close_session_sidebar();
        assert_eq!(root.session_sidebar_layout(120), (false, 0, 120));
        assert_eq!(root.session_sidebar_reserved_cols(120), 0);
    }

    #[test]
    fn sidebar_width_cycles_and_keeps_min_content() {
        let mut root = UiRoot::new();
        root.set_term_cols(80);
        root.show_session_sidebar("s1");
        assert_eq!(root.session_sidebar_layout(80), (true, 24, 56));
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(root.session_sidebar_layout(80), (true, 32, 48));
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(root.session_sidebar_layout(80), (true, 40, 40));
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(root.session_sidebar_layout(80), (true, 24, 56));
        assert_eq!(root.session_sidebar_layout(79), (false, 0, 79));
    }

    #[test]
    fn sidebar_contains_respects_content_edge() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.show_session_sidebar("s1");
        assert!(!root.session_sidebar_contains(95));
        assert!(root.session_sidebar_contains(96));
        assert!(root.session_sidebar_contains(119));
    }

    #[test]
    fn sidebar_scroll_clamps_to_placeholder_content() {
        let mut root = UiRoot::new();
        assert_eq!(root.session_sidebar_content_rows(), 3);
        assert!(
            !root.scroll_session_sidebar(1, 24),
            "content that fits must not scroll"
        );
        assert!(root.scroll_session_sidebar(1, 1));
        assert_eq!(root.session_sidebar_scroll(), 1);
        assert!(root.scroll_session_sidebar(10, 1));
        assert_eq!(root.session_sidebar_scroll(), 2);
        assert!(!root.scroll_session_sidebar(1, 1));
        assert!(root.scroll_session_sidebar(-10, 1));
        assert_eq!(root.session_sidebar_scroll(), 0);
    }

    #[test]
    fn details_cycle_walks_popup_sidebar_off() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.cycle_session_details();
        assert!(root.take_pending_session_popup());
        root.show_session_popup("s1");

        root.cycle_session_details();
        assert!(!root.session_popup_open());
        assert!(root.take_pending_session_sidebar());
        root.show_session_sidebar("s1");

        root.cycle_session_details();
        assert!(!root.session_sidebar_on());
    }

    #[test]
    fn details_cycle_degrades_on_narrow_terminal() {
        let mut root = UiRoot::new();
        root.set_term_cols(78);
        root.cycle_session_details();
        assert!(root.take_pending_session_popup());
        root.show_session_popup("s1");

        root.cycle_session_details();
        assert!(!root.session_popup_open());
        assert!(
            !root.take_pending_session_sidebar(),
            "narrow terminal must not request an unavailable sidebar"
        );
    }

    #[test]
    fn overlay_hits_cover_queue_strip() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.ui_model.enqueue_steer_strip("queued".into());
        root.last_queue_rows = 3;
        root.queue_strip_row = 7;
        let hits = root.overlay_hits(120, 40);
        let hit = hits
            .iter()
            .find(|hit| (*hit).into_region().id == UiRoot::QUEUE_STRIP_HIT_ID)
            .expect("queue strip hit");
        let region = (*hit).into_region();
        assert_eq!(region.space, HitSpace::Chrome);
        assert_eq!(region.content_row, 7);
        assert_eq!(region.col_start, 0);
        assert_eq!(region.col_end, 120);
        assert_eq!(region.height, 3);
        assert!(region.capturing);
    }

    #[test]
    fn empty_queue_strip_has_no_hit() {
        let root = UiRoot::new();
        let hits = root.overlay_hits(120, 40);
        assert!(
            hits.iter()
                .all(|hit| (*hit).into_region().id != UiRoot::QUEUE_STRIP_HIT_ID)
        );
    }

    #[test]
    fn overlay_hits_cover_popup_sidebar_and_separator() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.show_session_popup("s1");
        root.show_session_sidebar("s1");
        let hits = root.overlay_hits(120, 40);
        let ids: Vec<_> = hits.iter().map(|hit| (*hit).into_region().id).collect();
        assert!(ids.contains(&UiRoot::SESSION_POPUP_HIT_ID));
        assert!(ids.contains(&UiRoot::SESSION_SIDEBAR_HIT_ID));
        assert!(ids.contains(&UiRoot::SESSION_SIDEBAR_SEPARATOR_HIT_ID));
        for hit in &hits {
            let region = (*hit).into_region();
            assert_eq!(region.space, HitSpace::Chrome);
            assert!(region.capturing, "chrome hits must capture the pointer");
        }
    }

    fn fake_row() -> ModelPickerRow {
        ModelPickerRow {
            id: "fake".into(),
            label: "Fake".into(),
            levels: Vec::new(),
            provisional: "off".into(),
        }
    }

    #[test]
    fn models_esc_close_cancels_recorded_confirm() {
        let mut root = UiRoot::new();
        root.mount_models_picker(vec![fake_row()]);
        root.handle_key(enter());
        assert!(
            root.pending.model_select.is_some(),
            "Enter must record the picker confirm"
        );
        root.on_escape();
        assert!(
            root.take_pending_model_select().is_none(),
            "Esc close must cancel the recorded model confirm"
        );
    }

    #[test]
    fn themes_esc_close_cancels_recorded_confirm() {
        let mut root = UiRoot::new();
        root.mount_themes_picker(Some("dark"));
        root.handle_key(enter());
        assert!(
            root.pending.theme_select.is_some(),
            "Enter must record the theme confirm"
        );
        root.on_escape();
        assert!(
            root.take_pending_theme_select().is_none(),
            "Esc close must cancel the recorded theme confirm"
        );
    }

    #[test]
    fn models_enter_flags_confirmed_row_in_frame() {
        use xylitol_tui::Component;
        let mut root = UiRoot::new();
        root.mount_models_picker(vec![fake_row()]);
        root.handle_key(enter());
        let frame = root.render(80).join("\n");
        assert!(
            frame.contains("✓ 已选 Fake"),
            "Enter must flag the confirmed model row: {frame}"
        );
    }

    #[test]
    fn themes_enter_flags_confirmed_row_in_frame() {
        use xylitol_tui::Component;
        let mut root = UiRoot::new();
        root.mount_themes_picker(Some("dark"));
        root.handle_key(enter());
        let frame = root.render(80).join("\n");
        assert!(
            frame.contains("✓ 已选 dark"),
            "Enter must flag the confirmed theme row: {frame}"
        );
    }

    #[test]
    fn client_notice_stack_right_aligns_copied_not_error_bar() {
        use xylitol_tui::utils::strip_ansi_codes;
        let mut root = UiRoot::new();
        root.arm_copy_notice();
        let frame = root.render_with_viewport_overlay(80);
        let joined = frame.join("\n");
        let first = strip_ansi_codes(&frame[0]);
        assert!(
            first.contains("✓ Copied"),
            "Copied must overlay top-right: {first:?}"
        );
        let trimmed = first.trim_start();
        assert!(
            first.ends_with("Copied") || trimmed.contains("✓ Copied"),
            "notice should sit on the right: {first:?}"
        );
        assert!(!joined.contains("Error: Copied"));
        assert!(root.toast_notice_body().is_none());
        let dock_before = root.last_dock_rows();
        root.push_client_notice(ClientNoticeKind::Info, "hint");
        let _ = root.render_with_viewport_overlay(80);
        assert_eq!(
            root.last_dock_rows(),
            dock_before,
            "notice stack must not grow dock footprint"
        );
    }

    #[test]
    fn client_notice_stack_caps_at_three_newest_first() {
        use xylitol_tui::utils::strip_ansi_codes;
        let mut root = UiRoot::new();
        root.push_client_notice(ClientNoticeKind::Info, "n1");
        root.push_client_notice(ClientNoticeKind::Info, "n2");
        root.push_client_notice(ClientNoticeKind::Info, "n3");
        root.push_client_notice(ClientNoticeKind::Success, "n4");
        let frame = strip_ansi_codes(&root.render_with_viewport_overlay(80).join("\n"));
        assert!(frame.contains("✓ n4"), "{frame}");
        assert!(frame.contains("◆ n3"), "{frame}");
        assert!(frame.contains("◆ n2"), "{frame}");
        assert!(!frame.contains("n1"), "oldest dropped: {frame}");
    }
}
