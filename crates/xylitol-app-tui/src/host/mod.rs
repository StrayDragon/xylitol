mod editor_history;
mod input_policy;
mod link_grace;
mod pending;
mod session_ops;

pub use link_grace::{
    LINK_DOWN_NOTICE, LINK_GRACE_INITIAL, LINK_GRACE_RECONNECT, LINK_RECOVERED_NOTICE, LinkGrace,
    LinkNotice, observe_notice,
};
pub use pending::PendingOps;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::{InputEvent, InputReaction, PointerClickPhase, RenderError, TUI, Terminal};

use crate::driver::{XyDriver, XyDriverError, XyEvent};
use xylitol_plugin_sdk::XyBashResult;

use super::bridge::{UiEntry, UiModel, UiPhase, apply_xy_event};
use super::layout::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};

pub use super::commands::{PendingBash, PendingSlash, bash_block_status, bash_output_body};

pub const MIN_COLS: u16 = 40;
pub const MIN_ROWS: u16 = 6;

pub fn display_path(path: impl AsRef<std::path::Path>) -> String {
    let abs = path.as_ref().display().to_string();
    strip_home(&abs).unwrap_or(abs)
}

fn strip_home(abs: &str) -> Option<String> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let home = home.to_string_lossy();
    abs.strip_prefix(home.as_ref())
        .map(|rest| format!("~{rest}"))
}

pub fn display_cwd() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    display_path(cwd)
}

pub const TOO_SMALL_HINT: &str = "请放大终端";

pub fn is_too_small(cols: u16, rows: u16) -> bool {
    cols < MIN_COLS || rows < MIN_ROWS
}

#[derive(Debug, Clone)]
pub enum HostEvent {
    Input(InputEvent),
    Resize {
        cols: u16,
        rows: u16,
    },
    Tick,
    Quit,
    Xy(Box<XyEvent>),
    FooterTokens {
        job_id: u64,
        label: Option<String>,
        heuristic: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Ready,
    TooSmall,
}

pub struct HostSession<T: Terminal> {
    pub tui: TUI<T>,
    mode: LayoutMode,
    quit: bool,
    quit_flag: Arc<AtomicBool>,
    ui_root: Option<Rc<RefCell<UiRoot>>>,
    rebuild: Box<dyn FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>>>,
    ui_model: UiModel,
    pending: PendingOps,
    run_active: bool,
    bash_active: bool,
    reload_active: bool,
    reload_cancel: Option<tokio_util::sync::CancellationToken>,
    suppress_idle_esc: bool,
    suppress_xy_until_stream_end: bool,
    paint_dirty: bool,
    layout_cwd: String,
    theme_preference: Option<String>,
    #[cfg(test)]
    force_real_external_editor: bool,
    #[cfg(test)]
    external_editor_cmd_override: Option<Result<String, super::error::TuiSurfaceError>>,
    footer_token_gen: u64,
    footer_token_tx: tokio::sync::mpsc::UnboundedSender<(u64, Option<String>, bool)>,
    footer_token_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, Option<String>, bool)>,
    last_mid_turn_footer_refresh: Option<std::time::Instant>,
    run_applied_token_settlement: bool,
    pending_settlement_estimate: Option<xylitol_plugin_sdk::model::ContextTokenEstimate>,
    pub(crate) client_storage: Option<std::sync::Arc<crate::storage::ClientStorage>>,
    editor_history_seed_sessions: u32,
    editor_history_seed_job: Option<(std::time::Instant, tokio::task::JoinHandle<Vec<String>>)>,
    mcp_blocks_agent: bool,
    ask_gateway: Option<Arc<crate::ask_host::AskHostGateway>>,
    debug_live_ask_close: bool,
    designing: bool,
    keybindings: Option<Rc<RefCell<xylitol_tui::KeybindingsManager>>>,
    _keybindings_scope: Option<xylitol_tui::KeybindingsScope>,
    pointer_down_had_selection: bool,
    loaded_history: Option<crate::history_window::LoadedHistory>,
}

impl<T: Terminal> HostSession<T> {
    pub fn new(
        terminal: T,
        rebuild: impl FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>> + 'static,
    ) -> Self {
        Self::new_with_interaction_mode(terminal, xylitol_tui::InteractionMode::Inline, rebuild)
    }

