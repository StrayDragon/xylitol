//! Terminal UI application surface — host-driven on `xylitol_tui`.
//!
//! See `AGENTS.md` in this directory. Engine: `packages/xylitol-tui`.

mod bridge;
mod glyphs;
mod host;
mod scrollback;
mod terminal_guard;
mod theme;
mod ui_root;

#[cfg(test)]
mod tests;

use std::time::Duration;

use crossterm::event::EventStream as CrosstermEventStream;
use crossterm::event::{Event, KeyEventKind};
use futures::StreamExt;
use xylitol_tui::{CrosstermTerminal, InputEvent, Terminal};

use crate::app::core::driver::{Driver, EventStream as AgentEventStream};

use self::host::{HostEvent, HostSession};
use self::terminal_guard::{TerminalGuard, exit_requested, install_lifecycle_hooks};

pub use self::bridge::{QueueBadge, UiEntry, UiModel, UiPhase, apply_xy_event};
pub use self::glyphs::GlyphSet;
pub use self::host::{
    HostEvent as TuiHostEvent, HostSession as TuiHostSession, LayoutMode, MIN_COLS, MIN_ROWS,
    TOO_SMALL_HINT, display_cwd, is_too_small,
};
pub use self::theme::ChromeTheme;

/// Failures that MUST abort before raw-mode / host loop (CLI-level, no TTY corruption).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiPreflightError {
    StdinNotTty,
    StdoutNotTty,
    NoModelSelected,
    TerminalSizeUnavailable(String),
}

impl std::fmt::Display for TuiPreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdinNotTty => {
                write!(
                    f,
                    "TUI requires an interactive terminal (stdin is not a TTY)"
                )
            }
            Self::StdoutNotTty => {
                write!(
                    f,
                    "TUI requires an interactive terminal (stdout is not a TTY)"
                )
            }
            Self::NoModelSelected => write!(
                f,
                "TUI requires a selected model; configure `.xylitol/config.local.yaml` or pass `--model`"
            ),
            Self::TerminalSizeUnavailable(e) => {
                write!(f, "cannot read terminal size ({e}); is this a real TTY?")
            }
        }
    }
}

impl std::error::Error for TuiPreflightError {}

/// Check environment before entering raw mode. Safe to call from CLI.
///
/// Does **not** open raw mode. Too-small terminals are allowed (in-TUI hint);
/// missing model / non-TTY MUST fail here so the user sees a normal CLI error.
///
/// Planned extensions (still CLI-level, never after raw mode):
/// - API key unresolved for the selected provider
/// - `--tui` forced on a pipe without a PTY
/// - (optional) unreachable `base_url` probe — keep off critical path unless requested
pub fn preflight(driver: &dyn Driver) -> Result<(), TuiPreflightError> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(TuiPreflightError::StdinNotTty);
    }
    if !std::io::stdout().is_terminal() {
        return Err(TuiPreflightError::StdoutNotTty);
    }
    if driver.current_model().is_none() {
        return Err(TuiPreflightError::NoModelSelected);
    }
    // Size probe without raw mode — catches pipes / broken PTY early.
    if let Err(e) = crossterm::terminal::size() {
        return Err(TuiPreflightError::TerminalSizeUnavailable(e.to_string()));
    }
    Ok(())
}

/// Enter the interactive TUI REPL (host-driven; never calls `TUI::start()`).
///
/// Callers MUST run [`preflight`] first (CLI does). This still fails closed if
/// `TerminalGuard::enter` cannot start the terminal.
pub async fn run(driver: &mut dyn Driver) -> Result<(), String> {
    install_lifecycle_hooks();
    tracing::info!(target: "xylitol::tui", "starting product TUI host");

    let guard = TerminalGuard::enter()?;
    let terminal = guard.take();

    let result = run_host_loop(terminal, driver).await;

    if let Err(ref e) = result {
        tracing::error!(target: "xylitol::tui", error = %e, "TUI host exited with error");
        terminal_guard::emergency_restore();
    }
    result
}

async fn run_host_loop(terminal: CrosstermTerminal, driver: &mut dyn Driver) -> Result<(), String> {
    let model = driver
        .current_model()
        .map(|m| {
            if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            }
        })
        .unwrap_or_else(|| "—".into());
    let mut session = HostSession::new_product_ui_with_chrome(terminal, host::display_cwd(), model);
    session.render_now()?;

    let mut term_events = CrosstermEventStream::new();
    let mut agent_stream: Option<AgentEventStream> = None;
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while !session.should_quit() && !exit_requested() {
        // Start a run if idle Enter queued a prompt.
        if agent_stream.is_none()
            && let Some(prompt) = session.take_submit()
        {
            tracing::info!(
                target: "xylitol::tui",
                prompt_len = prompt.len(),
                "Driver::run starting"
            );
            session.on_run_started(&prompt);
            let _ = session.render_now();
            agent_stream = Some(driver.run(&prompt).await);
        }

        tokio::select! {
            _ = ticker.tick() => {
                session.step(HostEvent::Tick)?;
            }
            maybe = term_events.next() => {
                match maybe {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                            continue;
                        }
                        // Ctrl+C clear/quit is handled by InputListener (c455).
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
            maybe_agent = async {
                match agent_stream.as_mut() {
                    Some(stream) => stream.next().await,
                    None => std::future::pending().await,
                }
            } => {
                match maybe_agent {
                    Some(xy) => {
                        session.step(HostEvent::Xy(Box::new(xy)))?;
                    }
                    None => {
                        tracing::debug!(target: "xylitol::tui", "agent EventStream ended");
                        agent_stream = None;
                        session.on_run_stream_closed();
                        let _ = session.tui.try_render();
                    }
                }
            }
        }
    }

    session.tui.terminal.stop();
    tracing::info!(target: "xylitol::tui", "product TUI host stopped");
    Ok(())
}
