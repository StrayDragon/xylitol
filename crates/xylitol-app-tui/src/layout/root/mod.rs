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
mod sidebar_panel;
mod slot_input;
mod slot_nav;
mod theme_apply;

use pending_slot::PendingSlotOps;
use sidebar_panel::SidebarPanel;
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
    PanelBounds, PanelOverflow, PanelPlacement, PanelRegion, ScreenRect, SizeValue,
    SlashArgCompletionSource, SlashCommandSource, SystemClock, TreeNode, fg_rgb, mix_rgb,
};

use super::dollar_skill_source::DollarSkillSource;
use super::mcp_source::{McpPickSink, McpResourceSource};
use super::models_picker::PendingModelChoice;
use super::slash_catalog::slash_commands_for_editor;

use super::session_tree::FilterMode;
use super::slots::{AskSlot, EditorSlot, EditorSlotKind, ImportSlot, ThemesSlot, TreeSlot};
use super::theme::LayoutTheme;
use super::{WidthTier, width_tier};
use crate::activity_fold::{
    ActivityFoldState, AutoTrigger, cluster_is_thought_only, ingest_rebuild_clocks,
    partition_segments,
};
use crate::bridge::{STREAMING_THINK_ID, UiEntry, UiModel};
use crate::driver::LoadedResourcesSnapshot;
use crate::mcp_attach::AttachmentTray;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrayChipHit {
    col_start: usize,
    col_end: usize,
    index: usize,
}

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
    session_sidebar_on: bool,
    session_sidebar_width: usize,
    session_sidebar_scroll: usize,
    session_sidebar_panel: PanelRegion,
    session_sidebar_panel_lines: Vec<String>,
    session_sidebar_panel_rect: Option<ScreenRect>,
    session_sidebar_height_cap: usize,
    session_sidebar_dragging: bool,
    session_sidebar_drag_moved: bool,
    session_sidebar_drag_start_col: u16,
    session_sidebar_drag_start_width: usize,
    session_sidebar_layout_dirty: bool,
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
    mcp_tray: AttachmentTray,
    mcp_catalog: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    mcp_pick_sink: McpPickSink,
    last_tray_rows: usize,
    tray_strip_row: usize,
    last_tray_hits: Vec<TrayChipHit>,
    todo_selection: xylitol_tui::SelectionController,
    todo_clipboard: Vec<String>,
    todo_pointer_dirty: bool,
    slot_pointer_dirty: bool,
    client_notices: VecDeque<ClientNotice>,
    #[cfg(test)]
    upper_rebuild_count: u64,
}

fn find_fd() -> Option<String> {
    for name in ["fd", "fdfind"] {
        let status = std::process::Command::new(name)
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return Some(name.to_string());
        }
    }
    None
}

fn snapshot_has_mcp(snap: &LoadedResourcesSnapshot) -> bool {
    snap.mcp_configured > 0
        || !snap.mcp_connected.is_empty()
        || !snap.mcp_servers.is_empty()
        || snap.mcp_connecting_label.is_some()
        || !snap.mcp_diag_short.is_empty()
        || !snap.mcp_details.is_empty()
}

