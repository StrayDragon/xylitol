//! Product TUI host — event step machine (testable without a real TTY).

mod input_policy;
mod pending;
mod session_ops;

pub use pending::PendingOps;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::{InputEvent, RenderError, TUI, Terminal};

use crate::app::core::driver::{XyDriver, XyDriverError, XyEvent};
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::SessionEntry;

use super::bridge::{UiEntry, UiModel, UiPhase, apply_xy_event};
use super::layout::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};

#[allow(unused_imports)] // re-export for callers / harness
pub use super::commands::{BangParse, parse_bang_command};
pub use super::commands::{PendingBash, PendingSlash, bash_block_status, bash_output_body};

/// Minimum usable terminal size (ath4).
pub const MIN_COLS: u16 = 40;
pub const MIN_ROWS: u16 = 6;

/// Compact cwd for footer (`$HOME` → `~`).
pub fn display_cwd() -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".into());
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = home.to_string_lossy();
        if let Some(rest) = cwd.strip_prefix(home.as_ref()) {
            return format!("~{rest}");
        }
    }
    cwd
}

/// Friendly prompt when the terminal is too small.
pub const TOO_SMALL_HINT: &str = "请放大终端";

/// Whether the terminal is below the product minimum.
pub fn is_too_small(cols: u16, rows: u16) -> bool {
    cols < MIN_COLS || rows < MIN_ROWS
}

/// Events the host loop can process. Production maps crossterm → these;
/// harnesses inject them directly (ath5).
#[derive(Debug, Clone)]
pub enum HostEvent {
    Input(InputEvent),
    Resize {
        cols: u16,
        rows: u16,
    },
    Tick,
    Quit,
    /// Agent lifecycle event from `XyDriver::run` EventStream (c465).
    Xy(Box<XyEvent>),
    /// Background footer token estimate finished (must not block input/render).
    FooterTokens {
        job_id: u64,
        label: Option<String>,
    },
}

/// Layout mode after applying size policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Normal product UI root (transcript / editor / footer).
    Ready,
    TooSmall,
}

/// One host-driven TUI session. Does **not** call [`TUI::start`](xylitol_tui::TUI::start).
pub struct HostSession<T: Terminal> {
    pub tui: TUI<T>,
    mode: LayoutMode,
    quit: bool,
    /// Set by InputListener (Ctrl+C on empty editor).
    quit_flag: Arc<AtomicBool>,
    /// Shared UI root when constructed via [`Self::new_product_ui`].
    ui_root: Option<Rc<RefCell<UiRoot>>>,
    /// Rebuild root children when mode flips.
    rebuild: Box<dyn FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>>>,
    /// UI-only model; updated solely via [`apply_xy_event`] / submit.
    ui_model: UiModel,
    /// Side effects for [`crate::app::tui::effects::drain_pending`] (c730).
    pending: PendingOps,
    /// True while a `XyDriver::run` stream is open (blocks duplicate submit).
    run_active: bool,
    /// True while interactive `!`/`!!` bash is in flight (c665 Esc abort).
    bash_active: bool,
    /// Absorb leftover Esc / key-repeat after an abort while **idle** so the next
    /// bang is not cancelled at submit. Cleared on non-Esc input and when a new
    /// busy period starts (`begin_bash_exec` / `on_run_started`). MUST NOT gate
    /// Esc while busy — that made the second bang un-abortable (pi: no suppress
    /// on bash Esc; fresh cancel each run).
    suppress_idle_esc: bool,
    /// After Esc abort: drop agent `XyEvent`s until this run's EventStream ends (c670).
    suppress_xy_until_stream_end: bool,
    /// Bang chunk / deferred content: Tick must paint even if Loader idle (ath24 / ath7).
    paint_dirty: bool,
    layout_cwd: String,
    /// Active theme preference for `/reload` (c1120 / c1095). `None` → keep current default.
    theme_preference: Option<String>,
    /// Test-only: force real external-editor path (skip harness stub gate).
    #[cfg(test)]
    force_real_external_editor: bool,
    /// Test-only: override `$VISUAL`/`$EDITOR` resolve (avoids process-wide env races).
    #[cfg(test)]
    external_editor_cmd_override: Option<Result<String, String>>,
    /// Latest footer-token job generation (stale results discarded).
    footer_token_gen: u64,
    /// Background estimate results → host `select!` (production).
    footer_token_tx: tokio::sync::mpsc::UnboundedSender<(u64, Option<String>)>,
    footer_token_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, Option<String>)>,
    /// Throttle mid-turn Api usage footer refresh (c1730).
    last_mid_turn_footer_refresh: Option<std::time::Instant>,
    /// `tui.editor_history_seed_sessions` (c1560).
    editor_history_seed_sessions: u32,
    /// MCP bootstrap in flight — gate agent prompt / bang / some slash (c1200).
    mcp_blocks_agent: bool,
}

