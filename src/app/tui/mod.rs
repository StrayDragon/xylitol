//! Terminal UI application surface — host-driven on `xylitol_tui`.
//!
//! See `AGENTS.md` in this directory. Engine: `packages/xylitol-tui`.

pub(crate) mod activity_fold;
/// P2 无头产品帧挂载面（package-tui-testing 契约）：
/// SceneBuilder 流式脚本事件 → 真实 UiRoot 渲染 → 剥离 ANSI 的纯文本帧 + 语义 dump。
pub use activity_fold::scene::{SceneBuilder, SemanticDump};
mod ask_host;
mod bridge;
mod commands;
mod editor_history_seed;
mod effects;
pub(crate) mod error;
mod external_editor;
mod host;
/// P3 无头键鼠交互挂载面：和弦路由 / 折叠命中 / per-id 覆盖表的产品路径夹具。
mod interaction_scene;
pub use interaction_scene::InteractionBdd;
pub(crate) mod keybindings;
mod layout;
pub(crate) mod session_resume;
pub(crate) mod terminal_guard;
mod themes;
mod widgets;

/// ati30：BDD/单测共用的 ScriptedDriver + TestTerminal + host 泵。
/// 正常编译路径（SceneBuilder tt08 先例）——验证与生产共用唯一副作用泵。
pub mod harness;
#[cfg(test)]
mod lab_ao_perf;
#[cfg(test)]
mod lab_design_frame;
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
pub use self::bridge::{BashBlockStatus, QueueBadge, UiEntry, UiModel, UiPhase, apply_xy_event};
/// att12/att18：travel 重建 seam 窄导出（BDD 与外部消费者同一管道）。
pub use self::bridge::{rebuild_scrollback_from_travel, travel_history_note};
pub use self::commands::{
    BangParse, PendingBash, PendingSlash as TuiPendingSlash, bash_result_entries,
    parse_bang_command,
};
pub use self::host::{
    HostEvent as TuiHostEvent, HostSession as TuiHostSession, LayoutMode, MIN_COLS, MIN_ROWS,
    TOO_SMALL_HINT, display_cwd, is_too_small,
};
pub use self::layout::{EditorSlot, EditorSlotKind, LayoutTheme};
pub use self::widgets::{FoldHitTable, FoldTarget, GlyphSet, ScrollbackFold};
/// att26：ActivityFold 自动收纳旋钮（信封/簇折叠配置面）。
pub use activity_fold::ActivityFoldSettings;
/// att13：折叠态工具人话摘要的窄导出（BDD 直驱纯函数合约）。
pub use bridge::human_tool_args_preview;
// TuiRunOptions exported via struct above in this module

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
    ///
    /// Lab peek: CLI may pass `lab_interaction_mode_from_env` (`XYLITOL_TUI_INLINE`).
    /// That is **not** a product flag or setting.
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

/// Lab peek env for constructing the product host as Inline (`1` / `true` / `yes`).
///
/// **Not** a product flag. Default / unset remains ApplicationOwned (ath30).
pub(crate) const LAB_TUI_INLINE_ENV: &str = "XYLITOL_TUI_INLINE";

pub(crate) fn parse_lab_inline_opt_in(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
    })
}

pub(crate) fn lab_interaction_mode(inline_env: Option<&str>) -> xylitol_tui::InteractionMode {
    if parse_lab_inline_opt_in(inline_env) {
        xylitol_tui::InteractionMode::Inline
    } else {
        xylitol_tui::InteractionMode::ApplicationOwned
    }
}

pub(crate) fn lab_interaction_mode_from_env() -> xylitol_tui::InteractionMode {
    lab_interaction_mode(std::env::var(LAB_TUI_INLINE_ENV).ok().as_deref())
}

