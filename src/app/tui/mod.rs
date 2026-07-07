//! Terminal UI — inline REPL over the Driver seam, on the c399 line-array engine.
//!
//! c399 stage 4 rewrote this from the ratatui inline-viewport model to pi-tui's
//! retained-widget + line-array + differential-rendering model
//! (`.agents/skills/tui-pro-of-pi-tui/`). The conversation history is one
//! growing `Vec<StyledLine>`; each frame the engine diffs previous vs new and
//! writes only changed lines (CSI 2026 synchronized output).
//!
//! ## Event model
//!
//! The host loop is a `tokio::select!` over **three sources**: keyboard events
//! (a `spawn_blocking` task doing blocking `event::poll`/`read` — the new
//! engine's `ProcessTerminal` writes only to stdout, so stdin is free for this
//! reader, unlike the old ratatui inline DSR-query conflict), agent XyEvents
//! (drained from `Driver::run` via `TuiApp::spawn_drain`), and a steady tick
//! (drives the Loader spinner).
//!
//! ## Host ↔ engine ownership
//!
//! The engine owns the render tree root (`Box<dyn Component>`). The skill says
//! the host owns the "transcript surface" on top of the engine + widgets
//! (ux.md). We bridge this by wrapping the transcript/input/loader widgets in
//! `Rc<RefCell<>>` and inserting [`SharedComponent`] shims into the root
//! `Container`; the host keeps `Rc` clones and mutates the widgets directly,
//! then calls `request_render` + `try_render`.
//!
//! Architecture constraints (spec `app-tui`):
//! - inline render (NOT alt-screen);
//! - driven by `InProcessDriver` (same path as print);
//! - imports only `app::core` (Driver/composition), `protocol`, `domain` —
//!   never `agent::*` internals or `infra`.

pub mod app;
mod commands;
pub mod engine;
mod init;
mod render;
pub mod syntect_highlight;
pub mod theme;
mod transcript;
pub mod widgets;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::core::driver::Driver;
use crate::app::tui::app::TuiApp;
use crate::app::tui::commands::{CommandOutcome, dispatch};
use crate::app::tui::engine::component::{Component, Container, InputResult};
use crate::app::tui::engine::style::{Span, StyledLine};
use crate::app::tui::engine::terminal::{ProcessTerminal, Terminal};
use crate::app::tui::engine::tui::Tui;
use crate::app::tui::render::{RenderedLine, user_message_rendered};
use crate::app::tui::transcript::TranscriptWidget;
use crate::app::tui::widgets::input::Input;
use crate::app::tui::widgets::loader::Loader;

/// A thin shim that lets the engine own a widget inside the render tree while
/// the host retains an `Rc<RefCell<T>>` handle for mutation. All `Component`
/// calls forward into the borrow. This is how the host grows the transcript /
/// drives the input without the engine exposing `root_mut` (which it can't:
/// `root: Box<dyn Component>` is a trait object with no downcast to
/// `Container::add`).
struct SharedComponent<T: Component> {
    inner: Rc<RefCell<T>>,
}

impl<T: Component> Component for SharedComponent<T> {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        self.inner.borrow().render(width)
    }
    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) -> InputResult {
        self.inner.borrow_mut().handle_input(key)
    }
    fn invalidate(&mut self) {
        self.inner.borrow_mut().invalidate();
    }
    fn focused(&self) -> bool {
        self.inner.borrow().focused()
    }
    fn set_focused(&mut self, focused: bool) {
        self.inner.borrow_mut().set_focused(focused);
    }
    fn take_outcome(&mut self) -> Option<crate::app::tui::engine::outcome::UxOutcome> {
        self.inner.borrow_mut().take_outcome()
    }
}

/// Messages delivered to the main loop.
enum Msg {
    /// A terminal key/mouse/resize/paste event from the blocking reader task.
    Term(Event),
    /// The keyboard reader task ended (stdin closed).
    KeyEof,
    /// A timed tick (drives the Loader spinner at a steady cadence).
    Tick,
    /// An agent XyEvent for the current turn.
    Xy(Box<crate::domain::lifecycle::XyEvent>),
    /// The agent event stream ended (stream `None` or drain cancel) — the
    /// whole user turn is done. Distinct from an intermediate `TurnEnd`, which
    /// only marks a single ReAct iteration boundary (c370).
    XyDone,
}