impl<T: Terminal> HostSession<T> {
    /// Create a session. `rebuild(mode)` supplies the root component tree.
    pub fn new(
        terminal: T,
        mut rebuild: impl FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>> + 'static,
    ) -> Self {
        let cols = terminal.columns();
        let rows = terminal.rows();
        let mode = if is_too_small(cols, rows) {
            LayoutMode::TooSmall
        } else {
            LayoutMode::Ready
        };
        let mut tui = TUI::new(terminal);
        for child in rebuild(mode) {
            tui.add_child(child);
        }
        tui.set_focus(Some(0));
        // Soft pending paint — pi `start()` also uses soft `requestRender()`.
        // force=true would set the clear sentinel and wipe the screen on mount.
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
            editor_history_seed_sessions: 1,
            mcp_blocks_agent: false,
        }
    }

    /// Sync MCP connecting gate from the driver (c1200).
    pub fn set_mcp_blocks_agent(&mut self, blocks: bool) {
        self.mcp_blocks_agent = blocks;
    }

    pub fn mcp_blocks_agent(&self) -> bool {
        self.mcp_blocks_agent
    }

    /// Product empty UI: shared `UiRoot` + Ctrl+C / Esc / idle-Enter listeners.
    pub fn new_product_ui(terminal: T) -> Self {
        Self::new_product_ui_with_meta(
            terminal,
            display_cwd(),
            crate::app::core::bootstrap::UNSET_MODEL_DISPLAY.into(),
        )
    }

    /// Product UI with footer identity (`cwd · model`).
    pub fn new_product_ui_with_meta(terminal: T, cwd: String, model: String) -> Self {
        // c1090: install tui.* + app.* before any input listeners run.
        // Path is local (no infra reach); disk load only outside tests.
        #[cfg(test)]
        {
            super::keybindings::install_product_keybindings_defaults_only();
        }
        #[cfg(not(test))]
        {
            let agent_dir = super::keybindings::default_agent_dir();
            let _ = super::keybindings::install_product_keybindings(&agent_dir);
        }

        let ui_root = Rc::new(RefCell::new(UiRoot::new()));
        ui_root.borrow_mut().set_layout_meta(cwd.clone(), model);
        let quit_flag = Arc::new(AtomicBool::new(false));
        let mut session = Self::new(terminal, shared_ui_root_rebuild(ui_root.clone()));
        session.layout_cwd = cwd;
        session.ui_root = Some(ui_root.clone());
        session.quit_flag = quit_flag.clone();
        install_ui_root_key_listeners(&ui_root, &quit_flag, &mut session.tui);
        session
    }

    /// Configure how many prior sessions seed ↑/↓ history on new session (c1560).
    pub fn set_editor_history_seed_sessions(&mut self, n: u32) {
        self.editor_history_seed_sessions = n;
    }

    /// Replace editor send history from already-loaded entries (resume/switch).
    pub fn seed_editor_history_from_entries(&mut self, entries: &[SessionEntry]) {
        let texts = super::editor_history_seed::user_prompt_texts_from_entries(entries);
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().replace_editor_send_history(texts);
        }
    }

    /// Seed ↑/↓ history from prior same-cwd sessions (pure new session).
    pub async fn seed_editor_history_for_new_session(&mut self, driver: &dyn XyDriver) {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| self.layout_cwd.clone());
        let current_id = driver.session_id();
        let texts = super::editor_history_seed::collect_new_session_seed(
            driver,
            &cwd,
            current_id.as_deref(),
            self.editor_history_seed_sessions,
        )
        .await
        .unwrap_or_default();
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().replace_editor_send_history(texts);
        }
    }

    pub fn mode(&self) -> LayoutMode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.quit || self.quit_flag.load(Ordering::SeqCst)
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    /// Shared UI root handle (product construction only).
    pub fn ui_root(&self) -> Option<&Rc<RefCell<UiRoot>>> {
        self.ui_root.as_ref()
    }

    /// UI-only model (harness / status).
    pub fn ui_model(&self) -> &UiModel {
        &self.ui_model
    }

    pub(crate) fn ui_model_mut(&mut self) -> &mut UiModel {
        &mut self.ui_model
    }

    pub fn is_busy(&self) -> bool {
        self.ui_model.phase == UiPhase::Busy || self.run_active || self.bash_active
    }

    pub fn run_active(&self) -> bool {
        self.run_active
    }

    /// Re-read `agent_dir/keybindings.json` into the global manager (c1090).
    /// Failure keeps the previous bindings.
    pub fn reload_keybindings(
        &self,
        agent_dir: &std::path::Path,
    ) -> super::keybindings::ReloadOutcome {
        super::keybindings::reload_keybindings(agent_dir)
    }

    /// Apply a built-in theme name to the product UI (c1095).
    ///
    /// Unknown names return `Err` and leave the current theme unchanged.
    /// Does not clear transcript.
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

    /// Mark bang bash started: uplink Bash pending block, busy status `Running`.
    pub fn begin_bash_exec(&mut self, command: &str, exclude_from_context: bool) {
        self.pending.abort = false;
        // New bang = fresh cancel window; do not inherit post-abort Esc suppress.
        self.suppress_idle_esc = false;
        self.bash_active = true;
        self.ui_model
            .begin_bash_block(command, exclude_from_context);
        self.sync_ui_root_from_model();
    }

    /// Clear bang busy after execute finishes (cancelled or not).
    pub fn end_bash_exec(&mut self) {
        self.bash_active = false;
        self.pending.abort = false;
        if !self.run_active {
            self.ui_model.phase = UiPhase::Idle;
            self.ui_model.status = None;
        }
        self.sync_ui_root_from_model();
    }

    /// Immediate Esc abort feedback: flush partial + System `Operation aborted`, idle (c1595).
    /// Drops further agent Xy events until the current EventStream ends (c670).
    pub fn note_user_abort(&mut self) {
        self.ui_model.note_user_abort();
        self.bash_active = false;
        self.run_active = false;
        self.pending.abort = false;
        self.suppress_xy_until_stream_end = true;
        // Crossterm may still deliver Esc Press/Repeat while idle after abort;
        // eat those only until the next non-Esc key or a new busy period.
        self.suppress_idle_esc = true;
        self.sync_ui_root_from_model();
    }

    /// Bang Esc abort: `(cancelled)` under `$ cmd` (pi); idle + Esc backlog suppress.
    pub fn note_bash_cancelled(&mut self) {
        self.ui_model.note_bash_cancelled();
        self.bash_active = false;
        self.pending.abort = false;
        self.suppress_idle_esc = true;
        self.sync_ui_root_from_model();
    }

    /// Take an idle-Enter submit request, if any.
    pub fn take_submit(&mut self) -> Option<String> {
        self.pending.take_submit()
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

    pub fn take_slash(&mut self) -> Option<PendingSlash> {
        self.pending.take_slash()
    }

    /// Take a pending idle bash (`!` / `!!`) request.
    pub fn take_bash(&mut self) -> Option<PendingBash> {
        self.pending.take_bash()
    }

    /// Append bash outcome into the pending Bash block (c668).
    pub fn push_bash_result(&mut self, _command: &str, result: &XyBashResult) {
        self.ui_model
            .finish_bash_block(bash_block_status(result), bash_output_body(result));
        self.sync_ui_root_from_model();
    }

    /// Append live bang output chunk (c669); mark paint_dirty — Tick merges render (ath7/ath24).
    pub fn append_bash_chunk(&mut self, chunk: &[u8]) {
        self.ui_model.append_bash_output(chunk);
        self.sync_ui_root_from_model();
        self.paint_dirty = true;
    }

    /// Refresh queue badge from driver stats (after local steer/follow-up/abort).
    pub fn set_queue_badge(&mut self, steer_count: usize, follow_up_count: usize) {
        self.ui_model.sync_queue(steer_count, follow_up_count);
        self.sync_ui_root_from_model();
    }

    /// Take a pending Alt+Up dequeue request (clear both driver queues).
    pub fn take_dequeue(&mut self) -> bool {
        self.pending.take_dequeue()
    }

    /// Take pending clipboard-image paste (c1155).
    pub fn take_paste_image(&mut self) -> bool {
        self.pending.take_paste_image()
    }

    /// Update footer model name after `/model`.
    pub fn set_footer_model(&mut self, model: impl Into<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut()
            .set_layout_meta(self.layout_cwd.clone(), model);
        self.sync_ui_root_from_model();
    }

    /// Silent UI sync for thinking level (border + footer only).
    pub fn apply_thinking_level_ui(&mut self, level: crate::protocol::types::ThinkingLevel) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_thinking_level_ui(level);
    }

    /// Sync footer active chrome + optional next-turn cue from driver (c1470).
    pub fn sync_runtime_chrome(&mut self, driver: &dyn crate::app::core::driver::XyDriver) {
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
            .unwrap_or_else(|| crate::app::core::bootstrap::UNSET_MODEL_DISPLAY.into());
        let selected_thinking = driver.thinking_level();
        let selected_omit = selected.as_ref().is_none_or(|m| !m.thinking);

        let agent_run = driver.has_active_turn();
        let (footer_label, footer_thinking, omit) =
            if let Some((label, thinking, omit_thinking)) = driver.active_turn() {
                (label, thinking, omit_thinking)
            } else {
                (selected_label.clone(), selected_thinking, selected_omit)
            };

        let cue = if agent_run {
            if let Some((active_label, active_thinking, _)) = driver.active_turn() {
                crate::app::tui::layout::status_next_turn_cue_text(
                    &active_label,
                    active_thinking,
                    &selected_label,
                    selected_thinking,
                )
            } else {
                None
            }
        } else {
            None
        };

        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.set_active_chrome(footer_label, footer_thinking, omit);
            root.set_status_next_turn_cue(cue);
        }
        self.sync_ui_root_from_model();
    }

    /// Set or clear footer token usage fragment (c1035).
    pub fn set_footer_token_label(&mut self, label: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_footer_token_label(label);
    }

    /// Request a XyDriver estimate refresh on the next `drain_pending` (c1035).
    pub fn request_footer_token_refresh(&mut self) {
        self.pending.footer_token_refresh = true;
    }

    /// Mid-turn Api usage refresh with cooldown (MUST NOT per-TextDelta encode).
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

    /// Allocate a generation id for a new background footer estimate job.
    pub fn begin_footer_token_job(&mut self) -> u64 {
        self.footer_token_gen = self.footer_token_gen.wrapping_add(1);
        self.footer_token_gen
    }

    pub fn footer_token_tx(&self) -> tokio::sync::mpsc::UnboundedSender<(u64, Option<String>)> {
        self.footer_token_tx.clone()
    }

    /// Poll one completed background estimate (non-blocking).
    pub fn try_recv_footer_token(&mut self) -> Option<(u64, Option<String>)> {
        self.footer_token_rx.try_recv().ok()
    }

    /// Await the next background footer estimate (production `select!`).
    pub async fn recv_footer_token(&mut self) -> Option<(u64, Option<String>)> {
        self.footer_token_rx.recv().await
    }

    pub fn footer_token_gen(&self) -> u64 {
        self.footer_token_gen
    }

    /// Push a scrollback notice (slash errors, nav/ops feedback).
    pub fn push_scroll_notice(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::ScrollNotice { text: text.into() });
        self.sync_ui_root_from_model();
    }

    /// Push chrome toast (shell notice above status; not `UiEntry`).
    pub fn push_chrome_toast(&mut self, text: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().push_chrome_toast(text);
            self.paint_dirty = true;
        }
    }

    /// Push a completed compaction transcript block (c1730 slash path).
    pub fn push_compaction_complete(&mut self, summary: String, tokens_before: u64) {
        self.ui_model.entries.push(UiEntry::Compaction {
            status: crate::app::tui::bridge::CompactionBlockStatus::Complete,
            summary,
            tokens_before,
            detail: None,
        });
        self.sync_ui_root_from_model();
    }

    /// Push an error line (`UiEntry::Error` / `errors.md` one-liner).
    pub fn push_error_note(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::Error { text: text.into() });
        self.sync_ui_root_from_model();
    }

    /// Test harness: force Ctrl+G down the real-editor path (no stub).
    #[cfg(test)]
    pub fn set_force_real_external_editor(&mut self, force: bool) {
        self.force_real_external_editor = force;
    }

    /// Test harness: override editor command resolve (`Err` = missing config).
    #[cfg(test)]
    pub fn set_external_editor_cmd_override(&mut self, cmd: Option<Result<String, String>>) {
        self.external_editor_cmd_override = cmd;
    }

    pub fn request_submit(&mut self, prompt: impl Into<String>) {
        if self.run_active || self.ui_model.phase == UiPhase::Busy {
            return;
        }
        let prompt = prompt.into();
        if prompt.trim().is_empty() {
            return;
        }
        self.pending.submit = Some(prompt);
    }

    /// Mark that `XyDriver::run` has started; seeds the user entry + busy phase.
    pub fn on_run_started(&mut self, prompt: &str) {
        self.suppress_idle_esc = false;
        self.suppress_xy_until_stream_end = false;
        self.run_active = true;
        self.ui_model.begin_run(prompt);
        self.sync_ui_root_from_model();
    }

    /// Stream ended (None) — clear run flag; idle only if bridge already did.
    ///
    /// **Sole** scheduler for post-run footer token refresh (c1035). Do not also
    /// arm refresh on `AgentEnd`: production `drain_pending` often runs between
    /// those two signals and would kick two `estimate_context_tokens` jobs
    /// (duplicate `token.estimate` spans / wasted encode).
    pub fn on_run_stream_closed(&mut self) {
        self.run_active = false;
        self.suppress_xy_until_stream_end = false;
        self.ui_model.on_stream_closed_without_agent_end();
        self.pending.footer_token_refresh = true;
        self.sync_ui_root_from_model();
    }

    pub(crate) fn sync_ui_root_from_model(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().apply_ui_model(&self.ui_model);
    }

    /// Apply one host event and attempt a throttled render.
    pub fn step(&mut self, event: HostEvent) -> Result<(), XyDriverError> {
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
                // Prefer crossterm Resize payload: ioctl refresh can lag/stale.
                let prev_cols = self.tui.terminal.columns();
                let prev_rows = self.tui.terminal.rows();
                self.tui.terminal.refresh_size();
                self.tui.terminal.set_size_hint(cols, rows);
                let size_changed = self.tui.terminal.columns() != prev_cols
                    || self.tui.terminal.rows() != prev_rows;
                // Cursor / multiplexers often flood identical Resize; skip paint.
                if size_changed {
                    self.sync_layout_from_terminal();
                    // Align pi: resize → soft requestRender(); doRender sees
                    // width/heightChanged → fullRender(true) with 2J/H/3J.
                    // force=true would zero/sentinel-skip that path incorrectly.
                    self.tui.request_render(false);
                }
            }
            HostEvent::Input(input) => {
                if self.mode == LayoutMode::Ready {
                    if self.try_suppress_stale_esc(&input)
                        || self.try_paste_image(&input)
                        || self.try_busy_input(&input)
                        || self.try_idle_enter_submit(&input)
                        || self.try_ctrl_g(&input)
                    {
                        // Consumed — do not forward to editor (no newline / no tree).
                    } else {
                        self.tui.dispatch_event(input);
                    }
                }
                self.tui.request_render(false);
            }
            HostEvent::Xy(xy) => {
                if self.suppress_xy_until_stream_end {
                    // c670: abort already noted — do not revive busy via deltas / AgentEnd.
                    self.tui.request_render(false);
                } else {
                    apply_xy_event(&mut self.ui_model, &xy);
                    match xy.as_ref() {
                        XyEvent::AgentEnd { .. } => {
                            self.run_active = false;
                            // Footer token refresh is owned by [`Self::on_run_stream_closed`]
                            // (single end-of-run signal). Scheduling here as well caused two
                            // estimate jobs when drain ran between AgentEnd and stream close.
                        }
                        XyEvent::CompactionEnd { .. } | XyEvent::TurnEnd { .. } => {
                            self.request_footer_token_refresh();
                        }
                        XyEvent::MessageEnd { role, message } => {
                            // Mid-turn Api usage: assistant MessageEnd often carries usage.
                            let has_usage = message.as_ref().is_some_and(|m| {
                                matches!(
                                    m,
                                    crate::protocol::message::AgentMessage::Llm(
                                        crate::protocol::message::LlmMessage::AssistantMessage {
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
                    self.tui.request_render(false);
                }
            }
            HostEvent::FooterTokens { job_id, label } => {
                if job_id == self.footer_token_gen {
                    self.set_footer_token_label(label);
                    // Differential engine: only footer line should rewrite.
                    self.tui.request_render(false);
                }
            }
        }

        match self.tui.try_render() {
            Ok(painted) => {
                if painted {
                    self.paint_dirty = false;
                }
                Ok(())
            }
            Err(RenderError { .. }) => self.recover_from_render_error(),
        }
    }

    /// ath4: min-size shows a hint; only exit if even the hint cannot paint.
    /// Overflow at a "valid" size degrades to TooSmall instead of freezing/exiting.
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
            // Force clearing redraw so Ready chrome does not ghost over the hint.
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

    /// Toggle terminal task progress (OSC 9;4) — used while loading session list.
    pub fn set_task_progress(&mut self, active: bool) {
        self.tui.terminal.set_progress(active);
    }

    /// Emit a preformatted OSC 52 clipboard sequence on the host/UI thread.
    ///
    /// Must run outside differential render batches (caller: slash effect after
    /// `XyDriver::copy_text_to_clipboard`, before `render_now`). Never call from
    /// a blocking-pool clipboard worker.
    pub fn emit_clipboard_osc52(&mut self, sequence: &str) {
        self.tui.terminal.write(sequence);
        self.tui.terminal.flush();
    }

    pub fn render_now(&mut self) -> Result<(), XyDriverError> {
        match self.tui.render_now() {
            Ok(_) => {
                self.paint_dirty = false;
                Ok(())
            }
            Err(e) => {
                let err = XyDriverError::from_opaque(e.to_string());
                err.log_failure("tui.render_now");
                Err(err)
            }
        }
    }

    /// Re-evaluate layout from the terminal's current size (after an external
    /// size mutation on a test double).
    pub fn sync_layout_from_terminal(&mut self) {
        let cols = self.tui.terminal.columns();
        let rows = self.tui.terminal.rows();
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