    pub fn new_with_interaction_mode(
        terminal: T,
        interaction_mode: xylitol_tui::InteractionMode,
        mut rebuild: impl FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>> + 'static,
    ) -> Self {
        let cols = terminal.columns();
        let rows = terminal.rows();
        let mode = if is_too_small(cols, rows) {
            LayoutMode::TooSmall
        } else {
            LayoutMode::Ready
        };
        let mut tui = xylitol_tui::TUI::with_interaction_mode(terminal, interaction_mode);
        for child in rebuild(mode) {
            tui.add_child(child);
        }
        tui.set_focus(Some(0));
        tui.request_render(false);
        let (footer_token_tx, footer_token_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            tui,
            mode,
            quit: false,
            quit_flag: Arc::new(AtomicBool::new(false)),
            ui_root: None,
            rebuild: Box::new(rebuild),
            ui_model: UiModel::new(),
            pending: PendingOps::default(),
            run_active: false,
            bash_active: false,
            reload_active: false,
            reload_cancel: None,
            suppress_idle_esc: false,
            suppress_xy_until_stream_end: false,
            paint_dirty: false,
            layout_cwd: display_cwd(),
            theme_preference: None,
            #[cfg(test)]
            force_real_external_editor: false,
            #[cfg(test)]
            external_editor_cmd_override: None,
            footer_token_gen: 0,
            footer_token_tx,
            footer_token_rx,
            last_mid_turn_footer_refresh: None,
            run_applied_token_settlement: false,
            pending_settlement_estimate: None,
            client_storage: None,
            editor_history_seed_sessions: 1,
            editor_history_seed_job: None,
            mcp_blocks_agent: false,
            ask_gateway: None,
            debug_live_ask_close: false,
            designing: false,
            keybindings: None,
            _keybindings_scope: None,
            pointer_down_had_selection: false,
            loaded_history: None,
        }
    }

    pub fn sync_dock_rows(&mut self) {
        let rows = if let Some(root) = self.ui_root.as_ref() {
            let r = root.borrow();
            let measured = r.last_dock_rows();
            if measured > 1 {
                measured
            } else {
                r.estimate_dock_rows()
            }
        } else {
            8
        };
        self.tui.set_dock_rows(rows.max(4));
    }

    pub fn set_mcp_blocks_agent(&mut self, blocks: bool) {
        self.mcp_blocks_agent = blocks;
    }

    pub fn mcp_blocks_agent(&self) -> bool {
        self.mcp_blocks_agent
    }

    pub fn set_ask_gateway(&mut self, gateway: Arc<crate::ask_host::AskHostGateway>) {
        self.ask_gateway = Some(gateway);
    }

