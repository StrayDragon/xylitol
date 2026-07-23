//! Terminal UI application surface — host-driven on `xylitol_tui`.
//!
//! See `AGENTS.md` in this directory. Engine: `packages/xylitol-tui`.

mod bridge;
mod commands;
mod editor_history_seed;
mod effects;
mod external_editor;
mod host;
pub(crate) mod keybindings;
mod layout;
pub(crate) mod session_resume;
pub(crate) mod terminal_guard;
mod themes;
mod widgets;

#[cfg(test)]
mod harness;
#[cfg(test)]
mod tests;

use std::time::Duration;

use crossterm::event::EventStream as CrosstermEventStream;
use crossterm::event::{Event, KeyEventKind};
use futures::StreamExt;
use xylitol_tui::{CrosstermTerminal, InputEvent};

use crate::app::core::driver::{EventStream as AgentEventStream, XyDriver, XyDriverError};

use self::effects::{drain_pending, run_interactive_bang};
use self::host::{HostEvent, HostSession};
use self::terminal_guard::{TerminalGuard, exit_requested, install_lifecycle_hooks};

pub use self::bridge::{QueueBadge, UiEntry, UiModel, UiPhase, apply_xy_event};
pub use self::commands::{
    BangParse, PendingBash, PendingSlash as TuiPendingSlash, bash_result_entries,
    parse_bang_command,
};
pub use self::host::{
    HostEvent as TuiHostEvent, HostSession as TuiHostSession, LayoutMode, MIN_COLS, MIN_ROWS,
    TOO_SMALL_HINT, display_cwd, is_too_small,
};
pub use self::layout::{EditorSlot, LayoutTheme};
pub use self::widgets::GlyphSet;
// TuiRunOptions exported via struct above in this module

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
                "TUI requires a selected model; configure `~/.config/xylitol/config.yaml` (or project `.xylitol/config.yaml`) or pass `--model`"
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
/// - `tui` verb / default TTY entry forced on a pipe without a PTY
/// - (optional) unreachable `base_url` probe — keep off critical path unless requested
pub fn preflight(driver: &dyn XyDriver) -> Result<(), TuiPreflightError> {
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

/// Options for [`run`] (c1560).
#[derive(Debug, Clone)]
pub struct TuiRunOptions {
    /// `tui.editor_history_seed_sessions` (default 1).
    pub editor_history_seed_sessions: u32,
    /// True when CLI restored an existing `--session` id.
    pub restored_session: bool,
}

impl Default for TuiRunOptions {
    fn default() -> Self {
        Self {
            editor_history_seed_sessions: 1,
            restored_session: false,
        }
    }
}

/// Enter the interactive TUI REPL (host-driven; never calls `TUI::start()`).
///
/// Callers MUST run [`preflight`] first (CLI does). This still fails closed if
/// `TerminalGuard::enter` cannot start the terminal.
pub async fn run(driver: &mut dyn XyDriver, options: TuiRunOptions) -> Result<(), XyDriverError> {
    install_lifecycle_hooks();
    log::info!(target: "xylitol::tui", "starting product TUI host");

    let guard = TerminalGuard::enter()?;
    let terminal = guard.take();

    let result = run_host_loop(terminal, driver, options).await;

    if let Err(ref e) = result {
        log::error!(
            target: "xylitol::tui",
            "TUI host exited with error error.kind={} detail.kind={} error={}",
            e.kind(),
            e.detail_kind(),
            e
        );
        terminal_guard::emergency_restore();
    }
    result
}

async fn run_host_loop(
    terminal: CrosstermTerminal,
    driver: &mut dyn XyDriver,
    options: TuiRunOptions,
) -> Result<(), XyDriverError> {
    let model = driver
        .current_model()
        .map(|m| {
            if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            }
        })
        .unwrap_or_else(|| crate::app::core::bootstrap::UNSET_MODEL_DISPLAY.into());
    let mut session = HostSession::new_product_ui_with_meta(terminal, host::display_cwd(), model);
    session.set_editor_history_seed_sessions(options.editor_history_seed_sessions);
    session.apply_thinking_level_ui(driver.thinking_level());
    session.set_model_arg_catalog_from_models(&driver.available_models());
    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
    session.refresh_loaded_resources(driver).await;
    if options.restored_session {
        match driver.get_messages().await {
            Ok(entries) => {
                if let Some(sid) = driver.session_id() {
                    session.apply_cli_restored_session(&sid, entries);
                } else {
                    session.seed_editor_history_from_entries(&entries);
                }
            }
            Err(e) => {
                log::debug!(target: "xylitol::tui", "CLI session restore UI failed: {e}");
            }
        }
    } else {
        session.seed_editor_history_for_new_session(driver).await;
    }
    session.render_now()?;

    let mut term_events = CrosstermEventStream::new();
    let mut agent_stream: Option<AgentEventStream> = None;
    // Busy / spinner: ~60Hz. Idle: slow wake so debug builds do not burn a core
    // on empty Tick+try_render while the TTY is quiet.
    const TICK_BUSY_MS: u64 = 16;
    const TICK_IDLE_MS: u64 = 250;
    let mut tick_busy = session.is_busy();
    let mut ticker = tokio::time::interval(Duration::from_millis(if tick_busy {
        TICK_BUSY_MS
    } else {
        TICK_IDLE_MS
    }));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Always restore the TTY (even on RenderError / other Err) so a failed
    // host exit does not leave raw mode / keyboard protocol stuck.
    let host_result = async {
        while !session.should_quit() && !exit_requested() {
            drain_pending(&mut session, driver, &mut agent_stream).await?;

            let want_busy_tick = session.is_busy();
            if want_busy_tick != tick_busy {
                tick_busy = want_busy_tick;
                let ms = if tick_busy {
                    TICK_BUSY_MS
                } else {
                    TICK_IDLE_MS
                };
                ticker = tokio::time::interval(Duration::from_millis(ms));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                // First tick() completes immediately — skip so we do not spin a frame.
                ticker.tick().await;
            }

            if let Some(bash) = session.take_bash() {
                // Guard: never start a second interactive bang while one is active.
                if session.bash_active() {
                    session.push_system_note(
                        "bash already running — wait or Esc to cancel (second ! rejected)",
                    );
                } else {
                    // Shared bang loop (c715/c725): same crossterm→HostEvent map as main arms.
                    let input = term_events
                        .by_ref()
                        .filter_map(|maybe| futures::future::ready(map_crossterm_item(maybe)));
                    run_interactive_bang(&mut session, driver, bash, &mut agent_stream, input)
                        .await?;
                    continue;
                }
            }

            tokio::select! {
                _ = ticker.tick() => {
                    // Drain any footer estimates that completed without waiting on select.
                    while let Some((job_id, label)) = session.try_recv_footer_token() {
                        session.step(HostEvent::FooterTokens { job_id, label })?;
                    }
                    session.step(HostEvent::Tick)?;
                }
                maybe = term_events.next() => {
                    match maybe {
                        Some(item) => {
                            if let Some(ev) = map_crossterm_item(item) {
                                match ev {
                                    Ok(host_ev) => session.step(host_ev)?,
                                    Err(e) => {
                                        e.log_failure("tui.term_input");
                                        return Err(e);
                                    }
                                }
                            }
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
                    on_agent_stream_item(&mut session, &mut agent_stream, maybe_agent)?;
                }
                maybe_footer = session.recv_footer_token() => {
                    if let Some((job_id, label)) = maybe_footer {
                        session.step(HostEvent::FooterTokens { job_id, label })?;
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    session.tui.finish_inline();
    log::info!(target: "xylitol::tui", "product TUI host stopped");
    host_result
}

/// Shared crossterm → HostEvent map for main select and bang input stream (c725 / 2A).
fn map_crossterm_item(
    item: Result<Event, std::io::Error>,
) -> Option<Result<HostEvent, XyDriverError>> {
    match item {
        Ok(Event::Key(key)) => {
            if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                None
            } else {
                Some(Ok(HostEvent::Input(InputEvent::Key(key))))
            }
        }
        Ok(Event::Paste(data)) => Some(Ok(HostEvent::Input(InputEvent::Paste(data)))),
        Ok(Event::Resize(cols, rows)) => Some(Ok(HostEvent::Resize { cols, rows })),
        Ok(_) => None,
        Err(e) => Some(Err(XyDriverError::io(format!("input error: {e}")))),
    }
}

fn on_agent_stream_item<T: xylitol_tui::Terminal>(
    session: &mut HostSession<T>,
    agent_stream: &mut Option<AgentEventStream>,
    maybe: Option<crate::protocol::lifecycle::XyEvent>,
) -> Result<(), XyDriverError> {
    match maybe {
        Some(xy) => session.step(HostEvent::Xy(Box::new(xy))),
        None => {
            log::debug!(target: "xylitol::tui", "agent EventStream ended");
            *agent_stream = None;
            session.on_run_stream_closed();
            let _ = session.tui.try_render();
            Ok(())
        }
    }
}
