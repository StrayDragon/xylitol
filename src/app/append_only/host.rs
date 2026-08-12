//! Thin append-only host: Inline carrier + closed keys (atao2/atao6/atao7).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{
    Event, EventStream as CrosstermEventStream, KeyCode, KeyEventKind, KeyModifiers,
};
use futures::StreamExt;
use xylitol_tui::{
    Component, CrosstermTerminal, Editor, EditorOptions, EditorTheme, Focusable, InputEvent,
    InputListenerResult, InteractionMode, SystemClock, TUI, Terminal, Text, visible_width,
};

use crate::app::core::driver::{EventStream, XyDriver, XyDriverError};

use super::bridge::apply_xy_event;
use super::keys::is_exit_slash;
use super::model::AppendOnlyModel;
use super::spill::{ensure_session_spill_dir, install_process_spill_dir};

/// Options for [`run`].
#[derive(Clone, Debug)]
pub struct AppendOnlyRunOptions {
    /// Session store directory (session spill lives beside it).
    pub sessions_dir: PathBuf,
}

impl Default for AppendOnlyRunOptions {
    fn default() -> Self {
        Self {
            sessions_dir: default_sessions_dir(),
        }
    }
}

/// `~/.xylitol/sessions` without reaching `infra::session` from this surface.
fn default_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
        .join("sessions")
}

/// Failures before raw-mode (CLI-visible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendOnlyPreflightError {
    StdinNotTty,
    StdoutNotTty,
    NoModelSelected,
    TerminalSizeUnavailable(String),
}

impl std::fmt::Display for AppendOnlyPreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdinNotTty => write!(f, "append-only surface requires a TTY (stdin)"),
            Self::StdoutNotTty => write!(f, "append-only surface requires a TTY (stdout)"),
            Self::NoModelSelected => write!(
                f,
                "append-only surface requires a selected model; pass --model or configure config.yaml"
            ),
            Self::TerminalSizeUnavailable(e) => {
                write!(f, "cannot read terminal size ({e})")
            }
        }
    }
}

impl std::error::Error for AppendOnlyPreflightError {}

pub fn preflight(driver: &dyn XyDriver) -> Result<(), AppendOnlyPreflightError> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return Err(AppendOnlyPreflightError::StdinNotTty);
    }
    if !std::io::stdout().is_terminal() {
        return Err(AppendOnlyPreflightError::StdoutNotTty);
    }
    if driver.current_model().is_none() {
        return Err(AppendOnlyPreflightError::NoModelSelected);
    }
    if let Err(e) = crossterm::terminal::size() {
        return Err(AppendOnlyPreflightError::TerminalSizeUnavailable(
            e.to_string(),
        ));
    }
    Ok(())
}

/// Enter the append-only surface (host-driven Inline; never `TUI::start()`).
pub async fn run(
    driver: &mut dyn XyDriver,
    options: AppendOnlyRunOptions,
) -> Result<(), XyDriverError> {
    crate::app::tui::terminal_guard::install_lifecycle_hooks();
    let sid = driver
        .session_id()
        .unwrap_or_else(|| "anonymous".to_string());
    let spill_dir =
        ensure_session_spill_dir(&options.sessions_dir, &sid).map_err(XyDriverError::message)?;
    install_process_spill_dir(Some(spill_dir.clone()));

    let guard = crate::app::tui::terminal_guard::TerminalGuard::enter()?;
    let terminal = guard.take();
    let result = run_host_loop(terminal, driver, spill_dir).await;
    install_process_spill_dir(None);
    if result.is_err() {
        crate::app::tui::terminal_guard::emergency_restore();
    }
    result
}