    pub fn set_designing_mode(&mut self, on: bool) {
        self.designing = on;
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().set_designing(on);
        }
    }

    pub fn designing_mode(&self) -> bool {
        self.designing
    }

    pub fn open_designing_scene_slot(&mut self) {
        if !self.designing_mode() {
            return;
        }
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().mount_scenes_picker();
        self.sync_ui_root_from_model();
    }

    pub fn close_scenes_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_slot();
            self.sync_ui_root_from_model();
        }
    }

    pub fn take_pending_scene_select(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_scene_select()
    }

    pub async fn inject_designing_scene(&mut self, driver: &mut dyn XyDriver, scene: &str) {
        if !self.designing_mode() {
            return;
        }
        crate::effects::designing::inject_scene(self, driver, scene.to_string()).await;
    }

    pub fn new_product_ui(terminal: T) -> Self {
        Self::new_product_ui_with_meta(terminal, display_cwd(), crate::UNSET_MODEL_DISPLAY.into())
    }

    pub fn new_product_ui_with_meta(terminal: T, cwd: String, model: String) -> Self {
        Self::new_product_ui_with_meta_mode(
            terminal,
            cwd,
            model,
            xylitol_tui::InteractionMode::ApplicationOwned,
        )
    }

    pub fn new_product_ui_with_meta_mode(
        terminal: T,
        cwd: String,
        model: String,
        interaction_mode: xylitol_tui::InteractionMode,
    ) -> Self {
        let manager = {
            #[cfg(test)]
            {
                super::keybindings::build_product_keybindings(xylitol_tui::KeybindingsConfig::new())
            }
            #[cfg(not(test))]
            {
                let agent_dir = super::keybindings::default_agent_dir();
                super::keybindings::load_product_keybindings(&agent_dir).0
            }
        };
        let keybindings = Rc::new(RefCell::new(manager));
        let keybindings_scope = xylitol_tui::KeybindingsScope::enter(keybindings.clone());

        let ui_root = Rc::new(RefCell::new(UiRoot::new()));
        ui_root.borrow_mut().set_layout_meta(cwd.clone(), model);
        ui_root.borrow_mut().set_term_rows(terminal.rows() as usize);
        ui_root
            .borrow_mut()
            .set_term_cols(terminal.columns() as usize);
        let quit_flag = Arc::new(AtomicBool::new(false));
        let mut session = Self::new_with_interaction_mode(
            terminal,
            interaction_mode,
            shared_ui_root_rebuild(ui_root.clone()),
        );
        session.layout_cwd = cwd;
        session.ui_root = Some(ui_root.clone());
        session.quit_flag = quit_flag.clone();
        session.keybindings = Some(keybindings);
        session._keybindings_scope = Some(keybindings_scope);
        install_ui_root_key_listeners(&ui_root, &quit_flag, &mut session.tui);
        if interaction_mode.is_application_owned() {
            session.sync_dock_rows();
            session.tui.begin_application_owned_session();
            session
                .tui
                .set_pointer_click_phase(PointerClickPhase::UpIfEmptySelection);
            session.tui.arm_pointer_probe(true);
            session
                .tui
                .set_scroll_gutter(Some(xylitol_tui::ScrollGutter));
            session.sync_dock_rows();
            session.sync_fold_hit_viewport();
            session.tui.request_render(true);
            session.paint_dirty = true;
        }
        session
    }

    pub(crate) fn sync_fold_hit_viewport(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let scroll_top = self.tui.application_owned_scroll_top();
        let term_rows = self.tui.terminal.rows() as usize;
        let dock = self.tui.dock_rows();
        let transcript_rows = term_rows.saturating_sub(dock) as u16;
        root.borrow_mut()
            .sync_fold_hit_viewport(scroll_top, transcript_rows);
        self.sync_pointer_hits();
    }

    fn sync_pointer_hits(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let cols = self.tui.terminal.columns() as usize;
        let term_rows = self.tui.terminal.rows() as usize;
        let dock = self.tui.dock_rows();
        let transcript_rows = term_rows.saturating_sub(dock) as u16;
        let root = root.borrow();
        let mut clicks = root.fold_hits().pointer_click_hits();
        let hovers = root.fold_hits().pointer_hover_hits();
        clicks.extend(root.overlay_hits(cols, transcript_rows as usize));
        drop(root);
        clicks.extend(self.tui.scroll_gutter_hit_set());
        self.tui.replace_hits(clicks);
        self.tui.replace_hover_hits(hovers);
    }

    pub fn mode(&self) -> LayoutMode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.quit || self.quit_flag.load(Ordering::SeqCst)
    }

    pub fn engine_stopped(&self) -> bool {
        self.tui.stop_requested()
    }

    pub fn engine_frame_count(&self) -> u64 {
        self.tui.frame_count()
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    pub fn ui_root(&self) -> Option<&Rc<RefCell<UiRoot>>> {
        self.ui_root.as_ref()
    }

    pub fn ui_model(&self) -> &UiModel {
        &self.ui_model
    }

    pub(crate) fn ui_model_mut(&mut self) -> &mut UiModel {
        &mut self.ui_model
    }

    pub fn is_busy(&self) -> bool {
        self.ui_model.phase == UiPhase::Busy || self.run_active || self.bash_active
    }

    pub fn reload_active(&self) -> bool {
        self.reload_active
    }

    pub fn wants_busy_tick(&self) -> bool {
        self.is_busy()
            || self.reload_active
            || (self.tui.is_render_requested()
                && self.tui.application_owned_wheel_render_deadline().is_none())
    }

    pub fn take_reload(&mut self) -> bool {
        self.pending.take_reload()
    }

    pub fn arm_reload(&mut self) {
        self.pending.reload = true;
    }

    pub fn begin_reload(&mut self) {
        let cancel = tokio_util::sync::CancellationToken::new();
        self.reload_cancel = Some(cancel);
        self.reload_active = true;
        self.ui_model.set_busy_status("Reloading");
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().set_suppress_status_right_cue(true);
        }
        self.sync_ui_root_from_model();
    }

    pub fn reload_cancel_token(&self) -> Option<tokio_util::sync::CancellationToken> {
        self.reload_cancel.clone()
    }

    pub fn request_reload_cancel(&mut self) {
        if let Some(t) = self.reload_cancel.as_ref() {
            t.cancel();
        }
    }

    pub fn end_reload(&mut self) {
        self.reload_active = false;
        self.reload_cancel = None;
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().set_suppress_status_right_cue(false);
        }
        if !self.run_active && !self.bash_active {
            self.ui_model.phase = UiPhase::Idle;
            self.ui_model.status = None;
        }
        self.sync_ui_root_from_model();
    }

    pub fn run_active(&self) -> bool {
        self.run_active
    }

    pub fn reload_keybindings(
        &self,
        agent_dir: &std::path::Path,
    ) -> super::keybindings::ReloadOutcome {
        if let Some(kb) = &self.keybindings {
            super::keybindings::reload_keybindings_into(&mut kb.borrow_mut(), agent_dir)
        } else {
            super::keybindings::reload_keybindings(agent_dir)
        }
    }

    pub fn reload_themes(&mut self, theme_name: &str) -> Result<(), XyDriverError> {
        let theme = super::themes::layout_theme_from_name(theme_name)?;
        if let Some(root) = &self.ui_root {
            root.borrow_mut().set_layout_theme(theme);
        }
        self.theme_preference = Some(theme_name.to_string());
        Ok(())
    }

    pub fn theme_preference(&self) -> Option<&str> {
        self.theme_preference.as_deref()
    }

    pub fn bash_active(&self) -> bool {
        self.bash_active
    }

    pub fn begin_bash_exec(&mut self, command: &str, exclude_from_context: bool) {
        self.pending.abort = false;
        self.suppress_idle_esc = false;
        self.bash_active = true;
        self.ui_model
            .begin_bash_block(command, exclude_from_context);
        self.sync_ui_root_from_model();
    }

    pub fn end_bash_exec(&mut self) {
        self.bash_active = false;
        self.pending.abort = false;
        if !self.run_active {
            self.ui_model.phase = UiPhase::Idle;
            self.ui_model.status = None;
        }
        self.sync_ui_root_from_model();
    }

    pub fn note_user_abort(&mut self) {
        self.ui_model.note_user_abort();
        self.bash_active = false;
        self.run_active = false;
        self.pending.abort = false;
        self.suppress_xy_until_stream_end = true;
        self.suppress_idle_esc = true;
        self.sync_ui_root_from_model();
    }

    pub fn note_bash_cancelled(&mut self) {
        self.ui_model.note_bash_cancelled();
        self.bash_active = false;
        self.pending.abort = false;
        self.pending.bash_cancelled = true;
        self.suppress_idle_esc = true;
        self.sync_ui_root_from_model();
    }

    pub fn take_submit(&mut self) -> Option<String> {
        self.pending.take_submit()
    }

    pub fn take_gated_submit(&mut self) -> Option<String> {
        self.pending.take_gated_submit()
    }

    pub fn set_gated_submit(&mut self, prompt: String) {
        self.pending.gated_submit = Some(prompt);
    }

    pub fn has_gated_submit(&self) -> bool {
        self.pending.peek_gated_submit()
    }

    pub fn end_gated_assemble_idle(&mut self) {
        if self.run_active || self.bash_active {
            return;
        }
        self.ui_model.phase = UiPhase::Idle;
        self.ui_model.status = None;
        self.sync_ui_root_from_model();
    }

    pub fn take_steer(&mut self) -> Option<String> {
        self.pending.take_steer()
    }

    pub fn take_follow_up(&mut self) -> Option<String> {
        self.pending.take_follow_up()
    }

    pub fn take_abort(&mut self) -> bool {
        self.pending.take_abort()
    }

    pub fn take_bash_cancelled(&mut self) -> bool {
        self.pending.take_bash_cancelled()
    }

    pub fn take_slash(&mut self) -> Option<PendingSlash> {
        self.pending.take_slash()
    }

    pub fn put_slash(&mut self, slash: PendingSlash) {
        self.pending.put_slash(slash);
    }

    pub fn take_bash(&mut self) -> Option<PendingBash> {
        self.pending.take_bash()
    }

    pub fn push_bash_result(&mut self, _command: &str, result: &XyBashResult) {
        self.ui_model
            .finish_bash_block(bash_block_status(result), bash_output_body(result));
        self.sync_ui_root_from_model();
    }

    pub fn append_bash_chunk(&mut self, chunk: &[u8]) {
        self.ui_model.append_bash_output(chunk);
        self.sync_ui_root_from_model();
        self.paint_dirty = true;
    }

    pub fn set_queue_badge(&mut self, steer_count: usize, follow_up_count: usize) {
        self.ui_model.sync_queue(steer_count, follow_up_count);
        self.sync_ui_root_from_model();
    }

    pub fn take_dequeue(&mut self) -> bool {
        self.pending.take_dequeue()
    }

    pub fn take_paste_image(&mut self) -> bool {
        self.pending.take_paste_image()
    }

    pub fn set_footer_model(&mut self, model: impl Into<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut()
            .set_layout_meta(self.layout_cwd.clone(), model);
        self.sync_ui_root_from_model();
    }

    pub fn apply_thinking_level_ui(&mut self, level: String) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_thinking_level_ui(level);
    }

    pub fn sync_fixed_zone(&mut self, driver: &dyn crate::driver::XyDriver) {
        let selected = driver.current_model();
        let selected_label = selected
            .as_ref()
            .map(|m| {
                if m.display_name.is_empty() {
                    m.id.clone()
                } else {
                    m.display_name.clone()
                }
            })
            .unwrap_or_else(|| crate::UNSET_MODEL_DISPLAY.into());
        let selected_thinking = driver.thinking_level();
        let selected_omit = selected.as_ref().is_none_or(|m| !m.thinking);

        let agent_run = driver.has_active_turn();
        let (footer_label, footer_thinking, omit) =
            if let Some((label, thinking, omit_thinking)) = driver.active_turn() {
                (label, thinking, omit_thinking)
            } else {
                (
                    selected_label.clone(),
                    selected_thinking.clone(),
                    selected_omit,
                )
            };

        let cue = if agent_run {
            if let Some((active_label, active_thinking, _)) = driver.active_turn() {
                crate::layout::status_next_turn_cue_text(
                    &active_label,
                    &active_thinking,
                    &selected_label,
                    &selected_thinking,
                )
            } else {
                None
            }
        } else {
            None
        };

        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.set_active_fixed_zone(footer_label, footer_thinking, omit);
            root.set_status_next_turn_cue(cue);
        }
        self.sync_ui_root_from_model();
    }

    pub fn set_footer_token_label(&mut self, label: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_footer_token_label(label);
    }

    pub fn request_footer_token_refresh(&mut self) {
        self.pending.footer_token_refresh = true;
    }

    pub fn request_footer_token_refresh_throttled(&mut self) {
        const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(750);
        let now = std::time::Instant::now();
        if self
            .last_mid_turn_footer_refresh
            .is_some_and(|t| now.duration_since(t) < MIN_INTERVAL)
        {
            return;
        }
        self.last_mid_turn_footer_refresh = Some(now);
        self.pending.footer_token_refresh = true;
    }

    pub fn take_pending_footer_token_refresh(&mut self) -> bool {
        self.pending.take_footer_token_refresh()
    }

    pub fn begin_footer_token_job(&mut self) -> u64 {
        self.footer_token_gen = self.footer_token_gen.wrapping_add(1);
        self.footer_token_gen
    }

    pub fn footer_token_tx(
        &self,
    ) -> tokio::sync::mpsc::UnboundedSender<(u64, Option<String>, bool)> {
        self.footer_token_tx.clone()
    }

    pub fn try_recv_footer_token(&mut self) -> Option<(u64, Option<String>, bool)> {
        self.footer_token_rx.try_recv().ok()
    }

    pub async fn recv_footer_token(&mut self) -> Option<(u64, Option<String>, bool)> {
        self.footer_token_rx.recv().await
    }

    pub fn footer_token_gen(&self) -> u64 {
        self.footer_token_gen
    }

    pub fn push_scroll_notice(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::ScrollNotice { text: text.into() });
        self.sync_ui_root_from_model();
    }

    pub fn push_toast_notice(&mut self, text: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().push_toast_notice(text);
            self.paint_dirty = true;
        }
    }

    pub fn push_toast_info_notice(&mut self, text: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().push_toast_info_notice(text);
            self.paint_dirty = true;
        }
    }

    pub fn arm_copy_last_msg_notice(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().arm_copy_last_msg_notice();
            self.paint_dirty = true;
        }
    }

    pub fn push_error_note(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::Error { text: text.into() });
        self.sync_ui_root_from_model();
    }

    #[cfg(test)]
    pub fn set_force_real_external_editor(&mut self, force: bool) {
        self.force_real_external_editor = force;
    }

    #[cfg(test)]
    pub(crate) fn set_external_editor_cmd_override(
        &mut self,
        cmd: Option<Result<String, super::error::TuiSurfaceError>>,
    ) {
        self.external_editor_cmd_override = cmd;
    }

    pub fn on_run_started(&mut self, prompt: &str) {
        self.suppress_idle_esc = false;
        self.suppress_xy_until_stream_end = false;
        self.run_active = true;
        self.run_applied_token_settlement = false;
        self.ui_model.begin_run(prompt);
        self.sync_ui_root_from_model();
    }

    pub fn on_run_stream_closed(&mut self) {
        self.run_active = false;
        self.suppress_xy_until_stream_end = false;
        self.ui_model.on_stream_closed_without_agent_end();
        if !self.run_applied_token_settlement {
            self.pending.footer_token_refresh = true;
        }
        self.sync_ui_root_from_model();
    }

    pub fn note_context_token_settlement(
        &mut self,
        estimate: xylitol_plugin_sdk::model::ContextTokenEstimate,
        reason: &str,
    ) {
        self.pending_settlement_estimate = Some(estimate);
        if matches!(reason, "turn_settled" | "after_compaction") {
            self.run_applied_token_settlement = true;
        }
    }

    pub fn take_pending_settlement_estimate(
        &mut self,
    ) -> Option<xylitol_plugin_sdk::model::ContextTokenEstimate> {
        self.pending_settlement_estimate.take()
    }

    pub(crate) fn sync_ui_root_from_model(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let rows = self.tui.terminal.rows() as usize;
        let mut root = root.borrow_mut();
        root.set_term_rows(rows);
        root.apply_ui_model(&self.ui_model);
        drop(root);
        self.tui.mark_ao_components_stale();
    }

    pub fn step(&mut self, event: HostEvent) -> Result<(), XyDriverError> {
        let defer_stream_paint = matches!(
            &event,
            HostEvent::Xy(xy)
                if matches!(
                    xy.as_ref(),
                    crate::driver::XyEvent::TextDelta(_)
                        | crate::driver::XyEvent::ThinkingDelta(_)
                )
        );
        match event {
            HostEvent::Quit => {
                self.quit = true;
            }
            HostEvent::Tick => {
                let anim_dirty = if self.mode == LayoutMode::Ready {
                    self.tui.idle_tick()
                } else {
                    false
                };
                if anim_dirty || self.paint_dirty {
                    self.tui.request_render(false);
                }
            }
            HostEvent::Resize { cols, rows } => {
                self.handle_resize(cols, rows);
            }
            HostEvent::Input(input) => {
                self.handle_input(input);
            }
            HostEvent::Xy(xy) => {
                self.handle_xy(xy);
            }
            HostEvent::FooterTokens { job_id, label, .. } => {
                if job_id == self.footer_token_gen {
                    self.set_footer_token_label(label);
                    self.tui.request_render(false);
                }
            }
        }

        if defer_stream_paint {
            return Ok(());
        }
        self.step_paint_only()
    }

    pub(crate) fn step_paint_only(&mut self) -> Result<(), XyDriverError> {
        let may_paint = self.tui.is_render_requested();
        let need_dock_sync =
            may_paint && self.tui.application_session_active() && self.tui.ao_components_stale();
        if need_dock_sync {
            self.sync_dock_rows();
        }
        let t_render = std::time::Instant::now();
        match self.tui.try_render() {
            Ok(painted) => {
                if painted {
                    crate::lag::note("host_step_paint", t_render);
                    self.paint_dirty = false;
                    if self.tui.application_session_active()
                        && !self.tui.last_render_perf().ao_reprojected
                    {
                        self.sync_dock_rows();
                    }
                    if self.tui.application_session_active() {
                        self.sync_fold_hit_viewport();
                    }
                }
                Ok(())
            }
            Err(RenderError { .. }) => self.recover_from_render_error(),
        }
    }

    pub fn sidebar_contains_column(&self, col: u16) -> bool {
        self.ui_root
            .as_ref()
            .is_some_and(|root| root.borrow().session_sidebar_contains(col as usize))
    }

    pub fn apply_sidebar_wheel_delta(&mut self, delta: isize) -> Result<(), XyDriverError> {
        if delta == 0 || !self.tui.application_session_active() {
            return Ok(());
        }
        let Some(root) = self.ui_root.as_ref() else {
            return Ok(());
        };
        let term_rows = self.tui.terminal.rows() as usize;
        let dock = self.tui.dock_rows();
        let transcript_rows = term_rows.saturating_sub(dock);
        let changed = root
            .borrow_mut()
            .scroll_session_sidebar(delta, transcript_rows);
        if changed {
            self.render_now()?;
            self.sync_fold_hit_viewport();
        }
        Ok(())
    }

    pub fn apply_ao_wheel_delta(&mut self, delta: isize) -> Result<(), XyDriverError> {
        if delta == 0 || !self.tui.application_session_active() {
            return Ok(());
        }
        if !self.tui.application_owned_scroll_by(delta) {
            return Ok(());
        }
        let t_wheel_render = std::time::Instant::now();
        match self.tui.try_render() {
            Ok(painted) => {
                if painted {
                    crate::lag::note("host_wheel_paint", t_wheel_render);
                    self.paint_dirty = false;
                    self.sync_fold_hit_viewport();
                }
                Ok(())
            }
            Err(RenderError { .. }) => self.recover_from_render_error(),
        }
    }

    fn handle_resize(&mut self, cols: u16, rows: u16) {
        let prev_cols = self.tui.terminal.columns();
        let prev_rows = self.tui.terminal.rows();
        self.tui.terminal.refresh_size();
        self.tui.terminal.set_size_hint(cols, rows);
        let size_changed =
            self.tui.terminal.columns() != prev_cols || self.tui.terminal.rows() != prev_rows;
        if size_changed {
            self.sync_layout_from_terminal();
            if self.tui.application_session_active() {
                self.sync_dock_rows();
            }
            self.tui.request_render(false);
        }
    }

    fn dispatch_pointer_click(&mut self, id: xylitol_tui::HitId) {
        match id {
            xylitol_tui::GUTTER_TOP_HIT_ID => {
                self.tui.application_owned_scroll_home();
            }
            xylitol_tui::GUTTER_BOTTOM_HIT_ID => {
                self.tui.application_owned_scroll_to_end();
            }
            crate::layout::UiRoot::SESSION_POPUP_HIT_ID => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut().close_session_popup();
                }
                self.tui.request_render(false);
            }
            crate::layout::UiRoot::SESSION_SIDEBAR_HIT_ID => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut().close_session_sidebar();
                }
                self.sync_session_sidebar_reserve();
                self.tui.request_render(false);
            }
            crate::layout::UiRoot::SESSION_SIDEBAR_SEPARATOR_HIT_ID => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut().session_sidebar_width_cycle();
                }
                self.sync_session_sidebar_reserve();
                self.tui.request_render(false);
            }
            _ => {
                if let Some(root) = self.ui_root.as_ref() {
                    let mut root = root.borrow_mut();
                    if let Some(target) = root.fold_hits().target_for_hit(id) {
                        root.toggle_fold_target(target);
                    }
                }
            }
        }
    }

    fn handle_input(&mut self, input: InputEvent) {
        if let InputEvent::Key(ref key) = input
            && key.code == crossterm::event::KeyCode::Up
            && key.modifiers.is_empty()
        {
            self.merge_client_history();
        }
        let left_down = matches!(
            input,
            InputEvent::Mouse(crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                ..
            })
        );
        let left_up = matches!(
            input,
            InputEvent::Mouse(crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
                ..
            })
        );
        if left_down {
            self.pointer_down_had_selection = self.tui.application_owned_has_selection();
        }
        let reaction = if self.mode == LayoutMode::Ready {
            self.follow_bottom_on_submit_key(&input);
            if self.try_suppress_stale_esc(&input)
                || self.try_paste_image(&input)
                || self.try_reload_input(&input)
                || self.try_busy_input(&input)
                || self.try_idle_enter_submit(&input)
                || self.try_ctrl_g(&input)
            {
                InputReaction::Rerender
            } else {
                self.tui.dispatch_event(input)
            }
        } else {
            InputReaction::rerender_if(!matches!(input, InputEvent::Mouse(_)))
        };
        if left_up {
            let clicked = self.tui.take_pointer_click();
            if !self.pointer_down_had_selection
                && let Some(id) = clicked
            {
                self.dispatch_pointer_click(id);
            }
        }
        if self.tui.application_session_active() {
            if let Some(root) = self.ui_root.as_ref() {
                let mut root = root.borrow_mut();
                root.sync_editor_screen_origin();
                if root.take_fold_dirty() {
                    drop(root);
                    self.tui.mark_ao_components_stale();
                }
            }
            if self.tui.take_copy_notice()
                && let Some(root) = self.ui_root.as_ref()
            {
                root.borrow_mut().arm_copy_notice();
                self.tui.request_render(false);
            }
        }
        if reaction == InputReaction::Rerender {
            self.tui.request_render(false);
        }
    }

    fn is_agent_tape_event(xy: &XyEvent) -> bool {
        matches!(
            xy,
            XyEvent::AgentStart { .. }
                | XyEvent::AgentEnd { .. }
                | XyEvent::TurnStart { .. }
                | XyEvent::TurnEnd { .. }
                | XyEvent::MessageStart { .. }
                | XyEvent::MessageUpdate { .. }
                | XyEvent::MessageEnd { .. }
                | XyEvent::TextDelta(_)
                | XyEvent::ThinkingDelta(_)
                | XyEvent::ToolExecutionStart { .. }
                | XyEvent::ToolExecutionUpdate { .. }
                | XyEvent::ToolExecutionEnd { .. }
                | XyEvent::AutoRetryStart { .. }
                | XyEvent::AutoRetryEnd { .. }
        )
    }

    pub(crate) fn handle_xy(&mut self, xy: Box<XyEvent>) {
        if self.suppress_xy_until_stream_end {
            self.tui.request_render(false);
            return;
        }
        if !self.run_active && Self::is_agent_tape_event(xy.as_ref()) {
            log::debug!(
                target: "xylitol::tui",
                "agent tape event dropped while idle"
            );
            return;
        }
        self.apply_xy_unfenced(xy);
    }

    pub(crate) fn apply_xy_unfenced(&mut self, xy: Box<XyEvent>) {
        apply_xy_event(&mut self.ui_model, &xy);
        match xy.as_ref() {
            XyEvent::AgentEnd { .. } => {
                self.run_active = false;
            }
            XyEvent::ContextTokenSettlement {
                estimate, reason, ..
            } => {
                self.note_context_token_settlement(estimate.clone(), reason);
            }
            XyEvent::CompactionEnd { .. } => {
                self.request_footer_token_refresh();
            }
            XyEvent::TurnEnd { .. } => {}
            XyEvent::MessageEnd { role, message } => {
                let has_usage = message.as_ref().is_some_and(|m| {
                    matches!(
                        m,
                        xylitol_plugin_sdk::message::AgentMessage::Llm(
                            xylitol_plugin_sdk::message::XyLlmMessage::AssistantMessage {
                                usage: Some(_),
                                ..
                            }
                        )
                    )
                });
                if role == "assistant" && has_usage {
                    self.request_footer_token_refresh_throttled();
                }
            }
            _ => {}
        }
        self.sync_ui_root_from_model();
        if matches!(xy.as_ref(), XyEvent::AgentEnd { .. })
            && let Some(root) = self.ui_root.as_ref()
        {
            root.borrow_mut().apply_activity_after_turn_end();
        }
        self.tui.request_render(false);
    }

    fn recover_from_render_error(&mut self) -> Result<(), XyDriverError> {
        let cols = self.tui.terminal.columns();
        let rows = self.tui.terminal.rows();
        if self.mode != LayoutMode::TooSmall {
            self.mode = LayoutMode::TooSmall;
            self.tui.clear_children();
            for child in (self.rebuild)(LayoutMode::TooSmall) {
                self.tui.add_child(child);
            }
            self.tui.set_focus(Some(0));
            self.tui.request_render(true);
            if self.tui.render_now().is_ok() {
                log::warn!(
                    target: "xylitol::tui",
                    "render overflow at {cols}x{rows}; degraded to TooSmall hint"
                );
                return Ok(());
            }
        }
        self.quit = true;
        let err = XyDriverError::io(format!(
            "render failed: terminal too extreme ({cols}x{rows}); restoring and exiting"
        ));
        err.log_failure("tui.render.too_extreme");
        Err(err)
    }

    pub fn set_task_progress(&mut self, active: bool) {
        self.tui.terminal.set_progress(active);
    }

    pub fn emit_clipboard_osc52(&mut self, sequence: &str) {
        self.tui.terminal.write(sequence);
        self.tui.terminal.flush();
    }

    pub fn render_now(&mut self) -> Result<(), XyDriverError> {
        if self.tui.application_session_active() {
            self.sync_dock_rows();
        }
        match self.tui.render_now() {
            Ok(_) => {
                self.paint_dirty = false;
                if self.tui.application_session_active() {
                    self.sync_dock_rows();
                }
                Ok(())
            }
            Err(e) => {
                let err = XyDriverError::io(e.to_string());
                err.log_failure("tui.render_now");
                Err(err)
            }
        }
    }

    pub fn sync_layout_from_terminal(&mut self) {
        let cols = self.tui.terminal.columns();
        let rows = self.tui.terminal.rows();
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().set_term_rows(rows as usize);
            root.borrow_mut().set_term_cols(cols as usize);
        }
        let next = if is_too_small(cols, rows) {
            LayoutMode::TooSmall
        } else {
            LayoutMode::Ready
        };
        if next != self.mode {
            self.mode = next;
            self.tui.clear_children();
            for child in (self.rebuild)(self.mode) {
                self.tui.add_child(child);
            }
            self.tui.set_focus(Some(0));
        }
    }
}