impl UiRoot {
    pub fn new() -> Self {
        let theme = LayoutTheme::product_dark();
        let accent = theme.palette().accent;
        let dim = mix_rgb(accent, theme.palette().surface, 0.55);
        let muted = theme.palette().muted;
        let pulse_start = Instant::now();
        let status_loader = Loader::new(
            Box::new(move |s| {
                let phase =
                    (pulse_start.elapsed().as_secs_f32() * std::f32::consts::TAU / 1.2).sin() * 0.5
                        + 0.5;
                fg_rgb(mix_rgb(dim, accent, phase), s)
            }),
            Box::new(move |s| fg_rgb(muted, s)),
            String::new(),
            Some(LoaderIndicatorOptions {
                frames: vec!["⬢".into()],
                interval_ms: 100,
            }),
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
            session_sidebar_on: false,
            session_sidebar_width: Self::SESSION_SIDEBAR_WIDTHS[0],
            session_sidebar_scroll: 0,
            session_sidebar_panel: PanelRegion::new(
                Box::new(SidebarPanel::new(theme)),
                PanelBounds {
                    width: SizeValue::Absolute(Self::SESSION_SIDEBAR_WIDTHS[0]),
                    min_width: Self::SESSION_SIDEBAR_WIDTHS[0],
                    max_width: Some(Self::SESSION_SIDEBAR_WIDTHS[0]),
                    min_height: Self::SESSION_SIDEBAR_CONTENT_ROWS,
                    max_height: SizeValue::Percent(60.0),
                    overflow: PanelOverflow::Scroll,
                },
                PanelPlacement::DockRight,
            ),
            session_sidebar_panel_lines: Vec::new(),
            session_sidebar_panel_rect: None,
            session_sidebar_height_cap: 24,
            session_sidebar_dragging: false,
            session_sidebar_drag_moved: false,
            session_sidebar_drag_start_col: 0,
            session_sidebar_drag_start_width: Self::SESSION_SIDEBAR_WIDTHS[0],
            session_sidebar_layout_dirty: false,
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
            mcp_tray: AttachmentTray::default(),
            mcp_catalog: Vec::new(),
            mcp_pick_sink: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            last_tray_rows: 0,
            tray_strip_row: 0,
            last_tray_hits: Vec::new(),
            todo_selection: xylitol_tui::SelectionController::new(),
            todo_clipboard: Vec::new(),
            todo_pointer_dirty: false,
            slot_pointer_dirty: false,
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
            Box::new(
                SlashArgCompletionSource::new(
                    "mcp",
                    vec![
                        ("prompt".into(), "Pick an MCP prompt into the editor".into()),
                        (
                            "resource".into(),
                            "Attach an MCP resource to the next message".into(),
                        ),
                    ],
                )
                .with_id("mcp-panel"),
            ),
            Box::new(McpResourceSource::new(
                self.mcp_catalog.clone(),
                self.mcp_pick_sink.clone(),
            )),
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
            Box::new(AtPathSource::new_with_fd(
                self.at_path_base.clone(),
                find_fd(),
            )),
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

    pub fn set_mcp_catalog_entries(
        &mut self,
        entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    ) {
        let changed = self.mcp_catalog.len() != entries.len()
            || self
                .mcp_catalog
                .iter()
                .zip(entries.iter())
                .any(|(a, b)| a.server != b.server || a.uri != b.uri || a.name != b.name);
        if !changed {
            return;
        }
        self.mcp_catalog = entries;
        self.install_completion_sources();
    }

    pub fn mcp_tray_add(&mut self, server: &str, uri: &str) -> bool {
        let added = self.mcp_tray.add(server, uri);
        if added {
            self.invalidate();
        }
        added
    }

    pub fn mcp_tray_remove_last(&mut self) -> bool {
        let removed = self.mcp_tray.remove_last();
        if removed {
            self.invalidate();
        }
        removed
    }

    pub fn mcp_tray_remove_at(&mut self, index: usize) -> bool {
        let removed = self.mcp_tray.remove_at(index);
        if removed {
            self.invalidate();
        }
        removed
    }

    pub fn mcp_tray_clear(&mut self) -> bool {
        let cleared = self.mcp_tray.clear();
        if cleared {
            self.invalidate();
        }
        cleared
    }

    pub fn mcp_tray_is_empty(&self) -> bool {
        self.mcp_tray.is_empty()
    }

    pub fn mcp_tray_len(&self) -> usize {
        self.mcp_tray.len()
    }

    pub fn mcp_tray_refs(&self) -> Vec<xylitol_plugin_sdk::mcp::McpResourceRef> {
        self.mcp_tray.refs().to_vec()
    }

    pub fn drain_mcp_completion_picks(&mut self) -> usize {
        super::mcp_source::drain_picks_into_tray(&self.mcp_pick_sink, &mut self.mcp_tray)
    }

    pub fn set_plugin_commands(&mut self, commands: Vec<(String, String)>) {
        if self.plugin_commands == commands {
            return;
        }
        self.plugin_commands = commands;
        self.install_completion_sources();
    }

    pub fn set_loaded_resources(&mut self, mut snap: LoadedResourcesSnapshot) {
        let prev = &self.loaded_resources;
        let prev_has_mcp = snapshot_has_mcp(prev);
        let next_has_mcp = snapshot_has_mcp(&snap);
        if prev_has_mcp && !next_has_mcp && !snap.mcp_bootstrap_complete {
            snap.mcp_configured = prev.mcp_configured;
            snap.mcp_connected = prev.mcp_connected.clone();
            snap.mcp_servers = prev.mcp_servers.clone();
            snap.mcp_diag_short = prev.mcp_diag_short.clone();
            snap.mcp_details = prev.mcp_details.clone();
            snap.mcp_connecting_label = prev.mcp_connecting_label.clone();
            snap.mcp_bootstrap_complete = prev.mcp_bootstrap_complete;
            snap.tools_table_frozen = prev.tools_table_frozen;
        }
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
        let thinking_full = matches!(
            settings.display_level,
            crate::activity_fold::DisplayLevel::Recent | crate::activity_fold::DisplayLevel::Full
        );
        self.fold.apply_visible_items(settings.visible_items);
        self.fold.thinking_expanded = thinking_full;
        self.fold.streaming_thinking_expanded = thinking_full;
        self.activity.settings = settings;
    }

    pub fn cycle_display_level(&mut self) {
        let mut settings = self.activity.settings.clone();
        settings.apply_display_level(settings.display_level.next());
        self.set_activity_settings(settings);
        self.bump_upper_gen();
    }

    pub fn toggle_args_default(&mut self) {
        self.fold.args_expanded = match self.fold.args_expanded {
            crate::widgets::ArgsMode::Hidden => crate::widgets::ArgsMode::Preview,
            crate::widgets::ArgsMode::Preview => crate::widgets::ArgsMode::Full,
            crate::widgets::ArgsMode::Full => crate::widgets::ArgsMode::Hidden,
        };
        self.bump_upper_gen();
    }

    pub fn reset_fold_to_initial(&mut self) -> bool {
        let before = self.fold.clone();
        self.reset_fold_overrides();
        self.fold != before
    }

    pub fn reset_fold_overrides(&mut self) {
        self.fold.clear_output_overrides();
        self.fold.clear_args_overrides();
        self.fold.clear_user_overrides();
        self.fold.clear_tools_overrides();
        self.fold.clear_thinking_overrides();
        let mut settings = self.activity.settings.clone();
        settings.apply_display_level(crate::activity_fold::DisplayLevel::Summary);
        self.set_activity_settings(settings);
        self.bump_upper_gen();
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
        let degraded =
            self.activity
                .auto_degrade(&self.ui_model.entries, AutoTrigger::TurnEnd, false);
        let expanded = matches!(
            self.activity.settings.display_level,
            crate::activity_fold::DisplayLevel::Recent
        ) && self.activity.expand_nearest(&self.ui_model.entries);
        if degraded || expanded {
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
            FoldTarget::ToolArgs(id) => {
                self.fold.toggle_args(&id);
            }
            FoldTarget::Thinking(id) => {
                self.fold.toggle_thinking(&id);
            }
            FoldTarget::User(id) => {
                self.fold.toggle_user(&id);
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
        if !self.todo_hit_at(col, row_in_bar) {
            return false;
        }
        self.toggle_todo_expanded();
        true
    }

    fn todo_hit_at(&self, col: usize, row_in_bar: usize) -> bool {
        self.last_todo_hits
            .iter()
            .any(|h| h.row == row_in_bar && col >= h.col_start && col < h.col_end)
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
        )
        .saturating_add(self.last_tray_rows)
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

    pub fn session_sidebar_on(&self) -> bool {
        self.session_sidebar_on
    }

    pub fn close_session_sidebar(&mut self) {
        self.session_sidebar_on = false;
        self.session_sidebar_scroll = 0;
        self.session_sidebar_panel.scroll_to_top();
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

    pub fn show_session_sidebar(&mut self, _session_id: &str) {
        self.session_sidebar_on = true;
        self.session_sidebar_scroll = 0;
        self.session_sidebar_panel.scroll_to_top();
    }

    pub fn set_term_cols(&mut self, cols: usize) {
        self.term_cols = cols;
    }

    pub fn session_sidebar_available(&self) -> bool {
        self.session_sidebar_on && self.session_sidebar_layout(self.term_cols).0.is_some()
    }

    pub fn session_sidebar_width(&self) -> usize {
        self.session_sidebar_width
    }

    pub fn session_sidebar_height_cap(&self) -> usize {
        self.session_sidebar_height_cap
    }

    pub fn take_session_sidebar_layout_dirty(&mut self) -> bool {
        std::mem::take(&mut self.session_sidebar_layout_dirty)
    }

    pub fn take_session_sidebar_drag_moved(&mut self) -> bool {
        std::mem::take(&mut self.session_sidebar_drag_moved)
    }

    pub fn set_session_sidebar_layout(&mut self, width: usize, height_cap: usize) {
        self.session_sidebar_width = width.max(Self::SESSION_SIDEBAR_WIDTHS[0]);
        self.session_sidebar_height_cap = height_cap.clamp(Self::SESSION_SIDEBAR_CONTENT_ROWS, 24);
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
            self.session_sidebar_layout_dirty = true;
            true
        }
    }

    pub fn session_sidebar_scroll(&self) -> usize {
        self.session_sidebar_scroll
    }

    pub fn session_sidebar_content_rows(&self) -> usize {
        self.session_sidebar_panel
            .content_rows()
            .max(Self::SESSION_SIDEBAR_CONTENT_ROWS)
    }

    pub fn scroll_session_sidebar(&mut self, delta: isize, visible_rows: usize) -> bool {
        let content_rows = self.session_sidebar_content_rows();
        let max_scroll = content_rows.saturating_sub(visible_rows.max(1));
        let current = self.session_sidebar_panel.scroll_top();
        let next = (current as isize + delta).clamp(0, max_scroll as isize) as usize;
        if next == current {
            false
        } else {
            self.session_sidebar_scroll = next;
            self.session_sidebar_panel.set_scroll(next);
            true
        }
    }

    pub const SESSION_SIDEBAR_W: usize = Self::SESSION_SIDEBAR_WIDTHS[0];
    pub const SESSION_SIDEBAR_WIDTHS: [usize; 3] = [24, 32, 40];
    pub const SESSION_SIDEBAR_MIN_CONTENT: usize = 40;
    pub const SESSION_SIDEBAR_CONTENT_ROWS: usize = 3;
    pub const SESSION_SIDEBAR_HIT_ID: HitId = 1 << 61 | 1;
    pub const SESSION_SIDEBAR_SEPARATOR_HIT_ID: HitId = 1 << 61 | 2;
    pub const QUEUE_STRIP_HIT_ID: HitId = 1 << 61 | 3;
    pub const TODO_SUMMARY_HOVER_HIT_ID: HitId = 1 << 61 | 4;

    pub fn session_sidebar_reserved_cols(&self, width: usize) -> usize {
        let (placement, panel_w, _, _) = self.session_sidebar_layout(width);
        if matches!(placement, Some(PanelPlacement::DockRight)) {
            panel_w
        } else {
            0
        }
    }

    pub fn session_sidebar_layout(
        &self,
        width: usize,
    ) -> (Option<PanelPlacement>, usize, usize, usize) {
        let width = width.max(1);
        let transcript = self.transcript_rows();
        if !self.session_sidebar_on || transcript < Self::SESSION_SIDEBAR_CONTENT_ROWS {
            return (None, 0, width, 0);
        }
        if width_tier(width) >= WidthTier::Standard {
            let panel_w = self.session_sidebar_width_for(width);
            let content_w = width.saturating_sub(panel_w);
            if content_w >= Self::SESSION_SIDEBAR_MIN_CONTENT {
                return (
                    Some(PanelPlacement::DockRight),
                    panel_w,
                    content_w,
                    self.session_sidebar_height(transcript),
                );
            }
        }
        if width >= 26 {
            let panel_w = self.session_sidebar_width.clamp(
                Self::SESSION_SIDEBAR_WIDTHS[0],
                width.saturating_sub(2).max(Self::SESSION_SIDEBAR_WIDTHS[0]),
            );
            return (
                Some(PanelPlacement::FloatRight),
                panel_w,
                width,
                self.session_sidebar_height(transcript),
            );
        }
        (None, 0, width, 0)
    }

    fn session_sidebar_width_for(&self, width: usize) -> usize {
        self.session_sidebar_width.clamp(
            Self::SESSION_SIDEBAR_WIDTHS[0],
            width
                .saturating_sub(Self::SESSION_SIDEBAR_MIN_CONTENT)
                .max(Self::SESSION_SIDEBAR_WIDTHS[0]),
        )
    }

    fn session_sidebar_height(&self, transcript_rows: usize) -> usize {
        let pct = transcript_rows.saturating_mul(60) / 100;
        pct.min(24)
            .min(
                self.session_sidebar_height_cap
                    .max(Self::SESSION_SIDEBAR_CONTENT_ROWS),
            )
            .max(Self::SESSION_SIDEBAR_CONTENT_ROWS)
            .min(transcript_rows)
            .max(1)
    }

    fn transcript_rows(&self) -> usize {
        self.term_rows
            .saturating_sub(self.last_dock_rows.max(1))
            .max(1)
    }

    pub fn session_sidebar_contains(&self, col: usize) -> bool {
        if let Some(rect) = self.session_sidebar_panel_rect {
            return col >= rect.col as usize && col < rect.col as usize + rect.width as usize;
        }
        let (placement, panel_w, content_w, _) = self.session_sidebar_layout(self.term_cols);
        match placement {
            Some(PanelPlacement::DockRight) => col >= content_w,
            Some(PanelPlacement::FloatRight) => col >= self.term_cols.saturating_sub(panel_w),
            None | Some(PanelPlacement::DockLeft) | Some(PanelPlacement::FloatLeft) => false,
        }
    }

    pub fn overlay_hits(&self, width: usize, transcript_rows: usize) -> Vec<Hit> {
        let mut hits = Vec::new();

        hits.extend(self.queue_strip_hit(width));
        hits.extend(self.sidebar_hits(width, transcript_rows));
        hits
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

    pub fn overlay_hover_hits(&self, width: usize) -> Vec<Hit> {
        let mut hits = Vec::new();
        if let Some(hit) = self.queue_strip_hover_hit(width) {
            hits.push(hit);
        }
        if let Some(hit) = self.todo_summary_hover_hit() {
            hits.push(hit);
        }
        hits
    }

    fn queue_strip_hover_hit(&self, width: usize) -> Option<Hit> {
        if self.last_queue_rows == 0 {
            return None;
        }
        Some(
            Hit::at_screen(
                Self::QUEUE_STRIP_HIT_ID,
                0,
                u16::try_from(self.queue_strip_row).unwrap_or(u16::MAX),
            )
            .cols(0, width)
            .tall(self.last_queue_rows),
        )
    }

    fn todo_summary_hover_hit(&self) -> Option<Hit> {
        let (row, end) = self.todo_summary_row_and_cols()?;
        Some(
            Hit::at_screen(Self::TODO_SUMMARY_HOVER_HIT_ID, 0, u16::try_from(row).ok()?)
                .cols(0, end),
        )
    }

    fn todo_summary_row_and_cols(&self) -> Option<(usize, usize)> {
        if self.last_todo_rows == 0 {
            return None;
        }
        let summary = self.last_todo_hits.first()?;
        let dock = self.last_dock_rows.max(1).min(self.term_rows);
        let dock_top = self.term_rows.saturating_sub(dock);
        let row = dock_top
            .saturating_add(self.last_queue_rows)
            .saturating_add(summary.row);
        let end = self
            .last_todo_plain
            .get(summary.row)
            .map(|line| xylitol_tui::visible_width(line).max(summary.col_end))
            .unwrap_or(summary.col_end)
            .max(1);
        Some((row, end))
    }

    fn sidebar_hits(&self, width: usize, transcript_rows: usize) -> Vec<Hit> {
        let (placement, panel_w, content_w, panel_h) = self.session_sidebar_layout(width);
        let Some(placement) = placement else {
            return Vec::new();
        };
        let (rect, body_height) = if let Some(rect) = self.session_sidebar_panel_rect {
            let body_height = (rect.height as usize).min(transcript_rows);
            (rect, body_height)
        } else {
            let col = match placement {
                PanelPlacement::DockRight => content_w,
                PanelPlacement::FloatRight => width.saturating_sub(panel_w),
                _ => return Vec::new(),
            };
            let rect = ScreenRect {
                row: 0,
                col: col.min(u16::MAX as usize) as u16,
                height: panel_h.min(transcript_rows).min(u16::MAX as usize) as u16,
                width: panel_w.min(u16::MAX as usize) as u16,
            };
            (rect, panel_h.min(transcript_rows))
        };
        let separator_height = match placement {
            PanelPlacement::DockRight => transcript_rows,
            PanelPlacement::FloatRight => body_height,
            _ => body_height,
        };
        let mut hits = Vec::new();
        if body_height > 0 {
            hits.push(
                Hit::at_cell(Self::SESSION_SIDEBAR_HIT_ID, rect.col, rect.row)
                    .cols(rect.col as usize, rect.col as usize + rect.width as usize)
                    .tall(body_height)
                    .capturing()
                    .space(HitSpace::Chrome),
            );
        }
        if separator_height > 0 {
            hits.push(
                Hit::at_cell(Self::SESSION_SIDEBAR_SEPARATOR_HIT_ID, rect.col, rect.row)
                    .cols(rect.col as usize, rect.col as usize + 1)
                    .tall(separator_height)
                    .capturing()
                    .space(HitSpace::Chrome),
            );
        }
        hits
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

    pub fn pending_take_mcp_prompt_select(&mut self) -> Option<(String, String)> {
        self.pending.mcp_prompt_select.take()
    }

    pub fn pending_take_mcp_oauth_start(&mut self) -> Option<String> {
        self.pending.mcp_oauth_start.take()
    }

    pub fn pending_take_mcp_resource_add(
        &mut self,
    ) -> Option<xylitol_plugin_sdk::mcp::McpResourceRef> {
        self.pending.mcp_resource_add.take()
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
        let prev_live_think = self.ui_model.streaming_think_id.clone();
        let upper_changed = self.ui_model.entries != model.entries
            || self.ui_model.streaming_assistant != model.streaming_assistant
            || self.ui_model.streaming_thinking != model.streaming_thinking
            || self.ui_model.streaming_think_id != model.streaming_think_id
            || self.ui_model.phase != model.phase;
        self.ui_model = model.clone();
        if upper_changed {
            self.bump_upper_gen();
        }
        if prev_live_think.as_deref() == Some(STREAMING_THINK_ID)
            && model.streaming_think_id.is_none()
        {
            self.promote_streaming_think_fold();
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

    fn promote_streaming_think_fold(&mut self) {
        let Some(expanded) = self.fold.thinking_overrides.remove(STREAMING_THINK_ID) else {
            return;
        };
        let Some(id) = self.ui_model.entries.iter().rev().find_map(|e| match e {
            UiEntry::Thinking { id, .. } => Some(id.clone()),
            _ => None,
        }) else {
            return;
        };
        self.fold.thinking_overrides.insert(id, expanded);
        if expanded {
            let segs = partition_segments(&self.ui_model.entries);
            if let Some(seg) = segs.last()
                && let Some(cl) = seg.clusters.last()
                && cluster_is_thought_only(&self.ui_model.entries, cl)
            {
                let _ = self.activity.expand_live_cluster(&self.ui_model.entries);
            }
        }
        self.fold_dirty = true;
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

    pub fn todo_summary_row_for_test(&self) -> Option<usize> {
        self.last_todo_hits.first().map(|hit| hit.row)
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
                .saturating_add(self.last_status_rows)
                .saturating_add(self.last_tray_rows),
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
    fn footer_chips_render_marker_and_fold_summary_at_standard_tier() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let frame = root.render(120);
        let joined: String = frame.iter().map(|l| plain(l)).collect();
        assert!(!joined.contains("[侧栏]"), "{joined}");
        assert!(joined.contains("◨"), "{joined}");
        assert!(joined.contains("[步骤显示级别: 摘要]"), "{joined}");
        assert!(!joined.contains("[详情]"), "{joined}");
        let expected_screen_row = root
            .term_rows
            .saturating_sub(root.last_editor_rows)
            .saturating_sub(1);
        assert_eq!(Some(expected_screen_row), root.last_chip_row);
    }

    #[test]
    fn narrow_terminal_hides_details_chip() {
        let mut root = UiRoot::new();
        root.set_term_cols(70);
        let frame = root.render(70);
        let joined: String = frame.iter().map(|l| plain(l)).collect();
        assert!(!joined.contains("[详情]"), "{joined}");
        assert!(!joined.contains("[侧栏]"), "{joined}");
        assert!(!joined.contains('◨'), "{joined}");
        assert!(joined.contains("[步骤显示级别: 摘要]"), "{joined}");
    }

    #[test]
    fn fold_summary_chip_cycles_display_level() {
        use crate::activity_fold::DisplayLevel;
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let _ = root.render(120);
        assert_eq!(
            root.activity().settings.display_level,
            DisplayLevel::Summary
        );
        assert!(root.click_footer_action(super::footer_chips::FooterAction::FoldSummary));
        assert_eq!(root.activity().settings.display_level, DisplayLevel::Recent);
    }

    #[test]
    fn marker_click_toggles_sidebar() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let lines = root.render(120);
        let line = lines
            .iter()
            .find(|line| plain(line).contains('◨'))
            .expect("marker row");
        let width = xylitol_tui::visible_width(&plain(line));
        assert!(
            root.try_click_painted_chip(width.saturating_sub(2), line),
            "marker must consume"
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
    fn right_click_summary_resets_fold() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.toggle_fold_target(FoldTarget::Tool("t".into()));
        let lines = root.render(120);
        let line = lines
            .iter()
            .find(|line| {
                xylitol_tui::utils::strip_ansi_codes(line).contains("[步骤显示级别: 摘要]")
            })
            .expect("summary chip row");
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let col = xylitol_tui::visible_width(&plain[..plain.find("[步骤显示级别: 摘要]").unwrap()]);
        assert!(root.try_right_click_summary(col, line));
        assert!(root.fold().tools_overrides.is_empty());
        assert!(
            root.client_notices
                .iter()
                .any(|notice| notice.body.contains("重置折叠成功")),
            "reset must notify"
        );
    }

    #[test]
    fn sidebar_layout_gated_on_standard_tier() {
        let mut root = UiRoot::new();
        root.show_session_sidebar("s1");
        assert_eq!(
            root.session_sidebar_layout(79),
            (Some(PanelPlacement::FloatRight), 24, 79, 9)
        );
        assert_eq!(
            root.session_sidebar_layout(80),
            (Some(PanelPlacement::DockRight), 24, 56, 9)
        );
        assert_eq!(root.session_sidebar_reserved_cols(79), 0);
        assert_eq!(root.session_sidebar_reserved_cols(80), 24);
        root.close_session_sidebar();
        assert_eq!(root.session_sidebar_layout(120), (None, 0, 120, 0));
        assert_eq!(root.session_sidebar_reserved_cols(120), 0);
    }

    #[test]
    fn sidebar_width_cycles_and_keeps_min_content() {
        let mut root = UiRoot::new();
        root.set_term_cols(80);
        root.show_session_sidebar("s1");
        assert_eq!(
            root.session_sidebar_layout(80),
            (Some(PanelPlacement::DockRight), 24, 56, 9)
        );
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(
            root.session_sidebar_layout(80),
            (Some(PanelPlacement::DockRight), 32, 48, 9)
        );
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(
            root.session_sidebar_layout(80),
            (Some(PanelPlacement::DockRight), 40, 40, 9)
        );
        assert!(root.session_sidebar_width_cycle());
        assert_eq!(
            root.session_sidebar_layout(80),
            (Some(PanelPlacement::DockRight), 24, 56, 9)
        );
        assert_eq!(
            root.session_sidebar_layout(79),
            (Some(PanelPlacement::FloatRight), 24, 79, 9)
        );
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
    fn sidebar_dock_keeps_dock_rows_full_width() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.show_session_sidebar("s1");
        let lines = Component::render(&mut root, 96);
        let dock_rows = root.last_dock_rows();
        let transcript_rows = lines.len().saturating_sub(dock_rows);
        let mut viewport = lines;
        Component::overlay_viewport(&root, &mut viewport, 120, transcript_rows);
        assert!(
            viewport
                .iter()
                .take(13)
                .any(|line| line.contains("会话侧栏")),
            "dock sidebar must paint in transcript rows"
        );
        let dock = &viewport[viewport.len().saturating_sub(dock_rows)..];
        assert!(
            dock.iter().all(|line| !line.contains("会话侧栏")),
            "dock rows must not contain the sidebar"
        );
        assert!(
            dock.iter()
                .any(|line| xylitol_tui::visible_width(line) > 96),
            "dock rows must stay full width"
        );
    }

    #[test]
    fn sidebar_float_draws_in_transcript_only() {
        let mut root = UiRoot::new();
        root.set_term_cols(72);
        root.set_term_rows(30);
        root.show_session_sidebar("s1");
        assert_eq!(
            root.session_sidebar_layout(72).0,
            Some(PanelPlacement::FloatRight)
        );
        let lines = Component::render(&mut root, 72);
        let dock_rows = root.last_dock_rows();
        let mut viewport = lines.clone();
        Component::overlay_viewport(
            &root,
            &mut viewport,
            72,
            lines.len().saturating_sub(dock_rows),
        );
        assert!(
            viewport
                .iter()
                .take(13)
                .any(|line| line.contains("会话侧栏")),
            "float sidebar must paint in transcript rows"
        );
        let dock = &viewport[viewport.len().saturating_sub(dock_rows)..];
        assert!(
            dock.iter().all(|line| !line.contains("会话侧栏")),
            "dock rows must not contain the float sidebar"
        );
    }

    #[test]
    fn sidebar_hidden_on_too_narrow_and_returns() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.show_session_sidebar("s1");
        assert_eq!(
            root.session_sidebar_layout(120).0,
            Some(PanelPlacement::DockRight)
        );
        root.set_term_cols(72);
        assert_eq!(
            root.session_sidebar_layout(72).0,
            Some(PanelPlacement::FloatRight)
        );
        root.set_term_cols(20);
        assert_eq!(root.session_sidebar_layout(20).0, None);
        assert!(!root.session_sidebar_available());
        root.set_term_cols(120);
        assert_eq!(
            root.session_sidebar_layout(120).0,
            Some(PanelPlacement::DockRight)
        );
        assert!(root.session_sidebar_on(), "open intent must survive");
    }

    #[test]
    fn sidebar_height_cap_uses_percent_and_absolute() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.show_session_sidebar("s1");
        assert_eq!(root.session_sidebar_layout(120).3, 13);
        root.set_session_sidebar_layout(24, 3);
        assert_eq!(root.session_sidebar_layout(120).3, 3);
    }

    fn root_with_dock(queue_rows: usize) -> UiRoot {
        use xylitol_plugin_sdk::session::{TodoItem, TodoList, TodoStatus};

        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.set_term_rows(30);
        root.ui_model.todo = TodoList::new(vec![TodoItem {
            id: "t1".into(),
            content: "doing thing".into(),
            status: TodoStatus::InProgress,
        }]);
        let todo = root.render_todo_bar_slot(120);
        root.last_todo_rows = todo.len();
        root.last_queue_rows = queue_rows;
        root.last_dock_rows = queue_rows + todo.len() + 5;
        root.queue_strip_row = 20;
        root
    }

    #[test]
    fn dock_registers_hover_hits_for_queue_and_todo_summary() {
        let root = root_with_dock(3);
        let hovers = root.overlay_hover_hits(120);
        let ids: Vec<_> = hovers.iter().map(|h| (*h).into_region().id).collect();
        assert!(
            ids.contains(&UiRoot::QUEUE_STRIP_HIT_ID),
            "queue hover: {ids:?}"
        );
        assert!(
            ids.contains(&UiRoot::TODO_SUMMARY_HOVER_HIT_ID),
            "todo summary hover: {ids:?}"
        );
        for hit in &hovers {
            let region = (*hit).into_region();
            assert_eq!(
                region.space,
                HitSpace::Dock,
                "dock hover hits must map through the dock row space"
            );
            assert!(!region.capturing, "hover hits must not capture the pointer");
        }
    }

    #[test]
    fn todo_summary_hover_sits_below_the_queue_strip() {
        let root = root_with_dock(3);
        let todo = root
            .overlay_hover_hits(120)
            .into_iter()
            .map(|hit| hit.into_region())
            .find(|region| region.id == UiRoot::TODO_SUMMARY_HOVER_HIT_ID)
            .expect("todo summary hover hit");
        let queue = root
            .overlay_hover_hits(120)
            .into_iter()
            .map(|hit| hit.into_region())
            .find(|region| region.id == UiRoot::QUEUE_STRIP_HIT_ID)
            .expect("queue hover hit");
        assert_eq!(todo.col_start, 0);
        assert!(todo.col_end > 0);
        assert_eq!(
            todo.content_row,
            queue.content_row + 3 + crate::widgets::TODO_BAR_SPACER_ROWS,
            "todo summary sits under the queue strip and its blank spacer row"
        );
    }

    #[test]
    fn empty_dock_registers_no_hover_hits() {
        let root = UiRoot::new();
        assert!(
            root.overlay_hover_hits(120).is_empty(),
            "no queue and no todo means no dock hover targets"
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
    fn overlay_hits_cover_sidebar_and_separator() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.show_session_sidebar("s1");
        let hits = root.overlay_hits(120, 40);
        let ids: Vec<_> = hits.iter().map(|hit| (*hit).into_region().id).collect();
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
    fn spinner_pulses_color_without_changing_glyph() {
        use xylitol_tui::utils::strip_ansi_codes;
        let mut root = UiRoot::new();
        let mut model = UiModel::new();
        model.status = Some("working".into());
        root.apply_ui_model(&model);
        let first = root
            .render(80)
            .into_iter()
            .find(|line| strip_ansi_codes(line).contains('⬢'))
            .expect("spinner row");
        std::thread::sleep(std::time::Duration::from_millis(150));
        root.status_loader.set_message("working".into());
        let second = root
            .render(80)
            .into_iter()
            .find(|line| strip_ansi_codes(line).contains('⬢'))
            .expect("spinner row");
        assert_ne!(first, second, "spinner color must pulse between frames");
        assert_eq!(
            strip_ansi_codes(&first),
            strip_ansi_codes(&second),
            "spinner glyph must not change"
        );
    }

    #[test]
    fn multiline_input_keeps_more_cue_and_chrome() {
        let mut root = UiRoot::new();
        root.set_term_cols(80);
        root.set_term_rows(16);
        root.set_editor_text("a\na\na\na\na\na");
        let lines = root.render(80);
        let joined: String = lines
            .iter()
            .map(|l| plain(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("↑ 1 more") || joined.contains("↓ 1 more"),
            "multi-line input must keep the overflow cue:\n{joined}"
        );
        assert!(joined.contains("[步骤显示级别: 摘要]"), "{joined}");
        assert!(
            joined.contains('❯'),
            "input prompt must be painted:\n{joined}"
        );
        assert!(
            joined.contains('│'),
            "content rows must have vertical sides:\n{joined}"
        );
        let marker_line = lines
            .iter()
            .find(|line| plain(line).contains('◨'))
            .expect("marker line");
        assert!(
            !plain(marker_line).starts_with('╰'),
            "marker must sit above the bottom border:\n{}",
            plain(marker_line)
        );
        let bottom_line = lines
            .iter()
            .find(|line| plain(line).contains('╰') && plain(line).contains("[步骤显示级别: 摘要]"))
            .expect("bottom border");
        assert!(
            plain(bottom_line).ends_with('╯')
                && plain(bottom_line).contains("[步骤显示级别: 摘要]"),
            "bottom border must keep ╯ below the marker:\n{}",
            plain(bottom_line)
        );
        assert!(!joined.contains("[侧栏]"), "{joined}");
        assert!(!joined.contains("[重置折叠]"), "{joined}");
        assert!(
            !joined.contains('…'),
            "border must not truncate with ellipsis:\n{joined}"
        );
    }

    #[test]
    fn editor_content_keeps_cursor_ansi() {
        let mut root = UiRoot::new();
        root.set_term_cols(80);
        root.set_editor_text("abc");
        let lines = root.render(80);
        let joined: String = lines
            .iter()
            .map(|l| plain(l))
            .collect::<Vec<_>>()
            .join("\n");
        let raw: String = lines.join("\n");
        assert!(
            raw.contains("\x1b[7m"),
            "cursor styling must survive:\n{joined}"
        );
    }

    #[test]
    fn prompt_and_marker_use_button_colours() {
        let mut root = UiRoot::new();
        root.set_term_cols(80);
        root.set_thinking_level_ui("high".into());
        let lines = root.render(80);
        let raw = lines.join("\n");
        let palette = crate::layout::LayoutTheme::product_dark().palette();
        assert!(
            raw.contains(&xylitol_tui::fg_rgb(palette.user, "❯")),
            "prompt must use the user colour"
        );
        assert!(
            raw.contains(&xylitol_tui::fg_rgb(palette.muted, "◨")),
            "inactive marker must use the muted default"
        );
    }

    #[test]
    fn summary_click_with_body_updates_chip_in_place() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let mut model = UiModel::new();
        model
            .entries
            .push(crate::bridge::UiEntry::User { text: "hi".into() });
        model.entries.push(crate::bridge::UiEntry::Assistant {
            text: "hello".into(),
        });
        root.apply_ui_model(&model);
        let lines = root.render(120);
        let line = lines
            .iter()
            .find(|line| plain(line).contains("[步骤显示级别: 摘要]"))
            .expect("summary row");
        let plain_line = plain(line);
        let col = xylitol_tui::visible_width(
            &plain_line[..plain_line.find("[步骤显示级别: 摘要]").unwrap()],
        );
        assert!(root.try_click_painted_chip(col, line));
        let after = plain(&root.render(120).join("\n"));
        assert!(
            after.contains("[步骤显示级别: 最近]"),
            "click must update chip in place: {after}"
        );
        assert!(
            !root
                .client_notices
                .iter()
                .any(|notice| notice.body.contains("显示等级")),
            "in-place chip must not push a notice"
        );
    }

    #[test]
    fn marker_click_closes_open_sidebar() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.show_session_sidebar("s1");
        let lines = root.render(120);
        let line = lines
            .iter()
            .find(|line| plain(line).contains('◨'))
            .expect("marker line");
        let plain_line = plain(line);
        let col = xylitol_tui::visible_width(&plain_line[..plain_line.find('◨').expect("marker")]);
        assert!(
            root.try_click_painted_chip(col, line),
            "marker must consume"
        );
        assert!(
            !root.session_sidebar_on(),
            "second click must close sidebar"
        );
    }

    #[test]
    fn summary_click_without_body_updates_chip_in_place() {
        let mut root = UiRoot::new();
        root.set_term_cols(120);
        let lines = root.render(120);
        let line = lines
            .iter()
            .find(|line| plain(line).contains("[步骤显示级别: 摘要]"))
            .expect("summary row");
        let plain_line = plain(line);
        let col = xylitol_tui::visible_width(
            &plain_line[..plain_line.find("[步骤显示级别: 摘要]").unwrap()],
        );
        assert!(root.try_click_painted_chip(col, line));
        let after = plain(&root.render(120).join("\n"));
        assert!(
            after.contains("[步骤显示级别: 最近]"),
            "empty transcript click must update chip in place: {after}"
        );
        assert!(
            !root
                .client_notices
                .iter()
                .any(|notice| notice.body.contains("显示等级")),
            "in-place chip must not push a notice"
        );
    }

    #[test]
    fn transient_empty_snapshot_keeps_mcp_welcome_row() {
        let mut root = UiRoot::new();
        root.set_term_cols(100);
        root.set_loaded_resources(LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_connecting_label: Some("connecting 0/2".into()),
            ..LoadedResourcesSnapshot::default()
        });
        let with_mcp = plain(&root.render(100).join("\n"));
        assert!(with_mcp.contains("mcp"), "{with_mcp}");
        assert_eq!(
            root.status_next_turn_cue_for_test().as_deref(),
            Some(crate::driver::MCP_PENDING_CUE)
        );

        root.set_loaded_resources(LoadedResourcesSnapshot::default());
        let after_empty = plain(&root.render(100).join("\n"));
        assert!(
            after_empty.contains("mcp"),
            "unfinished empty snapshot must not hide the welcome row: {after_empty}"
        );
        assert_eq!(
            root.status_next_turn_cue_for_test().as_deref(),
            Some(crate::driver::MCP_PENDING_CUE)
        );

        root.set_loaded_resources(LoadedResourcesSnapshot {
            mcp_bootstrap_complete: true,
            ..LoadedResourcesSnapshot::default()
        });
        let settled = plain(&root.render(100).join("\n"));
        assert!(
            !settled.contains("mcp"),
            "settled no-MCP snapshot must hide the row: {settled}"
        );
        assert_eq!(root.status_next_turn_cue_for_test(), None);
    }

    #[test]
    fn status_slot_keeps_row_height_across_busy_idle() {
        let mut root = UiRoot::new();
        root.set_term_rows(30);
        root.set_term_cols(80);
        let mut model = UiModel::new();
        model.begin_run("long paragraph");
        model.entries.push(crate::bridge::UiEntry::User {
            text: "long paragraph ".repeat(20),
        });
        model.entries.push(crate::bridge::UiEntry::Assistant {
            text: "streaming output ".repeat(40),
        });
        root.apply_ui_model(&model);
        let _ = root.render(80);
        let busy_status_rows = root.last_status_rows;
        let busy_top = root.last_slot_top_row;

        let mut idle = UiModel::new();
        idle.entries = model.entries.clone();
        root.apply_ui_model(&idle);
        let _ = root.render(80);
        assert_eq!(
            root.last_status_rows, busy_status_rows,
            "status row height must not change when spinner disappears"
        );
        assert_eq!(
            root.last_slot_top_row, busy_top,
            "editor top must not move when spinner disappears"
        );
    }

    #[test]
    fn input_marker_hit_covers_two_columns() {
        use xylitol_tui::utils::strip_ansi_codes;
        let mut root = UiRoot::new();
        let lines = root.render(80);
        let line = lines
            .iter()
            .find(|line| strip_ansi_codes(line).contains('◨'))
            .expect("input marker row");
        let width = xylitol_tui::visible_width(&strip_ansi_codes(line));
        assert!(
            root.try_click_painted_chip(width.saturating_sub(2), line),
            "marker reserves two columns"
        );
        assert!(
            !root.try_click_painted_chip(width / 2, line),
            "border dashes must not be a chip"
        );
    }

    #[test]
    fn display_level_controls_thinking_expansion() {
        let mut root = UiRoot::new();
        for (level, expected) in [
            (crate::activity_fold::DisplayLevel::Summary, false),
            (crate::activity_fold::DisplayLevel::Recent, true),
            (crate::activity_fold::DisplayLevel::Full, true),
            (crate::activity_fold::DisplayLevel::Quiet, false),
        ] {
            let mut settings = crate::activity_fold::ActivityFoldSettings::default();
            settings.apply_display_level(level);
            root.set_activity_settings(settings);
            assert_eq!(
                root.fold_view().thinking_expanded,
                expected,
                "level {level:?} thinking expansion"
            );
        }
    }

    #[test]
    fn tray_chip_pointer_click_removes_attachment() {
        use crossterm::event::KeyModifiers;
        use xylitol_tui::InputEvent;

        let mut root = UiRoot::new();
        root.set_term_cols(120);
        root.mcp_tray_add("fs", "file:///notes");
        let _ = root.render(120);
        let row = u16::try_from(root.tray_strip_row).expect("tray row");
        let hit = root.last_tray_hits[0];
        let down = |col: u16| {
            InputEvent::Mouse(crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        root.handle_slot_input(down(1));
        assert_eq!(root.mcp_tray_len(), 1, "lead label must not remove");
        root.handle_slot_input(down(u16::try_from(hit.col_start).unwrap() + 1));
        assert_eq!(root.mcp_tray_len(), 0, "× hit region must remove the chip");
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