/// Run the inline TUI REPL against a constructed driver.
///
/// The caller (cli dispatch) assembles the driver via the shared composition
/// root, identical to print mode. This function owns the terminal lifecycle.
pub async fn run(driver: &mut dyn Driver) -> Result<(), String> {
    // Panic-restore hook BEFORE entering raw mode (c355): if ProcessTerminal's
    // start panics, or any later panic occurs (incl. spawned tasks or under
    // panic=abort), this hook restores the terminal. Drop alone cannot.
    init::install_terminal_restore_hook();
    let term = ProcessTerminal::start().map_err(|e| format!("enter terminal: {e}"))?;
    let width = term.width() as usize;

    // Host-owned widgets, shared with the engine via Rc<RefCell>.
    let transcript = Rc::new(RefCell::new(TranscriptWidget::new()));
    let input = Rc::new(RefCell::new(Input::new()));
    let loader = Rc::new(RefCell::new(Loader::new("Ready")));

    // Greeting line (committed finalized row).
    let greeting = status_line("xylitol — type a prompt and press Enter. /exit to quit.");
    transcript.borrow_mut().commit(greeting);

    // Build the render tree: transcript history + spacer + loader + input.
    // Focus is on the input (index 3 after transcript/spacer/loader).
    let mut root = Container::new();
    root.add(Box::new(SharedComponent {
        inner: transcript.clone(),
    }));
    root.add(Box::new(SpacerLine));
    root.add(Box::new(SharedComponent {
        inner: loader.clone(),
    }));
    root.add(Box::new(SharedComponent {
        inner: input.clone(),
    }));
    root.set_focused_index(Some(3));
    input.borrow_mut().set_focused(true);

    let mut tui = Tui::new(term, Box::new(root));
    tui.render_now()
        .map_err(|e| format!("first render: {e:?}"))?;

    let mut app = TuiApp::default();
    // Per-turn cancellation token. A fresh token is created for each turn;
    // aborting cancels the current one (c370: a global token would stay
    // cancelled forever and starve later turns).
    let mut current_cancel: Option<Arc<CancellationToken>> = None;

    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    spawn_keyboard_reader(tx.clone());
    // pi's Loader ticks at 80ms (loader.ts DEFAULT_INTERVAL_MS); match that
    // cadence so the spinner animates as smoothly as the reference.
    spawn_tick(tx.clone(), Duration::from_millis(80));

    let result = repl_loop(
        driver,
        &mut tui,
        &mut app,
        &transcript,
        &input,
        &loader,
        &mut rx,
        &tx,
        &mut current_cancel,
        width,
    )
    .await;

    // Clean exit: move cursor past content, restore terminal.
    tui.stop();
    result
}

/// A zero-height separator between transcript and the loader/input area. Renders
/// one blank line so the input stays visually distinct from the history.
struct SpacerLine;
impl Component for SpacerLine {
    fn render(&self, _width: usize) -> Vec<StyledLine> {
        vec![StyledLine::empty()]
    }
}

fn status_line(msg: &str) -> Vec<StyledLine> {
    let p = theme::palette();
    let mut line = StyledLine::new();
    line.spans.push(Span::styled(msg.to_string(), p.text_dim()));
    vec![line]
}

/// Render the current pending tail (streaming reply / thinking / tool status)
/// into styled rows for the transcript's pending region.
fn pending_tail_rows(app: &TuiApp, width: usize) -> Vec<StyledLine> {
    use crate::app::tui::app::MutableKind as K;
    let Some((text, kind)) = app.pending_tail() else {
        return Vec::new();
    };
    let p = theme::palette();
    let style = match kind {
        K::Thinking => p.thinking(),
        K::Text => p.assistant(),
        K::Tool => p.tool(),
    };
    // Wrap the single pending line to width (CJK-aware) so the engine's
    // hard-width invariant holds; each wrapped row carries the same style.
    let mut line = StyledLine::new();
    line.spans.push(Span::styled(text.to_string(), style));
    crate::app::tui::engine::width::wrap(&line, width)
}

/// Spawn a dedicated blocking task that reads crossterm events and forwards
/// them. Uses blocking `event::poll`/`read` (NOT `EventStream`). The new
/// engine's `ProcessTerminal` writes only to stdout (no DSR cursor query like
/// the old ratatui inline viewport), so stdin is free for this reader.
fn spawn_keyboard_reader(tx: mpsc::UnboundedSender<Msg>) {
    tokio::task::spawn_blocking(move || {
        loop {
            if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if tx.is_closed() {
                    break;
                }
                continue;
            }
            match event::read() {
                Ok(ev) => {
                    if tx.send(Msg::Term(ev)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(Msg::KeyEof);
                    break;
                }
            }
        }
    });
}