/// Enter the interactive TUI REPL (host-driven; never calls `TUI::start()`).
///
/// Callers MUST check TTY themselves before entering (CLI does). This still
/// fails closed if `TerminalGuard::enter` cannot start the terminal.
pub async fn run(driver: &mut dyn XyDriver, options: TuiRunOptions) -> Result<(), XyDriverError> {
    install_lifecycle_hooks();
    log::info!(target: "xylitol::tui", "starting product TUI host");
    if options.interaction_mode.is_inline() {
        log::info!(
            target: "xylitol::tui",
            "lab {LAB_TUI_INLINE_ENV}: product host using Inline (not a product setting)"
        );
    }

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

type CliRestoreHandle = tokio::task::JoinHandle<
    Result<
        Vec<crate::protocol::session::SessionEntry>,
        crate::protocol::error::XySessionStoreError,
    >,
>;

/// 冷恢复（`--session` 延续）：有 cloneable store 时后台 spawn JSONL 装载；
/// 否则阻塞装载并直接投喂 UI。返回后台任务句柄供主循环收割。
async fn apply_cli_restore(
    session: &mut HostSession<CrosstermTerminal>,
    driver: &mut dyn XyDriver,
) -> Option<(std::time::Instant, String, CliRestoreHandle)> {
    match (driver.session_id(), driver.session_store()) {
        (Some(sid), Some(store)) => Some((
            std::time::Instant::now(),
            sid.clone(),
            editor_history_seed::spawn_cli_session_load(store, sid),
        )),
        _ => match driver.get_messages().await {
            Ok(entries) => {
                if let Some(sid) = driver.session_id() {
                    session.apply_cli_restored_session(&sid, entries);
                } else {
                    session.seed_editor_history_from_entries(&entries);
                }
                None
            }
            Err(e) => {
                log::debug!(target: "xylitol::tui", "CLI session restore UI failed: {e}");
                None
            }
        },
    }
}

/// 新会话的编辑器历史种子：可异步则后台 spawn（返回句柄），否则阻塞 seed
/// （Scripted / remote 无 cloneable store）。
async fn seed_editor_history(
    session: &mut HostSession<CrosstermTerminal>,
    driver: &mut dyn XyDriver,
) -> Option<(std::time::Instant, tokio::task::JoinHandle<Vec<String>>)> {
    if session.kick_editor_history_seed_async(driver) {
        session.take_editor_history_seed_job()
    } else {
        session.seed_editor_history_for_new_session(driver).await;
        None
    }
}

async fn run_host_loop(
    terminal: CrosstermTerminal,
    driver: &mut dyn XyDriver,
    options: TuiRunOptions,
) -> Result<(), XyDriverError> {
    driver.attach_session().await?;
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
    // welcome fixed-zone paint is not blocked by JSONL load or list_sessions work.
    session.render_now()?;

    let mut cli_restore = if options.restored_session {
        apply_cli_restore(&mut session, driver).await
    } else {
        None
    };

    let mut editor_seed = if options.restored_session {
        None
    } else {
        seed_editor_history(&mut session, driver).await
    };
    // Refresh while editor-history seed / CLI restore run in the background.
    session.refresh_loaded_resources(driver).await;
    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
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
    // ath42/c2480: connection grace state machine — transient link churn
    // stays invisible; a prolonged outage/recovery surfaces as toast notice.
    let mut link_grace = host::LinkGrace::new();

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
                    session.push_toast_notice(self::commands::RELOADING_WAIT_NOTICE);
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
                    if agent_stream.is_none() {
                        apply_idle_downlink(&mut session, driver)?;
                    }
                    // ath42: link grace UX — announce only past the grace
                    // window, once, via toast notice (never transcript rows).
                    if let Some(notice) = link_grace.tick(
                        driver.link_health(),
                        std::time::Instant::now(),
                        host::LINK_GRACE_INITIAL,
                        host::LINK_GRACE_RECONNECT,
                    ) {
                        match notice {
                            host::LinkNotice::Disconnected => {
                                session.push_toast_notice(host::LINK_DOWN_NOTICE);
                            }
                            host::LinkNotice::Recovered => {
                                session.push_toast_notice(host::LINK_RECOVERED_NOTICE);
                            }
                        }
                    }
                    if driver.poll_mcp_bootstrap().await {
                        let t0 = std::time::Instant::now();
                        if let Some(snap) = driver.loaded_resources_cached() {
                            session.refresh_loaded_resources_from_snap(snap);
                        } else {
                            session.refresh_loaded_resources(driver).await;
                        }
                        session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
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

fn apply_idle_downlink<T: xylitol_tui::Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) -> Result<(), XyDriverError> {
    if driver.take_resync_rebuild() {
        session.ui_model_mut().entries.clear();
        session.ui_model_mut().streaming_assistant.clear();
        session.ui_model_mut().streaming_thinking.clear();
        session.sync_ui_root_from_model();
    }
    for ev in driver.drain_idle_events() {
        session.step(HostEvent::Xy(Box::new(ev)))?;
    }
    Ok(())
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
