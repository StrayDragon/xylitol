//! Product TUI host — event step machine (testable without a real TTY).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::{InputEvent, RenderError, TUI, Terminal, matches_key_event};

use crate::app::core::driver::XyEvent;

use super::bridge::{UiEntry, UiModel, UiPhase, apply_xy_event};
use super::ui_root::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};

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

/// Idle slash resolved for the async host loop (or applied locally).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingSlash {
    Exit,
    CycleModel,
    SetModel(String),
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
    /// True while a `Driver::run` stream is open (blocks duplicate submit).
    run_active: bool,
    chrome_cwd: String,
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
            run_active: false,
            chrome_cwd: display_cwd(),
        }
    }

    /// Product empty UI: shared `UiRoot` + Ctrl+C / Esc / idle-Enter listeners.
    pub fn new_product_ui(terminal: T) -> Self {
        Self::new_product_ui_with_chrome(terminal, display_cwd(), "—".into())
    }

    /// Product UI with footer identity (`cwd · model`).
    pub fn new_product_ui_with_chrome(terminal: T, cwd: String, model: String) -> Self {
        let ui_root = Rc::new(RefCell::new(UiRoot::new()));
        ui_root.borrow_mut().set_chrome_meta(cwd.clone(), model);
        let quit_flag = Arc::new(AtomicBool::new(false));
        let mut session = Self::new(terminal, shared_ui_root_rebuild(ui_root.clone()));
        session.chrome_cwd = cwd;
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
        self.ui_model.phase == UiPhase::Busy || self.run_active
    }

    pub fn run_active(&self) -> bool {
        self.run_active
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
            .set_chrome_meta(self.chrome_cwd.clone(), model);
        self.sync_ui_root_from_model();
    }

    /// Push a system line into the UI model (slash errors, notes).
    pub fn push_system_note(&mut self, text: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::System { text: text.into() });
        self.sync_ui_root_from_model();
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
        self.run_active = true;
        self.ui_model.begin_run(prompt);
        self.sync_ui_root_from_model();
    }

    /// Stream ended (None) — clear run flag; idle only if bridge already did.
    pub fn on_run_stream_closed(&mut self) {
        self.run_active = false;
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
                    if self.try_busy_input(&input) || self.try_idle_enter_submit(&input) {
                        // Consumed — do not forward to editor (no newline / no tree).
                    } else {
                        self.tui.dispatch_event(input);
                    }
                }
                self.tui.request_render(false);
            }
            HostEvent::Xy(xy) => {
                apply_xy_event(&mut self.ui_model, &xy);
                if matches!(xy.as_ref(), XyEvent::AgentEnd { .. }) {
                    self.run_active = false;
                }
                self.sync_ui_root_from_model();
                self.tui.request_render(false);
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
        if root.borrow().tree_open() {
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
            // pi chrome: dim Follow-up: above status — not a scrollback [steer] wall.
            self.ui_model.enqueue_follow_up_chrome(text.clone());
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
            root.remember_editor_send(text.clone());
            root.set_editor_text(String::new());
            drop(root);
            self.ui_model.enqueue_steer_chrome(text.clone());
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
        if root.tree_open() {
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
                PendingSlash::CycleModel | PendingSlash::SetModel(_) => {
                    self.pending_slash = Some(slash);
                }
            }
            return true;
        }

        if text.trim().starts_with('/') {
            root.set_editor_text(String::new());
            drop(root);
            self.push_system_note(format!(
                "unknown command: {} (try /exit, /model)",
                text.split_whitespace().next().unwrap_or("/")
            ));
            return true;
        }

        root.remember_editor_send(text.clone());
        root.set_editor_text(String::new());
        drop(root);
        self.pending_submit = Some(text);
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

/// Parse idle slash MVP (`/exit`, `/model` [id]).
fn parse_slash_command(text: &str) -> Option<PendingSlash> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix('/')?;
    if rest.is_empty() {
        return None;
    }
    let mut parts = rest.splitn(2, char::is_whitespace);
    let cmd = parts.next()?.to_ascii_lowercase();
    let arg = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    match (cmd.as_str(), arg) {
        ("exit" | "quit", _) => Some(PendingSlash::Exit),
        ("model", None) => Some(PendingSlash::CycleModel),
        ("model", Some(id)) => Some(PendingSlash::SetModel(id)),
        _ => None,
    }
}
