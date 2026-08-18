//! Terminal UI application surface — host-driven on `xylitol_tui`.
//!
//! See `AGENTS.md` in this directory. Engine: `packages/xylitol-tui`.

pub(crate) mod activity_fold;
mod ask_host;
mod bridge;
mod commands;
mod editor_history_seed;
mod effects;
pub(crate) mod error;
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
mod lab_ao_perf;
#[cfg(test)]
mod tests;

use std::time::Duration;

use crossterm::event::EventStream as CrosstermEventStream;
use crossterm::event::{Event, KeyEventKind, MouseEventKind};
use futures::FutureExt;
use futures::StreamExt;
use xylitol_tui::{CrosstermTerminal, InputEvent, Terminal};

use crate::app::core::driver::{EventStream as AgentEventStream, XyDriver, XyDriverError};

use self::effects::{drain_pending, run_interactive_bang, run_interactive_reload};
use self::host::{HostEvent, HostSession};
use self::terminal_guard::{TerminalGuard, exit_requested, install_lifecycle_hooks};

pub use self::ask_host::{AskHostGateway, ask_questions_to_choice};
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

/// Options for [`run`] (c1560 / c2070).
#[derive(Clone)]
pub struct TuiRunOptions {
    /// `tui.editor_history_seed_sessions` (default 1).
    pub editor_history_seed_sessions: u32,
    /// `tui.activity_fold` (c1761).
    pub activity_fold: crate::app::tui::activity_fold::ActivityFoldSettings,
    /// True when CLI restored an existing `--session` id.
    pub restored_session: bool,
    /// Process-local ask gateway (TUI-only); host polls for Choice mounts (c1850).
    pub ask_gateway: Option<std::sync::Arc<AskHostGateway>>,
    /// Interaction mode bound at host start (c2070 / ath30). Default ApplicationOwned.
    /// Not driven by `XYLITOL_TUI_MOUSE`. Mid-session switching is not supported —
    /// rebuild the host (or exit the process) to change modes.
    pub interaction_mode: xylitol_tui::InteractionMode,
}