async fn run_host_loop(
    terminal: CrosstermTerminal,
    driver: &mut dyn XyDriver,
    spill_dir: PathBuf,
) -> Result<(), XyDriverError> {
    let mut session = AppendOnlySession::new(terminal, spill_dir);
    session.render_now()?;

    let mut term_events = CrosstermEventStream::new();
    let mut agent_stream: Option<EventStream> = None;

    loop {
        if session.should_quit() || crate::app::tui::terminal_guard::exit_requested() {
            break;
        }

        // Drain pending submits from editor on_submit.
        if let Some(prompt) = session.take_submit() {
            if is_exit_slash(&prompt) {
                session.model.quit = true;
                continue;
            }
            if agent_stream.is_none() && !prompt.trim().is_empty() {
                session.model.busy = true;
                session.model.ctrl_c_armed = false;
                session.model.status_line = "running…".into();
                agent_stream = Some(driver.run(prompt.trim()).await);
                session.sync_transcript();
                session.render_now()?;
            }
        }

        tokio::select! {
            biased;
            maybe = term_events.next() => {
                let Some(Ok(ev)) = maybe else { continue };
                session.handle_crossterm(ev, agent_stream.is_some(), driver);
                session.sync_transcript();
                session.render_now()?;
            }
            event = async {
                match agent_stream.as_mut() {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                match event {
                    Some(xy) => {
                        apply_xy_event(&mut session.model, &xy)
                            .map_err(XyDriverError::message)?;
                        if matches!(
                            xy,
                            crate::app::core::driver::XyEvent::AgentEnd { .. }
                                | crate::app::core::driver::XyEvent::Error(_)
                        ) {
                            agent_stream = None;
                            session.model.busy = false;
                            session.model.status_line =
                                "append-only surface · /exit to quit".into();
                        }
                        session.sync_transcript();
                        session.render_now()?;
                    }
                    None => {
                        agent_stream = None;
                        session.model.busy = false;
                        session.sync_transcript();
                        session.render_now()?;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
    Ok(())
}

/// Shared root: transcript Text + Editor (no AO chrome / fold / plate).
struct AppendOnlyRoot {
    transcript: Rc<RefCell<String>>,
    editor: Rc<RefCell<Editor>>,
    focused: bool,
}

impl Component for AppendOnlyRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Text::new(self.transcript.borrow().clone(), 0, 0).render(width);
        let mut ed_lines = self.editor.borrow_mut().render(width);
        if ed_lines.is_empty() {
            let pad = " ".repeat(width.saturating_sub(visible_width("> ")));
            ed_lines.push(format!("> {pad}"));
        }
        lines.push(String::new());
        lines.extend(ed_lines);
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.editor.borrow_mut().handle_input(event);
    }

    fn invalidate(&mut self) {
        self.editor.borrow_mut().invalidate();
    }
}

impl Focusable for AppendOnlyRoot {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.editor.borrow_mut().set_focused(focused);
    }

    fn is_focused(&self) -> bool {
        self.focused
    }
}

/// Testable / production session shell (Inline mode).
pub struct AppendOnlySession<T: Terminal> {
    pub tui: TUI<T>,
    pub model: AppendOnlyModel,
    pub quit_flag: Arc<AtomicBool>,
    transcript: Rc<RefCell<String>>,
    pending_submit: Rc<RefCell<Option<String>>>,
    run_active: Rc<RefCell<bool>>,
}

impl<T: Terminal> AppendOnlySession<T> {
    pub fn new(terminal: T, spill_dir: PathBuf) -> Self {
        let mut tui = TUI::with_interaction_mode(terminal, InteractionMode::Inline);
        debug_assert!(tui.interaction_mode().is_inline());

        let transcript = Rc::new(RefCell::new(
            "append-only surface · /exit to quit\n".to_string(),
        ));
        let pending_submit = Rc::new(RefCell::new(None));
        let run_active = Rc::new(RefCell::new(false));
        let quit_flag = Arc::new(AtomicBool::new(false));

        let mut editor = Editor::new(
            EditorTheme::default(),
            EditorOptions {
                padding_x: 0,
                terminal_rows: 24,
            },
            Box::new(SystemClock),
        );
        let submit_slot = pending_submit.clone();
        editor.on_submit = Some(Box::new(move |text| {
            *submit_slot.borrow_mut() = Some(text);
        }));
        let editor = Rc::new(RefCell::new(editor));

        let root = AppendOnlyRoot {
            transcript: transcript.clone(),
            editor: editor.clone(),
            focused: true,
        };
        tui.add_child(Box::new(root));
        tui.set_focus(Some(0));

        // Closed-set listeners (atao6): swallow fold chords; idle double Ctrl+C quit.
        // Busy Esc / Ctrl+C abort is applied in handle_crossterm (needs XyDriver).
        let quit = quit_flag.clone();
        let run_flag = run_active.clone();
        let ctrl_armed = Rc::new(RefCell::new(false));
        tui.add_input_listener(move |ev| {
            let InputEvent::Key(key) = ev else {
                return InputListenerResult::Continue;
            };
            if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                return InputListenerResult::Continue;
            }
            // Forbid fold expand chords (atao3 / atao6)
            if (key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL))
                || (key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::ALT))
            {
                return InputListenerResult::Consumed;
            }
            let busy = *run_flag.borrow();
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if busy {
                    return InputListenerResult::Continue; // host aborts via driver
                }
                if *ctrl_armed.borrow() {
                    quit.store(true, Ordering::SeqCst);
                    return InputListenerResult::Consumed;
                }
                *ctrl_armed.borrow_mut() = true;
                return InputListenerResult::ConsumedRerender;
            }
            if !busy {
                *ctrl_armed.borrow_mut() = false;
            }
            InputListenerResult::Continue
        });

        tui.request_render(false);

        Self {
            tui,
            model: AppendOnlyModel::new(Some(spill_dir)),
            quit_flag,
            transcript,
            pending_submit,
            run_active,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.model.quit || self.quit_flag.load(Ordering::SeqCst)
    }

    pub fn interaction_mode(&self) -> InteractionMode {
        self.tui.interaction_mode()
    }

    pub fn take_submit(&mut self) -> Option<String> {
        self.pending_submit.borrow_mut().take()
    }

    pub fn sync_transcript(&mut self) {
        let body = format!(
            "{}\n{}",
            self.model.status_line,
            self.model.transcript_text()
        );
        *self.transcript.borrow_mut() = body;
        *self.run_active.borrow_mut() = self.model.busy;
        self.tui.request_render(false);
    }

    pub fn render_now(&mut self) -> Result<(), XyDriverError> {
        self.tui
            .try_render()
            .map_err(|e| XyDriverError::message(format!("render: {e}")))?;
        Ok(())
    }

    pub fn handle_crossterm(&mut self, ev: Event, run_active: bool, driver: &dyn XyDriver) {
        self.model.busy = run_active || self.model.busy;
        *self.run_active.borrow_mut() = self.model.busy;

        match ev {
            Event::Key(key)
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
            {
                let ctrl_c =
                    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
                let busy_abort = self.model.busy && (key.code == KeyCode::Esc || ctrl_c);
                if busy_abort {
                    driver.abort();
                    self.model.abort_latched = true;
                    self.model.status_line = "aborted".into();
                } else if ctrl_c && !self.model.busy {
                    if self.model.ctrl_c_armed {
                        self.model.quit = true;
                        self.quit_flag.store(true, Ordering::SeqCst);
                    } else {
                        self.model.ctrl_c_armed = true;
                        self.model.status_line = "press Ctrl+C again to quit (or /exit)".into();
                    }
                }

                let _ = self.tui.dispatch_event(InputEvent::Key(key));
            }
            Event::Paste(data) => {
                let _ = self.tui.dispatch_event(InputEvent::Paste(data));
            }
            Event::Resize(cols, rows) => {
                self.tui.terminal.set_size_hint(cols, rows);
                self.tui.request_render(true);
            }
            _ => {}
        }
    }
}

