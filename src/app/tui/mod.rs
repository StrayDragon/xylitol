//! Terminal UI — inline REPL over the Driver seam.
//!
//! Event model (the key to reliability): the render loop is a `tokio::select!`
//! over **two mpsc channels** — keyboard events and agent XyEvents. Neither
//! uses crossterm's async `EventStream`: keyboard input is read with the
//! blocking `event::poll`/`event::read` on a dedicated `spawn_blocking` task,
//! and agent events are drained from the `Driver::run` stream on a tokio task.
//!
//! This deliberately avoids `EventStream` because ratatui's inline viewport
//! queries the cursor position via a synchronous DSR read of stdin on every
//! draw; `EventStream` spawns a competing stdin reader that swallows the DSR
//! response (crossterm#963), causing the cursor query to time out. With the
//! blocking reader isolated to its own task, the main loop's draw can read the
//! DSR response unobstructed. This is the same property the official ratatui
//! inline example relies on.
//!
//! Architecture constraints (spec c325 / `app-tui`):
//! - inline render (`Viewport::Inline`, NOT alt-screen);
//! - driven by `InProcessDriver` (same path as print);
//! - depends on `ratatui-core` + `ratatui-crossterm` directly (not the
//!   umbrella `ratatui` crate); uses no built-in widgets — components are
//!   hand-rolled via `Line::render` or direct `Buffer` writes (c341);
//! - imports only `app::core` (Driver/composition), `protocol`, `domain` —
//!   never `agent::*` internals or `infra`.

pub mod app;
mod commands;
mod components;
mod init;
mod input;
mod render;
mod terminal;
mod theme;

use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::core::driver::Driver;
use crate::app::tui::app::TuiApp;
use crate::app::tui::commands::{CommandOutcome, dispatch};
use crate::app::tui::input::{InputOutcome, handle as handle_key};
use crate::app::tui::render::RenderedLine;
use crate::app::tui::terminal::InlineTerminal;

/// Messages delivered to the main loop.
enum Msg {
    /// A terminal key/mouse/resize event from the blocking reader task.
    Key(Event),
    /// The keyboard reader task ended (stdin closed).
    KeyEof,
    /// A timed tick (drives the spinner animation at a steady cadence).
    Tick,
    /// An agent XyEvent for the current turn.
    Xy(Box<crate::domain::lifecycle::XyEvent>),
    /// The agent event stream ended (stream `None` or drain cancel) — the
    /// whole user turn is done. This is distinct from an intermediate
    /// `XyEvent::TurnEnd`, which only marks a single ReAct iteration
    /// boundary in a multi-round tool-calling turn (c370).
    XyDone,
}

/// Run the inline TUI REPL against a constructed driver.
///
/// The caller (cli dispatch) assembles the driver via the shared composition
/// root, identical to print mode. This function owns the terminal lifecycle.
pub async fn run(driver: &mut dyn Driver) -> Result<(), String> {
    // Install the panic-safety hook BEFORE entering the viewport (c355): if
    // enter() itself panics, or any later panic occurs (incl. in spawned tasks
    // or under panic=abort), this hook restores the terminal. Drop alone cannot.
    init::install_terminal_restore_hook();
    let mut term = InlineTerminal::enter().map_err(|e| format!("enter terminal: {e}"))?;

    let greeting = "xylitol — type a prompt and press Enter. /exit to quit.";
    term.commit_to_scrollback(&[RenderedLine::Status(greeting.to_string())])
        .map_err(|e| format!("greeting: {e}"))?;

    let mut app = TuiApp::default();
    // Per-turn cancellation token. A fresh token is created for each turn in
    // the Submit branch; aborting cancels the current one. A single global
    // token would stay cancelled forever after the first abort, so every
    // later turn's drain (which runs to stream end, c370) would break
    // immediately and drop all events.
    let mut current_cancel: Option<Arc<CancellationToken>> = None;

    // Channel feeding the main loop. The keyboard reader runs as a blocking
    // task; agent events are spawned per-turn by TuiApp; a steady timer drives
    // the spinner animation.
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    spawn_keyboard_reader(tx.clone());
    spawn_tick(tx.clone(), Duration::from_millis(120));

    term.draw_tail(&app).map_err(|e| format!("draw: {e}"))?;
    let result = repl_loop(
        driver,
        &mut term,
        &mut app,
        &mut rx,
        &tx,
        &mut current_cancel,
    )
    .await;

    // Terminal restoration happens in InlineTerminal::Drop regardless of result.
    result
}