/// Spawn a steady timer that drives the Loader spinner while a turn streams.
fn spawn_tick(tx: mpsc::UnboundedSender<Msg>, period: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if tx.send(Msg::Tick).is_err() {
                break;
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn repl_loop(
    driver: &mut dyn Driver,
    tui: &mut Tui<ProcessTerminal>,
    app: &mut TuiApp,
    transcript: &Rc<RefCell<TranscriptWidget>>,
    input: &Rc<RefCell<Input>>,
    loader: &Rc<RefCell<Loader>>,
    rx: &mut mpsc::UnboundedReceiver<Msg>,
    tx: &mpsc::UnboundedSender<Msg>,
    current_cancel: &mut Option<Arc<CancellationToken>>,
    mut width: usize,
) -> Result<(), String> {
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::KeyEof => break,
            Msg::Tick => {
                // Advance the spinner only while streaming; idle keeps a steady
                // "Ready" with no animation. The Tick branch MUST pump its own
                // render (try_render) — unlike Term/Xy, no sibling branch drives
                // a frame here, so without this the spinner frame advances in
                // memory but never paints, looking frozen during sparse streaming.
                if app.is_streaming() {
                    loader.borrow_mut().advance();
                    tui.request_render(false);
                }
                // Defensive flush: always pump a pending render frame on tick.
                // Without this, a render that landed just before an XyDone (and
                // was rate-limit-skipped) would stay pending forever once idle
                // — because this very branch is the only thing that runs while
                // waiting for the user's next keystroke. try_render is a cheap
                // no-op when nothing is pending.
                tui.try_render().map_err(|e| format!("render: {e:?}"))?;
            }
            Msg::Term(ev) => {
                let outcome = handle_term_event(ev, tui, input, &mut width);
                if let Some(action) = outcome {
                    let stop = apply_host_action(
                        action,
                        driver,
                        app,
                        transcript,
                        input,
                        loader,
                        tx,
                        current_cancel,
                        width,
                    )
                    .await?;
                    if stop {
                        break;
                    }
                }
                tui.try_render().map_err(|e| format!("render: {e:?}"))?;
            }
            Msg::Xy(ev) => {
                let width_now = tui.term_mut().width() as usize;
                apply_xy_event(*ev, app, transcript, loader, width_now);
                tui.request_render(false);
                tui.try_render().map_err(|e| format!("render: {e:?}"))?;
            }
            Msg::XyDone => {
                app.end_stream();
                transcript.borrow_mut().clear_pending();
                loader.borrow_mut().set_message("Ready");
                loader.borrow_mut().set_spinning(false);
                // The final "turn done" frame MUST paint — it is the user's
                // confirmation the turn is over. `request_render(false) +
                // try_render()` was throttled (the preceding Xy event rendered
                // <16ms ago, so try_render no-ops), leaving a stale "Working…"
                // frame on screen until the user typed. render_now() bypasses
                // the rate limit for this terminal turn-boundary.
                tui.render_now().map_err(|e| format!("render: {e:?}"))?;
            }
        }
    }
    Ok(())
}

/// The high-level action the host must take after a terminal event (the
/// `UxOutcome` plus a paste marker). Returned by `handle_term_event` and
/// executed by `apply_host_action`, keeping the dispatch logic testable.
enum HostAction {
    /// Engine surfaced a UX outcome (Submit/Slash/Abort/Quit/Idle).
    Ux(crate::app::tui::engine::outcome::UxOutcome),
    /// A paste event was already injected into the Input widget.
    Paste,
    /// Resize: width already updated; just force a redraw.
    Resize,
    /// Nothing actionable.
    None,
}

/// Handle one terminal event: route keys through the engine, inject pastes,
/// and track resize. Returns the host action to take (if any).
fn handle_term_event(
    ev: Event,
    tui: &mut Tui<ProcessTerminal>,
    input: &Rc<RefCell<Input>>,
    width: &mut usize,
) -> Option<HostAction> {
    match ev {
        Event::Key(key) => {
            let outcome = tui.handle_event(&key);
            outcome.map(HostAction::Ux)
        }
        Event::Paste(text) => {
            input.borrow_mut().insert_paste(&text);
            tui.request_render(false);
            Some(HostAction::Paste)
        }
        Event::Resize(cols, _rows) => {
            *width = cols as usize;
            // A width change invalidates cached wraps: force a full redraw so
            // the transcript re-flows. (Finalized rows were committed at their
            // original width; the engine re-wraps pending on next commit. For
            // MVP we accept that historical rows keep their committed wrap —
            // only pending + future rows re-wrap.)
            tui.request_render(true);
            Some(HostAction::Resize)
        }
        _ => Some(HostAction::None),
    }
}