/// Headless pump for harness: apply scripted events without TTY.
#[cfg(test)]
pub async fn pump_scripted(
    model: &mut AppendOnlyModel,
    driver: &mut dyn XyDriver,
    prompt: &str,
) -> Result<(), String> {
    model.busy = true;
    let mut stream = driver.run(prompt).await;
    while let Some(ev) = stream.next().await {
        apply_xy_event(model, &ev)?;
        if matches!(
            ev,
            crate::app::core::driver::XyEvent::AgentEnd { .. }
                | crate::app::core::driver::XyEvent::Error(_)
        ) {
            break;
        }
    }
    model.busy = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemTerminal {
        cols: u16,
        rows: u16,
    }

    impl MemTerminal {
        fn new(cols: u16, rows: u16) -> Self {
            Self { cols, rows }
        }
    }

    impl Terminal for MemTerminal {
        fn write(&mut self, _s: &str) {}
        fn columns(&self) -> u16 {
            self.cols
        }
        fn rows(&self) -> u16 {
            self.rows
        }
        fn hide_cursor(&mut self) {}
        fn show_cursor(&mut self) {}
        fn clear_line(&mut self) {}
        fn clear_from_cursor(&mut self) {}
        fn clear_screen(&mut self) {}
        fn flush(&mut self) {}
    }

    #[test]
    fn atao2_session_binds_inline() {
        let tmp = tempfile::tempdir().unwrap();
        let session = AppendOnlySession::new(MemTerminal::new(80, 24), tmp.path().to_path_buf());
        assert!(session.interaction_mode().is_inline());
        assert!(!session.interaction_mode().is_application_owned());
    }
}