impl Default for TuiRunOptions {
    fn default() -> Self {
        Self {
            editor_history_seed_sessions: 1,
            activity_fold: crate::app::tui::activity_fold::ActivityFoldSettings::default(),
            restored_session: false,
            ask_gateway: None,
            interaction_mode: xylitol_tui::InteractionMode::ApplicationOwned,
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
    let mut session = HostSession::new_product_ui_with_meta_mode(
        terminal,
        host::display_cwd(),
        model,
        options.interaction_mode,
    );
    session.set_editor_history_seed_sessions(options.editor_history_seed_sessions);
    session.set_activity_fold_settings(options.activity_fold);
    if let Some(gw) = options.ask_gateway {
        session.set_ask_gateway(gw);
    }
    session.apply_thinking_level_ui(driver.thinking_level());
    session.set_model_arg_catalog_from_models(&driver.available_models());
    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
    session.set_mcp_blocks_agent(driver.mcp_blocks_agent());
    // First paint before CLI restore / loaded-resources / ↑/↓ history seed so
    // welcome chrome is not blocked by JSONL load or list_sessions work.
    session.render_now()?;

    type CliRestoreHandle = tokio::task::JoinHandle<
        Result<Vec<crate::protocol::session::SessionEntry>, crate::protocol::error::XyStoreError>,
    >;
    let mut cli_restore: Option<(std::time::Instant, String, CliRestoreHandle)> = None;
    if options.restored_session {
        match (driver.session_id(), driver.session_store()) {
            (Some(sid), Some(store)) => {
                cli_restore = Some((
                    std::time::Instant::now(),
                    sid.clone(),
                    editor_history_seed::spawn_cli_session_load(store, sid),
                ));
            }
            _ => match driver.get_messages().await {
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
            },
        }
    }

    let mut editor_seed: Option<(std::time::Instant, tokio::task::JoinHandle<Vec<String>>)> = None;
    if !options.restored_session {
        if session.kick_editor_history_seed_async(driver) {
            editor_seed = session.take_editor_history_seed_job();
        } else {
            // Scripted / remote: no cloneable store — keep blocking seed.
            session.seed_editor_history_for_new_session(driver).await;
        }
    }
    // Refresh while editor-history seed / CLI restore run in the background.
    session.refresh_loaded_resources(driver).await;
    session.set_mcp_blocks_agent(driver.mcp_blocks_agent());

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
            let t_drain = std::time::Instant::now();
            drain_pending(&mut session, driver, &mut agent_stream).await?;
            crate::app::core::lag::note("host_drain_pending", t_drain);
            // `/session-new` (and similar) may arm a background seed during drain.
            if editor_seed.is_none()
                && let Some(job) = session.take_editor_history_seed_job()
            {
                editor_seed = Some(job);
            }

            let want_busy_tick = session.wants_busy_tick();
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

            if session.take_reload() {
                if session.reload_active() {
                    session.push_chrome_toast(self::commands::RELOADING_WAIT_NOTICE);
                } else {
                    let input = term_events
                        .by_ref()
                        .filter_map(|maybe| futures::future::ready(map_crossterm_item(maybe)));
                    run_interactive_reload(&mut session, driver, input).await?;
                    continue;
                }
            }

            if let Some(bash) = session.take_bash() {
                // Guard: never start a second interactive bang while one is active.
                if session.bash_active() {
                    session.push_scroll_notice(
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

            let ao_wheel_render_deadline = session.tui.application_owned_wheel_render_deadline();
            tokio::select! {
                _ = wait_for_ao_wheel_deadline(ao_wheel_render_deadline) => {
                    session.step_paint_only()?;
                }
                _ = ticker.tick() => {
                    // Drain any footer estimates that completed without waiting on select.
                    while let Some((job_id, label)) = session.try_recv_footer_token() {
                        session.step(HostEvent::FooterTokens { job_id, label })?;
                    }
                    if driver.poll_mcp_bootstrap().await {
                        let t0 = std::time::Instant::now();
                        session.refresh_loaded_resources(driver).await;
                        session.set_mcp_blocks_agent(driver.mcp_blocks_agent());
                        crate::app::core::lag::note("host_mcp_poll_refresh", t0);
                    }
                    let t_tick = std::time::Instant::now();
                    session.step(HostEvent::Tick)?;
                    crate::app::core::lag::note("host_tick", t_tick);
                }
                maybe = term_events.next() => {
                    match maybe {
                        Some(item) => {
                            // Fast scroll: sum already-buffered wheel events and
                            // apply the full delta before one cheap AO reproject.
                            // Only when the first event is over the transcript pane.
                            let step = session.tui.application_owned_wheel_notch();
                            let coalesce_wheel = wheel_delta_from_item(&item, step).filter(|_| {
                                session.tui.application_session_active()
                                    && match &item {
                                        Ok(Event::Mouse(m)) => {
                                            let dock = session.tui.dock_rows() as u16;
                                            let th = Terminal::rows(&session.tui.terminal)
                                                .saturating_sub(dock);
                                            m.row < th
                                        }
                                        _ => false,
                                    }
                            });
                            if let Some(delta0) = coalesce_wheel {
                                let mut delta = delta0;
                                let mut deferred: Option<Result<Event, std::io::Error>> = None;
                                while let Some(next) = term_events.next().now_or_never().flatten()
                                {
                                    if let Some(d) = wheel_delta_from_item(&next, step) {
                                        delta = delta.saturating_add(d);
                                    } else {
                                        deferred = Some(next);
                                        break;
                                    }
                                }
                                session.apply_ao_wheel_delta(delta)?;
                                if let Some(next) = deferred
                                    && let Some(ev) = map_crossterm_item(next)
                                {
                                    match ev {
                                        Ok(host_ev) => session.step(host_ev)?,
                                        Err(e) => {
                                            e.log_failure("tui.term_input");
                                            return Err(e);
                                        }
                                    }
                                }
                            } else if let Some(ev) = map_crossterm_item(item) {
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
                maybe_seed = async {
                    match editor_seed.as_mut() {
                        Some((_, handle)) => handle.await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Some((started, _)) = editor_seed.take() {
                        let texts = maybe_seed.unwrap_or_default();
                        session.seed_editor_history_from_texts(texts);
                        crate::app::core::lag::note("tui_seed_editor_history", started);
                    }
                }
                maybe_restore = async {
                    match cli_restore.as_mut() {
                        Some((_, _, handle)) => handle.await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Some((started, sid, _)) = cli_restore.take() {
                        match maybe_restore {
                            Ok(Ok(entries)) => {
                                session.apply_cli_restored_session(&sid, entries);
                                let _ = session.render_now();
                            }
                            Ok(Err(e)) => {
                                log::debug!(
                                    target: "xylitol::tui",
                                    "CLI session restore UI failed: {e}"
                                );
                            }
                            Err(e) => {
                                log::debug!(
                                    target: "xylitol::tui",
                                    "CLI session restore join failed: {e}"
                                );
                            }
                        }
                        crate::app::core::lag::note("tui_cli_restore", started);
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    session.tui.finish();
    log::info!(target: "xylitol::tui", "product TUI host stopped");
    host_result
}

async fn wait_for_ao_wheel_deadline(deadline: Option<std::time::Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline.into()).await,
        None => std::future::pending().await,
    }
}

fn wheel_delta_from_item(item: &Result<Event, std::io::Error>, step: isize) -> Option<isize> {
    let step = step.max(1);
    match item {
        Ok(Event::Mouse(m)) => match m.kind {
            MouseEventKind::ScrollUp => Some(-step),
            MouseEventKind::ScrollDown => Some(step),
            _ => None,
        },
        _ => None,
    }
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
        Ok(Event::Mouse(mouse)) => {
            let event = InputEvent::Mouse(mouse);
            // Drop 1003 motion at the edge — never enter handle_input paint path.
            if event.is_pointer_motion() {
                None
            } else {
                Some(Ok(HostEvent::Input(event)))
            }
        }
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
        Some(xy) if is_coalesceable_stream_delta(&xy) => {
            // Typewriter bursts: merge already-buffered Text/Thinking deltas into
            // one model sync + one throttled paint (cuts AO full-render spam).
            apply_xy_event(session.ui_model_mut(), &xy);
            let mut deferred: Option<crate::protocol::lifecycle::XyEvent> = None;
            let mut stream_ended = false;
            while let Some(stream) = agent_stream.as_mut() {
                let Some(next) = stream.next().now_or_never() else {
                    break;
                };
                match next {
                    Some(ev) if is_coalesceable_stream_delta(&ev) => {
                        apply_xy_event(session.ui_model_mut(), &ev);
                    }
                    Some(ev) => {
                        deferred = Some(ev);
                        break;
                    }
                    None => {
                        stream_ended = true;
                        break;
                    }
                }
            }
            session.sync_ui_root_from_model();
            // Busy ticker paints ≤60Hz — do not paint on every token wake.
            session.tui.request_render(false);
            if stream_ended {
                log::debug!(target: "xylitol::tui", "agent EventStream ended");
                *agent_stream = None;
                session.on_run_stream_closed();
                let _ = session.tui.try_render();
            }
            if let Some(ev) = deferred {
                session.step(HostEvent::Xy(Box::new(ev)))?;
            }
            Ok(())
        }
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

fn is_coalesceable_stream_delta(xy: &crate::protocol::lifecycle::XyEvent) -> bool {
    matches!(
        xy,
        crate::protocol::lifecycle::XyEvent::TextDelta(_)
            | crate::protocol::lifecycle::XyEvent::ThinkingDelta(_)
    )
}
