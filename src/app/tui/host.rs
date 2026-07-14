//! Product TUI host — event step machine (testable without a real TTY).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::{InputEvent, RenderError, TUI, Terminal, matches_key_event};

use crate::app::core::driver::{ModelInfo, XyEvent};
use crate::runtime_protocol::XyBashResult;

use super::bridge::session_tree::rebuild_scrollback_from_travel;
use super::bridge::{UiEntry, UiModel, UiPhase, apply_xy_event};
use super::commands::parse_slash_command;
use super::layout::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};
use crate::domain::session_types::{SessionEntry, SessionTreeTravel};
use xylitol_tui::TreeNode;
use xylitol_tui::components::select_list::SelectItem;

pub use super::commands::{
    BangParse, PendingBash, PendingSlash, bash_block_status, bash_output_body, parse_bang_command,
};

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
    /// Agent lifecycle event from `Driver::run` EventStream (c465).
    Xy(Box<XyEvent>),
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
    /// Idle Enter submit request (consumed by the async host loop).
    pending_submit: Option<String>,
    pending_steer: Option<String>,
    pending_follow_up: Option<String>,
    pending_abort: bool,
    /// Alt+Up: restore queued messages to editor and clear both driver queues.
    pending_dequeue: bool,
    pending_slash: Option<PendingSlash>,
    pending_bash: Option<PendingBash>,
    /// True while a `Driver::run` stream is open (blocks duplicate submit).
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
    layout_cwd: String,
    /// Test-only: force real external-editor path (skip harness stub gate).
    #[cfg(test)]
    force_real_external_editor: bool,
    /// Test-only: override `$VISUAL`/`$EDITOR` resolve (avoids process-wide env races).
    #[cfg(test)]
    external_editor_cmd_override: Option<Result<String, String>>,
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
        tui.request_render(true);
        Self {
            tui,
            mode,
            quit: false,
            quit_flag: Arc::new(AtomicBool::new(false)),
            ui_root: None,
            rebuild: Box::new(rebuild),
            ui_model: UiModel::new(),
            pending_submit: None,
            pending_steer: None,
            pending_follow_up: None,
            pending_abort: false,
            pending_dequeue: false,
            pending_slash: None,
            pending_bash: None,
            run_active: false,
            bash_active: false,
            suppress_idle_esc: false,
            suppress_xy_until_stream_end: false,
            layout_cwd: display_cwd(),
            #[cfg(test)]
            force_real_external_editor: false,
            #[cfg(test)]
            external_editor_cmd_override: None,
        }
    }

    /// Product empty UI: shared `UiRoot` + Ctrl+C / Esc / idle-Enter listeners.
    pub fn new_product_ui(terminal: T) -> Self {
        Self::new_product_ui_with_meta(terminal, display_cwd(), "—".into())
    }

    /// Product UI with footer identity (`cwd · model`).
    pub fn new_product_ui_with_meta(terminal: T, cwd: String, model: String) -> Self {
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

    pub fn is_busy(&self) -> bool {
        self.ui_model.phase == UiPhase::Busy || self.run_active || self.bash_active
    }

    pub fn run_active(&self) -> bool {
        self.run_active
    }

    pub fn bash_active(&self) -> bool {
        self.bash_active
    }

    /// Mark bang bash started: uplink Bash pending block, busy status `Running`.
    pub fn begin_bash_exec(&mut self, command: &str, exclude_from_context: bool) {
        self.pending_abort = false;
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
        self.pending_abort = false;
        if !self.run_active {
            self.ui_model.phase = UiPhase::Idle;
            self.ui_model.status = None;
        }
        self.sync_ui_root_from_model();
    }

    /// Immediate Esc abort feedback: one System `Aborted`, idle status (agent run; c665).
    /// Drops further agent Xy events until the current EventStream ends (c670).
    pub fn note_user_abort(&mut self) {
        self.ui_model.note_user_abort();
        self.bash_active = false;
        self.run_active = false;
        self.pending_abort = false;
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
        self.pending_abort = false;
        self.suppress_idle_esc = true;
        self.sync_ui_root_from_model();
    }

    /// Take an idle-Enter submit request, if any.
    pub fn take_submit(&mut self) -> Option<String> {
        self.pending_submit.take()
    }

    pub fn take_steer(&mut self) -> Option<String> {
        self.pending_steer.take()
    }

    pub fn take_follow_up(&mut self) -> Option<String> {
        self.pending_follow_up.take()
    }

    pub fn take_abort(&mut self) -> bool {
        std::mem::take(&mut self.pending_abort)
    }

    pub fn take_slash(&mut self) -> Option<PendingSlash> {
        self.pending_slash.take()
    }

    /// Take a pending idle bash (`!` / `!!`) request.
    pub fn take_bash(&mut self) -> Option<PendingBash> {
        self.pending_bash.take()
    }

    /// Append bash outcome into the pending Bash block (c668).
    pub fn push_bash_result(&mut self, _command: &str, result: &XyBashResult) {
        self.ui_model
            .finish_bash_block(bash_block_status(result), bash_output_body(result));
        self.sync_ui_root_from_model();
    }

    /// Append live bang output chunk (c669); caller renders on Tick/Done.
    pub fn append_bash_chunk(&mut self, chunk: &[u8]) {
        self.ui_model.append_bash_output(chunk);
        self.sync_ui_root_from_model();
    }

    /// Refresh queue badge from driver stats (after local steer/follow-up/abort).
    pub fn set_queue_badge(&mut self, steer_count: usize, follow_up_count: usize) {
        self.ui_model.sync_queue(steer_count, follow_up_count);
        self.sync_ui_root_from_model();
    }

    /// Take a pending Alt+Up dequeue request (clear both driver queues).
    pub fn take_dequeue(&mut self) -> bool {
        std::mem::take(&mut self.pending_dequeue)
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

    /// Push a system line into the UI model (slash errors, notes).
    pub fn push_system_note(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::System { text: text.into() });
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

    pub fn take_pending_session_tree_open(&mut self) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        root.borrow_mut().take_pending_tree_open()
    }

    /// Queue a MessageHistory tree open (c700 `/tree`).
    pub fn request_session_tree_open(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().request_tree_open();
        }
    }

    /// Queue fork at `entry_id` (c700 `/fork` / tree Shift+F).
    pub fn request_session_tree_fork(&mut self, entry_id: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().request_tree_fork(entry_id.into());
        }
    }

    pub fn take_pending_session_tree_travel(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_travel()
    }

    pub fn take_pending_session_tree_fork(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_fork()
    }

    pub fn take_pending_session_tree_label(&mut self) -> Option<(String, Option<String>)> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_label()
    }

    pub fn apply_session_tree_label(&mut self, id: &str, label: Option<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().apply_tree_label(id, label);
        }
    }

    pub fn mount_session_tree(&mut self, roots: Vec<TreeNode>, active_id: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut()
            .mount_session_tree(roots, active_id.as_deref());
        self.sync_ui_root_from_model();
    }

    pub fn take_pending_model_select(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_model_select()
    }

    pub fn mount_models_picker(&mut self, models: Vec<ModelInfo>, current_id: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let catalog: Vec<(String, String)> = models
            .iter()
            .map(|m| {
                let desc = if m.display_name.is_empty() {
                    String::new()
                } else {
                    m.display_name.clone()
                };
                (m.id.clone(), desc)
            })
            .collect();
        let items = models
            .iter()
            .map(|m| model_info_to_select_item(m, &current_id))
            .collect();
        {
            let mut root = root.borrow_mut();
            root.set_model_arg_catalog(catalog);
            root.mount_models_picker(items);
        }
        self.sync_ui_root_from_model();
    }

    /// Seed `/model <id>` completion catalog without opening the picker (c999).
    pub fn set_model_arg_catalog_from_models(&mut self, models: &[ModelInfo]) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let catalog = models
            .iter()
            .map(|m| {
                let desc = if m.display_name.is_empty() {
                    String::new()
                } else {
                    m.display_name.clone()
                };
                (m.id.clone(), desc)
            })
            .collect();
        root.borrow_mut().set_model_arg_catalog(catalog);
    }

    pub fn close_models_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_slot();
            self.sync_ui_root_from_model();
        }
    }

    pub fn apply_session_tree_travel(
        &mut self,
        travel: SessionTreeTravel,
        entries: Vec<SessionEntry>,
    ) {
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            if let Some(text) = travel.editor_text {
                root.set_editor_text(text);
            } else {
                root.set_editor_text(String::new());
            }
        }
        self.sync_ui_root_from_model();
    }

    /// After Driver fork+switch: rebuild transcript from child entries and optional prefill.
    pub fn apply_session_tree_fork(
        &mut self,
        child_id: &str,
        entries: Vec<SessionEntry>,
        editor_prefill: Option<String>,
    ) {
        let leaf_id = entries
            .iter()
            .rev()
            .find_map(|e| e.entry_id().map(str::to_string));
        let travel = SessionTreeTravel {
            kind: crate::domain::session_types::SessionTreeKind::MessageHistory,
            selected_id: child_id.to_string(),
            leaf_id,
            editor_text: editor_prefill.clone(),
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(UiEntry::System { text }) = self.ui_model.entries.first_mut() {
            *text = format!("forked → session {child_id}");
        }
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            root.set_editor_text(editor_prefill.unwrap_or_default());
        }
        self.sync_ui_root_from_model();
        self.push_system_note(format!("Forked to new session {child_id}"));
    }

    /// Apply `/debug <scene>` load: rebuild transcript and optional footer model (c710).
    pub fn apply_debug_scene(&mut self, load: crate::app::core::driver::DebugSceneLoad) {
        let leaf_id = load
            .entries
            .iter()
            .rev()
            .find_map(|e| e.entry_id().map(str::to_string));
        let travel = SessionTreeTravel {
            kind: crate::domain::session_types::SessionTreeKind::MessageHistory,
            selected_id: leaf_id.clone().unwrap_or_else(|| load.session_id.clone()),
            leaf_id,
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &load.entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            root.set_editor_text(String::new());
        }
        if let Some(m) = load.model {
            let label = if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            };
            self.set_footer_model(label);
        }
        self.sync_ui_root_from_model();
        self.push_system_note(load.note);
    }

    /// Queue a submit from the host loop / harness (idle only).
    pub fn request_submit(&mut self, prompt: impl Into<String>) {
        if self.run_active || self.ui_model.phase == UiPhase::Busy {
            return;
        }
        let prompt = prompt.into();
        if prompt.trim().is_empty() {
            return;
        }
        self.pending_submit = Some(prompt);
    }

    /// Mark that `Driver::run` has started; seeds the user entry + busy phase.
    pub fn on_run_started(&mut self, prompt: &str) {
        self.suppress_idle_esc = false;
        self.suppress_xy_until_stream_end = false;
        self.run_active = true;
        self.ui_model.begin_run(prompt);
        self.sync_ui_root_from_model();
    }

    /// Stream ended (None) — clear run flag; idle only if bridge already did.
    pub fn on_run_stream_closed(&mut self) {
        self.run_active = false;
        self.suppress_xy_until_stream_end = false;
        self.ui_model.on_stream_closed_without_agent_end();
        self.sync_ui_root_from_model();
    }

    fn sync_ui_root_from_model(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().apply_ui_model(&self.ui_model);
    }

    /// Apply one host event and attempt a throttled render.
    pub fn step(&mut self, event: HostEvent) -> Result<(), String> {
        match event {
            HostEvent::Quit => {
                self.quit = true;
            }
            HostEvent::Tick => {
                if self.mode == LayoutMode::Ready {
                    let _ = self.tui.idle_tick();
                }
                self.tui.request_render(false);
            }
            HostEvent::Resize { cols, rows } => {
                self.tui.terminal.set_size_hint(cols, rows);
                self.tui.terminal.refresh_size();
                self.sync_layout_from_terminal();
                self.tui.request_render(true);
            }
            HostEvent::Input(input) => {
                if self.mode == LayoutMode::Ready {
                    if self.try_suppress_stale_esc(&input)
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
                    if matches!(xy.as_ref(), XyEvent::AgentEnd { .. }) {
                        self.run_active = false;
                    }
                    self.sync_ui_root_from_model();
                    self.tui.request_render(false);
                }
            }
        }

        match self.tui.try_render() {
            Ok(_) => Ok(()),
            Err(RenderError { .. }) => {
                self.quit = true;
                Err("render failed: terminal too extreme; restoring and exiting".into())
            }
        }
    }

    /// Drop Esc backlog after abort while idle so the next bang is not cancelled
    /// at submit. Never suppress Esc while busy (second bang must stay abortable).
    fn try_suppress_stale_esc(&mut self, input: &InputEvent) -> bool {
        if !self.suppress_idle_esc {
            return false;
        }
        // New busy work owns Esc for abort — clear suppress and do not consume.
        if self.is_busy() {
            self.suppress_idle_esc = false;
            return false;
        }
        let InputEvent::Key(key) = input else {
            // Paste / other input: user moved on; clear suppress.
            self.suppress_idle_esc = false;
            return false;
        };
        if matches_key_event(key, "escape") {
            self.pending_abort = false;
            return true;
        }
        // Any other key (typing the next `!cmd`) ends idle Esc suppress.
        self.suppress_idle_esc = false;
        false
    }

    /// Busy Enter / Alt+Enter / Esc / Alt+Up (c480). Returns true when consumed.
    fn try_busy_input(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        if !self.is_busy() {
            return false;
        }
        let InputEvent::Key(key) = input else {
            return false;
        };
        if root.borrow().slot().is_overlay() {
            return false;
        }

        if matches_key_event(key, "escape") {
            self.pending_abort = true;
            // Drop untaken local steer so loop does not re-enqueue after abort.
            self.pending_steer = None;
            return true;
        }

        if matches_key_event(key, "alt+up") {
            let queued = self.ui_model.take_queued_for_editor();
            if queued.is_empty() {
                return true;
            }
            let mut root = root.borrow_mut();
            let current = root.editor_text();
            let mut parts = queued;
            if !current.trim().is_empty() {
                parts.push(current);
            }
            root.set_editor_text(parts.join("\n\n"));
            drop(root);
            self.pending_steer = None;
            self.pending_follow_up = None;
            self.pending_dequeue = true;
            self.sync_ui_root_from_model();
            return true;
        }

        if matches_key_event(key, "alt+enter") {
            let mut root = root.borrow_mut();
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
            }
            root.remember_editor_send(text.clone());
            root.set_editor_text(String::new());
            drop(root);
            // pi queue strip: dim Follow-up: above status — not a scrollback [steer] wall.
            self.ui_model.enqueue_follow_up_strip(text.clone());
            self.pending_follow_up = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        if matches_key_event(key, "enter") {
            let mut root = root.borrow_mut();
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
            }
            // c669: hard-reject second bang while interactive bash is still running.
            if self.bash_active {
                match parse_bang_command(&text) {
                    BangParse::NotBang => {}
                    BangParse::Empty { .. } | BangParse::Cmd { .. } => {
                        // Keep editor text; do not clear / do not execute.
                        drop(root);
                        self.push_system_note(
                            "bash already running — wait or Esc to cancel (second ! rejected)",
                        );
                        return true;
                    }
                }
            }
            root.remember_editor_send(text.clone());
            root.set_editor_text(String::new());
            drop(root);
            self.ui_model.enqueue_steer_strip(text.clone());
            self.pending_steer = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        false
    }

    /// Idle Enter: slash or prompt submit. Steer / Alt+Enter are busy-only.
    /// Returns true when Enter was consumed.
    fn try_idle_enter_submit(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        if self.run_active || self.ui_model.phase == UiPhase::Busy {
            return false;
        }
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_key_event(key, "enter") {
            return false;
        }
        let mut root = root.borrow_mut();
        if root.slot().is_overlay() {
            return false;
        }
        let text = root.editor_text();
        if text.trim().is_empty() {
            return false;
        }

        if let Some(slash) = parse_slash_command(&text) {
            root.set_editor_text(String::new());
            drop(root);
            match slash {
                PendingSlash::Exit => {
                    self.request_quit();
                }
                PendingSlash::OpenModels
                | PendingSlash::SetModel(_)
                | PendingSlash::DebugScene(_)
                | PendingSlash::OpenTree
                | PendingSlash::ForkAtLeaf => {
                    self.pending_slash = Some(slash);
                }
            }
            return true;
        }

        if text.trim().starts_with('/') {
            root.set_editor_text(String::new());
            drop(root);
            self.push_system_note(format!(
                "unknown command: {} (try /exit, /model, /tree, /fork)",
                text.split_whitespace().next().unwrap_or("/")
            ));
            return true;
        }

        match parse_bang_command(&text) {
            BangParse::NotBang => {}
            BangParse::Empty { .. } => {
                root.set_editor_text(String::new());
                drop(root);
                self.push_system_note("empty bash command (try !ls or !!ls)");
                return true;
            }
            BangParse::Cmd {
                command,
                exclude_from_context,
            } => {
                root.remember_editor_send(text.clone());
                root.set_editor_text(String::new());
                drop(root);
                self.pending_bash = Some(PendingBash {
                    command,
                    exclude_from_context,
                });
                return true;
            }
        }

        root.remember_editor_send(text.clone());
        root.set_editor_text(String::new());
        drop(root);
        self.pending_submit = Some(text);
        true
    }

    /// Ctrl+G: external editor (c650 / ati17) — TTY real path or harness stub.
    fn try_ctrl_g(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_key_event(key, "ctrl+g") {
            return false;
        }
        let mut root = root.borrow_mut();
        if root.slot().is_overlay() {
            return false;
        }

        let prefer_real = {
            #[cfg(test)]
            {
                self.force_real_external_editor
                    || super::external_editor::prefer_real_external_editor()
            }
            #[cfg(not(test))]
            {
                super::external_editor::prefer_real_external_editor()
            }
        };

        if !prefer_real {
            let chars = root.editor_text().len();
            root.open_external_editor_stub();
            drop(root);
            self.push_system_note(format!(
                "external editor stub (Ctrl+G) · {chars} chars · $EDITOR not spawned"
            ));
            return true;
        }

        let text = root.editor_text();
        drop(root);

        let editor_cmd = {
            #[cfg(test)]
            {
                if let Some(over) = self.external_editor_cmd_override.clone() {
                    match over {
                        Ok(cmd) => cmd,
                        Err(msg) => {
                            self.push_error_note(msg);
                            return true;
                        }
                    }
                } else {
                    match super::external_editor::resolve_external_editor_command() {
                        Ok(cmd) => cmd,
                        Err(msg) => {
                            self.push_error_note(msg);
                            return true;
                        }
                    }
                }
            }
            #[cfg(not(test))]
            {
                match super::external_editor::resolve_external_editor_command() {
                    Ok(cmd) => cmd,
                    Err(msg) => {
                        self.push_error_note(msg);
                        return true;
                    }
                }
            }
        };

        let outcome = self.tui.with_terminal_suspended(|| {
            super::external_editor::run_external_editor_process_with_command(&editor_cmd, &text)
        });
        match outcome {
            Ok(Some(new_text)) => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut().set_editor_text(new_text);
                }
            }
            Ok(None) => {
                self.push_error_note(
                    "external editor exited non-zero — keeping original text".to_string(),
                );
            }
            Err(err) => {
                self.push_error_note(format!("external editor failed: {err}"));
            }
        }
        // Soft render via step(); suspend kept previous_lines for differential
        // (no full-screen clear — inline TUI preserves scrollback above).
        true
    }

    /// Force an immediate frame (bypasses throttle) — useful after mount.
    pub fn render_now(&mut self) -> Result<(), String> {
        self.tui.render_now().map(|_| ()).map_err(|e| e.to_string())
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

fn model_info_to_select_item(m: &ModelInfo, current_id: &Option<String>) -> SelectItem {
    let label = if m.display_name.is_empty() {
        m.id.clone()
    } else {
        m.display_name.clone()
    };
    let marked = if current_id.as_deref() == Some(m.id.as_str()) {
        format!("{label} *")
    } else {
        label
    };
    SelectItem::new(m.id.clone(), marked)
}