/// Apply a host action that touches the agent/driver layer. Returns `true` if
/// the REPL should stop (Quit / Ctrl+D-idle / Ctrl+C-idle).
#[allow(clippy::too_many_arguments)]
async fn apply_host_action(
    action: HostAction,
    driver: &mut dyn Driver,
    app: &mut TuiApp,
    transcript: &Rc<RefCell<TranscriptWidget>>,
    input: &Rc<RefCell<Input>>,
    loader: &Rc<RefCell<Loader>>,
    tx: &mpsc::UnboundedSender<Msg>,
    current_cancel: &mut Option<Arc<CancellationToken>>,
    width: usize,
) -> Result<bool, String> {
    use crate::app::tui::engine::outcome::UxOutcome;
    let ux = match action {
        HostAction::Ux(ux) => ux,
        HostAction::Paste | HostAction::Resize | HostAction::None => return Ok(false),
    };
    match ux {
        UxOutcome::Submit(prompt) => {
            // Interrupt any in-flight turn before starting a new one.
            if app.is_streaming() {
                driver.abort();
                if let Some(c) = current_cancel.take() {
                    c.cancel();
                }
                app.end_stream();
                transcript.borrow_mut().clear_pending();
            }
            // Echo the user's prompt into history, then start the turn.
            let rows = user_message_rendered(&prompt).to_lines(width);
            transcript.borrow_mut().commit(rows);
            input.borrow_mut().clear();

            let stream = driver.run(&prompt).await;
            app.start_stream();
            loader.borrow_mut().set_message("Working…");
            loader.borrow_mut().set_spinning(true);
            let cancel = Arc::new(CancellationToken::new());
            *current_cancel = Some(cancel.clone());
            let (xy_tx, mut xy_rx) = mpsc::unbounded_channel::<crate::domain::lifecycle::XyEvent>();
            TuiApp::spawn_drain(stream, xy_tx, cancel);
            tracing::debug!(target: "xylitol::tui", len = prompt.len(), "turn submitted");
            let main_tx = tx.clone();
            tokio::spawn(async move {
                while let Some(ev) = xy_rx.recv().await {
                    tracing::trace!(
                        target: "xylitol::tui",
                        kind = ?std::mem::discriminant(&ev),
                        "xy event"
                    );
                    if main_tx.send(Msg::Xy(Box::new(ev))).is_err() {
                        return;
                    }
                }
                let _ = main_tx.send(Msg::XyDone);
            });
            Ok(false)
        }
        UxOutcome::Slash(body) => match dispatch(&body, driver).await {
            CommandOutcome::Quit => Ok(true),
            CommandOutcome::Handled(msg) => {
                if let Some(text) = msg {
                    transcript.borrow_mut().commit(status_line(&text));
                }
                Ok(false)
            }
            CommandOutcome::Unknown(msg) => {
                transcript.borrow_mut().commit(status_line(&msg));
                Ok(false)
            }
        },
        UxOutcome::Abort => {
            if app.is_streaming() {
                driver.abort();
                if let Some(c) = current_cancel.take() {
                    c.cancel();
                }
                app.end_stream();
                transcript.borrow_mut().clear_pending();
                loader.borrow_mut().set_message("Ready");
                loader.borrow_mut().set_spinning(false);
            } else {
                // Idle Ctrl+C → quit (matches legacy behavior).
                return Ok(true);
            }
            Ok(false)
        }
        UxOutcome::Quit => Ok(true),
        UxOutcome::Idle => Ok(false),
    }
}

/// Apply one XyEvent: update streaming state, commit finalized rows, refresh
/// the pending tail and loader status.
fn apply_xy_event(
    ev: crate::domain::lifecycle::XyEvent,
    app: &mut TuiApp,
    transcript: &Rc<RefCell<TranscriptWidget>>,
    loader: &Rc<RefCell<Loader>>,
    width: usize,
) {
    // Drive the loader message from tool status.
    if let crate::domain::lifecycle::XyEvent::ToolExecutionStart { name, .. } = &ev {
        loader.borrow_mut().set_message(format!("running {name}"));
    }
    if let crate::domain::lifecycle::XyEvent::ToolExecutionEnd { name, .. } = &ev {
        loader.borrow_mut().set_message(format!("{name} done"));
    }

    let rendered: Vec<RenderedLine> = app.handle_xy_event(ev);
    // Commit finalized rows (complete lines + seam-produced tool/status lines).
    let mut rows = Vec::new();
    for line in rendered {
        rows.extend(line.to_lines(width));
    }
    if !rows.is_empty() {
        transcript.borrow_mut().commit(rows);
    }
    // Refresh the streaming pending tail.
    let pending = pending_tail_rows(app, width);
    if pending.is_empty() {
        transcript.borrow_mut().clear_pending();
    } else {
        transcript.borrow_mut().set_pending(pending);
    }
}
