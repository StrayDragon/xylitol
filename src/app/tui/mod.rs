//! Terminal UI application surface — host-driven on `xylitol_tui`.
//!
//! See `AGENTS.md` in this directory. Engine: `packages/xylitol-tui`.

mod host;
mod shell;
mod terminal_guard;

#[cfg(test)]
mod tests;

use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use xylitol_tui::{CrosstermTerminal, InputEvent, Terminal, matches_key_event};

use crate::app::core::driver::Driver;

use self::host::{HostEvent, HostSession};
use self::shell::build_root;
use self::terminal_guard::{TerminalGuard, exit_requested, install_lifecycle_hooks};

pub use self::host::{
    HostEvent as TuiHostEvent, HostSession as TuiHostSession, LayoutMode, MIN_COLS, MIN_ROWS,
    TOO_SMALL_HINT, is_too_small,
};

/// Enter the interactive TUI REPL (host-driven; never calls `TUI::start()`).
pub async fn run(_driver: &mut dyn Driver) -> Result<(), String> {
    install_lifecycle_hooks();
    tracing::info!(target: "xylitol::tui", "starting product TUI host");

    let guard = TerminalGuard::enter()?;
    let terminal = guard.take();
    // Drop guard without stop — we own the terminal and stop it below.
    // (take() already disarmed Drop.)

    let result = run_host_loop(terminal).await;

    // Always restore if the loop returned with the terminal still started.
    // run_host_loop stops on the happy path; emergency_restore covers panics.
    if let Err(ref e) = result {
        tracing::error!(target: "xylitol::tui", error = %e, "TUI host exited with error");
        terminal_guard::emergency_restore();
    }
    result
}

async fn run_host_loop(terminal: CrosstermTerminal) -> Result<(), String> {
    let mut session = HostSession::new(terminal, build_root);
    session.render_now()?;

    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while !session.should_quit() && !exit_requested() {
        tokio::select! {
            _ = ticker.tick() => {
                session.step(HostEvent::Tick)?;
            }
            maybe = events.next() => {
                match maybe {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                            continue;
                        }
                        if matches_key_event(&key, "ctrl+c") {
                            // Empty-shell: quit. (Editor clear lands with c480.)
                            session.request_quit();
                            continue;
                        }
                        session.step(HostEvent::Input(InputEvent::Key(key)))?;
                    }
                    Some(Ok(Event::Paste(data))) => {
                        session.step(HostEvent::Input(InputEvent::Paste(data)))?;
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        session.step(HostEvent::Resize { cols, rows })?;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        session.tui.terminal.stop();
                        return Err(format!("input error: {e}"));
                    }
                    None => {
                        session.request_quit();
                    }
                }
            }
        }
    }

    session.tui.terminal.stop();
    tracing::info!(target: "xylitol::tui", "product TUI host stopped");
    Ok(())
}