/// Spawn a dedicated blocking task that reads crossterm events and forwards
/// them. Using blocking `event::poll`/`read` (NOT `EventStream`) keeps stdin
/// free for ratatui's synchronous cursor query on the main thread.
fn spawn_keyboard_reader(tx: mpsc::UnboundedSender<Msg>) {
    tokio::task::spawn_blocking(move || {
        loop {
            // poll with a short timeout so the task remains responsive to drop.
            if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if tx.is_closed() {
                    break;
                }
                continue;
            }
            match event::read() {
                Ok(ev) => {
                    if tx.send(Msg::Key(ev)).is_err() {
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

/// Spawn a steady timer that drives the spinner animation while a turn streams.
/// Without this the spinner only advances on XyEvent arrivals (uneven speed).
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

async fn repl_loop(
    driver: &mut dyn Driver,
    term: &mut InlineTerminal,
    app: &mut TuiApp,
    rx: &mut mpsc::UnboundedReceiver<Msg>,
    tx: &mpsc::UnboundedSender<Msg>,
    current_cancel: &mut Option<Arc<CancellationToken>>,
) -> Result<(), String> {
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::KeyEof => break,
            Msg::Tick => {
                // Steady spinner animation: advance the frame and redraw the
                // tail only while streaming (idle needs no animation).
                if app.is_streaming() {
                    app.tick_spinner();
                    term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
                }
            }
            Msg::Key(ev) => {
                let Event::Key(key) = ev else {
                    continue;
                };
                match handle_key(key, app) {
                    InputOutcome::Submit(prompt) => {
                        // If a turn is already streaming, interrupt it before
                        // starting the new one (the input box is always active).
                        if app.is_streaming() {
                            driver.abort();
                            if let Some(c) = current_cancel.take() {
                                c.cancel();
                            }
                            app.end_stream();
                        }
                        // Echo the user's prompt to scrollback BEFORE the reply
                        // stream starts (修复 c340 §7 #3: user message was never
                        // committed). input_buffer is already cleared by
                        // take_input, so no double-display in the tail.
                        term.commit_to_scrollback(&[render::user_message_rendered(&prompt)])
                            .map_err(|e| format!("commit user msg: {e}"))?;
                        let stream = driver.run(&prompt).await;
                        app.start_stream();
                        // Fresh per-turn cancel token (c370): a global token
                        // would stay cancelled after the first abort and starve
                        // every later turn's drain.
                        let cancel = Arc::new(CancellationToken::new());
                        *current_cancel = Some(cancel.clone());
                        // Bridge the agent's async XyEvent stream into the main
                        // loop's Msg channel via a per-turn XyEvent channel.
                        let (xy_tx, mut xy_rx) =
                            mpsc::unbounded_channel::<crate::domain::lifecycle::XyEvent>();
                        TuiApp::spawn_drain(stream, xy_tx, cancel);
                        let main_tx = tx.clone();
                        tokio::spawn(async move {
                            while let Some(ev) = xy_rx.recv().await {
                                if main_tx.send(Msg::Xy(Box::new(ev))).is_err() {
                                    return;
                                }
                            }
                            // drain task ended (stream `None` or cancel) and
                            // dropped xy_tx → recv returned None → the whole
                            // user turn is done. Signal the main loop to clear
                            // the mutable tail / reset streaming state (c370).
                            let _ = main_tx.send(Msg::XyDone);
                        });
                        term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
                    }
                    InputOutcome::Slash(body) => {
                        match dispatch(&body, driver).await {
                            CommandOutcome::Quit => break,
                            CommandOutcome::Handled => {}
                            CommandOutcome::Unknown(msg) => {
                                term.commit_to_scrollback(&[RenderedLine::Status(msg)])
                                    .map_err(|e| format!("commit: {e}"))?;
                            }
                        }
                        term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
                    }
                    InputOutcome::Abort => {
                        if app.is_streaming() {
                            driver.abort();
                            if let Some(c) = current_cancel.take() {
                                c.cancel();
                            }
                            app.end_stream();
                        } else {
                            break;
                        }
                        term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
                    }
                    InputOutcome::Quit => break,
                    InputOutcome::Idle => {
                        term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
                    }
                }
            }
            Msg::Xy(ev) => {
                let lines = app.handle_xy_event(*ev);
                // c365 buffer route: all returned lines are finalized (streaming
                // complete AssistantText, ToolSummary, Status) → commit to
                // scrollback via insert_before. The un-terminated mutable tail is
                // NOT returned here — it lives in `pending_tail()` and is rendered
                // by the `MutableLine` widget (via `Tail`) each draw_tail. No
                // escape direct-write; everything is in the ratatui buffer
                // (TestBackend-verifiable, spec tui41/tui42).
                // An intermediate XyEvent::TurnEnd is handled here like any other
                // event (it flushes the current iteration's complete lines); it
                // does NOT end the turn — only Msg::XyDone does (c370).
                if !lines.is_empty() {
                    term.commit_to_scrollback(&lines)
                        .map_err(|e| format!("commit: {e}"))?;
                }
                term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
            }
            Msg::XyDone => {
                // The agent stream ended: the whole user turn is done. Clear the
                // mutable tail and reset streaming state. (`Tail` clears its area
                // each frame and `end_stream` resets `pending_tail`.)
                app.end_stream();
                term.draw_tail(app).map_err(|e| format!("draw: {e}"))?;
            }
        }
    }
    Ok(())
}
