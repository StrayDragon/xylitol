//! Shared pending side-effect pump for production host loop and harness (ath6 / c494).

mod bang;
mod debug_activity_fold;
mod debug_verify;
mod helpers;
mod pending_ui;
mod reload;
mod slash;

use xylitol_tui::Terminal;

use crate::app::core::driver::{EventStream, QueueStats, XyDriver, XyDriverError};

use crate::app::core::dispatch::dispatch;
use crate::protocol::Command;

use crate::app::tui::commands::slash_allowances;
use crate::protocol::wire::registry::Exec;

use super::host::HostSession;
use super::widgets::footer_token_label;

pub use bang::run_interactive_bang;
pub use reload::run_interactive_reload;

/// c2790 / ath45: bang-loop tick-arm pump — execute ONLY pending slashes whose
/// execution class is `Inline` (atm18: cache/local effects, never a remote
/// unary). Exclusive/Queued are put back verbatim for the main-loop
/// `drain_pending` (semantics unchanged). Model-picker confirm deliberately
/// stays out: SetModel is a real unary (writer lease) on remote.
pub async fn drain_inline_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if let Some(slash) = session.take_slash() {
        if slash_allowances(&slash).exec == Exec::Inline {
            slash::handle_slash(session, driver, slash).await;
        } else {
            session.put_slash(slash);
        }
    }
}

/// Load current-session entries via the shared Command path (c2710).
pub async fn session_entries(
    driver: &mut dyn XyDriver,
) -> Result<Vec<crate::protocol::session::SessionEntry>, XyDriverError> {
    match crate::app::core::dispatch::dispatch(driver, crate::protocol::Command::GetMessages {})
        .await?
    {
        crate::app::core::dispatch::DispatchOutcome::Messages { entries, .. } => Ok(entries),
        _ => Ok(Vec::new()),
    }
}

/// Read queue depths via the shared Command path (c2710).
pub async fn queue_stats(driver: &mut dyn XyDriver) -> QueueStats {
    match crate::app::core::dispatch::dispatch(driver, crate::protocol::Command::GetQueueStats {})
        .await
    {
        Ok(crate::app::core::dispatch::DispatchOutcome::QueueStats {
            steer_count,
            follow_up_count,
        }) => QueueStats {
            steer_count,
            follow_up_count,
        },
        _ => QueueStats::default(),
    }
}

/// Refresh footer token usage from [`XyDriver::estimate_context_tokens`] (c1035).
///
/// Empty session → omit field (MUST NOT forge `used 0`). Estimate errors → omit.
///
/// **Harness / tests**: awaits estimate (override is instant). Production host
/// MUST prefer `kick_footer_token_refresh` so HF encode does not block input.
pub async fn refresh_footer_tokens<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    let _ = session.take_pending_footer_token_refresh();
    let entries = match session_entries(driver).await {
        Ok(e) => e,
        Err(_) => {
            session.set_footer_token_label(None);
            return;
        }
    };
    if entries.is_empty() {
        session.set_footer_token_label(None);
        return;
    }
    match driver.estimate_context_tokens().await {
        Ok(est) => {
            let window = driver
                .current_model()
                .map(|m| m.context_window)
                .unwrap_or(0);
            let label = footer_token_label(est.provenance, est.tokens, window);
            session.set_footer_token_label(Some(label));
        }
        Err(_) => session.set_footer_token_label(None),
    }
}

/// Start a background footer estimate; return the label via the footer job tx.
///
/// Rides the [`XyDriver::estimate_context_tokens`] seam so in-process and remote
/// surfaces share the host-side overhead-aware estimate (c25 / c16) instead of a
/// local entries-only count.
#[cfg_attr(test, allow(dead_code))] // production `drain_pending` only (`not(test)`)
pub async fn kick_footer_token_refresh<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    let _ = session.take_pending_footer_token_refresh();
    let job_id = session.begin_footer_token_job();
    let tx = session.footer_token_tx();
    let context_window = driver
        .current_model()
        .map(|m| m.context_window)
        .unwrap_or(0);

    let label = match driver.estimate_context_tokens().await {
        Ok(est) => Some(footer_token_label(
            est.provenance,
            est.tokens,
            context_window,
        )),
        Err(_) => None,
    };
    let _ = tx.send((job_id, label));
}

/// Abort bookkeeping + steer / follow-up lane pump (ati3 / c665 / busy→idle).
///
/// Lanes dispatch into the driver queues only while a worker turn / bang /
/// Assembling gate is live. Lanes queued while idle (bang return, busy→idle
/// transition) would never be consumed — the ReAct worker drains queues only
/// mid-turn and at end-of-turn — so the first lane converts to a root submit
/// (returned to the caller). Abort keeps the 插话续跑与中止 contract: 插话
/// 丢弃、续跑留守队列条、不自动跑（bang Esc cancel shares this via the
/// `bash_cancelled` latch, additionally dropping a locally queued steer since
/// the bang loop already cleared driver-side steers).
///
/// Returns the lane converted to a root submit, if any.
async fn drain_abort_and_lanes<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &Option<EventStream>,
) -> Option<String> {
    let aborted_now = session.take_abort();
    if aborted_now {
        log::info!(target: "xylitol::tui", "XyDriver::abort (Esc)");
        driver.abort();
        // c1900: Assembling uses a host-local gated_submit + follow-up strip that is
        // NOT on the driver queue. Drop it before abort note so strip sync / idle
        // check see real driver depths (otherwise Esc looks aborted then still runs).
        let cancelled_gate = session.take_gated_submit().is_some();
        if cancelled_gate {
            let stats = queue_stats(driver).await;
            session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        }
        session.note_user_abort();
        let _ = dispatch(
            driver,
            Command::ClearQueue {
                clear_steer: true,
                clear_follow_up: false,
            },
        )
        .await;
        let stats = queue_stats(driver).await;
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    let bang_cancelled = session.take_bash_cancelled();
    if bang_cancelled && let Some(msg) = session.take_steer() {
        session.ui_model_mut().pop_steer_strip_matching(&msg);
        log::info!(
            target: "xylitol::tui",
            "bang cancel drops local steer prompt_len={}",
            msg.len()
        );
    }
    let idle_no_turn = agent_stream.is_none()
        && !session.run_active()
        && !session.bash_active()
        && !aborted_now
        && !bang_cancelled
        && !session.has_gated_submit();
    if idle_no_turn && let Some(msg) = session.take_steer().or_else(|| session.take_follow_up()) {
        session.ui_model_mut().pop_follow_up_strip_matching(&msg);
        session.ui_model_mut().pop_steer_strip_matching(&msg);
        log::info!(
            target: "xylitol::tui",
            "idle queued lane → root submit prompt_len={}",
            msg.len()
        );
        return Some(msg);
    }
    if !idle_no_turn && let Some(msg) = session.take_steer() {
        log::info!(target: "xylitol::tui", "Command::Steer prompt_len={}", msg.len());
        if let Err(e) = dispatch(
            driver,
            Command::Steer {
                message: msg.clone(),
            },
        )
        .await
        {
            e.log_failure("tui.steer");
            session.push_scroll_notice(format!("steer failed: {e}"));
        }
        calibrate_queue_after_local_enqueue(session, queue_stats(driver).await);
        let _ = session.render_now();
    }
    if !idle_no_turn && let Some(msg) = session.take_follow_up() {
        log::info!(target: "xylitol::tui", "Command::FollowUp prompt_len={}", msg.len());
        if let Err(e) = dispatch(
            driver,
            Command::FollowUp {
                message: msg.clone(),
            },
        )
        .await
        {
            e.log_failure("tui.follow_up");
            session.push_scroll_notice(format!("follow-up failed: {e}"));
        }
        calibrate_queue_after_local_enqueue(session, queue_stats(driver).await);
        let _ = session.render_now();
    }
    None
}

/// Consume HostSession pending ops and call XyDriver / dispatch.
///
/// Ordering matches the historical `run_host_loop` body (abort → dequeue → steer →
/// follow-up → slash → optional submit→`XyDriver::run`). Bang is **not** awaited here
/// (c665 — host `select!` / `run_pending_bash`). Production and harness MUST share
/// this entry so slash/steer branches cannot diverge.
///
/// When `agent_stream` is already `Some`, submit is not taken. A newly started
/// run is stored in `agent_stream`; callers decide whether to drain it (harness)
/// or poll it in a select loop (production).
///
/// Footer token refresh is **kicked** async (does not await HF encode).
pub async fn drain_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), XyDriverError> {
    drain_footer_token_if_pending(session, driver).await;
    let converted_submit = drain_abort_and_lanes(session, driver, agent_stream).await;
    if session.take_dequeue() {
        log::info!(target: "xylitol::tui", "XyDriver::clear_queue (Alt+Up dequeue)");
        let _ = dispatch(
            driver,
            Command::ClearQueue {
                clear_steer: true,
                clear_follow_up: true,
            },
        )
        .await;
        // Alt+Up already restored the gate strip into the editor; cancel Assembling
        // so a later freeze MUST NOT start the withdrawn prompt.
        if session.take_gated_submit().is_some() {
            session.end_gated_assemble_idle();
        }
        let stats = queue_stats(driver).await;
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }

    if let Some(slash) = session.take_slash() {
        slash::handle_slash(session, driver, slash).await;
    }

    // Bang (`!`/`!!`) is NOT awaited here — host loop / pump runs it so Esc can
    // abort concurrently (c665). Callers MUST `take_bash` after drain_pending.
    // `/reload` likewise: callers MUST `take_reload` → `run_interactive_reload` (c1205).

    pending_ui::drain_pending_ui(session, driver).await;
    // Session switch/resume/import apply mid-drain and arm footer refresh; pick it
    // up in the same cycle (flag is only checked once at the top otherwise).
    drain_footer_token_if_pending(session, driver).await;

    if let Some(notice) = driver.take_mcp_gate_notice() {
        session.push_scroll_notice(notice);
    }

    if agent_stream.is_none()
        && driver.is_tools_frozen()
        && let Some(prompt) = session.take_gated_submit()
    {
        // Freeze may have completed this tick; refresh snap so mcp pending cue clears
        // (stale tools_table_frozen:false would sticky-restore the cue on idle sync).
        session.refresh_loaded_resources(driver).await;
        start_run_after_tool_gate(session, driver, agent_stream, prompt).await;
    }

    if agent_stream.is_none()
        && let Some(prompt) = converted_submit.or_else(|| session.take_submit())
    {
        if !driver.is_tools_frozen() {
            log::info!(
                target: "xylitol::tui",
                "tool freeze gate armed prompt_len={}",
                prompt.len()
            );
            driver.arm_tool_freeze_gate().await;
            session.set_gated_submit(prompt.clone());
            session
                .ui_model_mut()
                .enqueue_follow_up_strip(prompt.clone());
            session.ui_model_mut().set_busy_status("Assembling");
            session.sync_ui_root_from_model();
            // Assembling (busy lead) MUST keep right-aligned MCP cue when still pending
            // (resume Settling/re-gate — see mcp_tools_pending).
            if let Some(root) = session.ui_root() {
                root.borrow_mut().refresh_mcp_short_cue();
            }
            let _ = session.render_now();
            if driver.is_tools_frozen()
                && let Some(prompt) = session.take_gated_submit()
            {
                session.refresh_loaded_resources(driver).await;
                start_run_after_tool_gate(session, driver, agent_stream, prompt).await;
            }
        } else {
            log::info!(target: "xylitol::tui", "XyDriver::run starting prompt_len={}", prompt.len());
            session.on_run_started(&prompt);
            let _ = session.render_now();
            let t0 = std::time::Instant::now();
            *agent_stream = Some(driver.run(&prompt).await);
            super::super::core::lag::note_detail(
                "host_run_await",
                t0,
                &format!("prompt_len={}", prompt.len()),
            );
        }
    }

    session.sync_fixed_zone(driver);
    let _ = session.render_now();

    Ok(())
}

/// Host depths calibrate the badge; never wipe local strip text with empty 0/0
/// before `QueueUpdate` confirms the host consumed the enqueue (`ati44`).
fn calibrate_queue_after_local_enqueue<T: Terminal>(
    session: &mut HostSession<T>,
    stats: QueueStats,
) {
    let local_steer = session.ui_model().pending_steer.len();
    let local_follow = session.ui_model().pending_follow_up.len();
    if stats.steer_count == 0 && stats.follow_up_count == 0 && (local_steer > 0 || local_follow > 0)
    {
        return;
    }
    session.set_queue_badge(stats.steer_count, stats.follow_up_count);
}

async fn start_run_after_tool_gate<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
    prompt: String,
) {
    session.ui_model_mut().pop_follow_up_strip_matching(&prompt);
    session.sync_ui_root_from_model();
    log::info!(
        target: "xylitol::tui",
        "XyDriver::run after tool freeze prompt_len={}",
        prompt.len()
    );
    session.on_run_started(&prompt);
    let _ = session.render_now();
    let t0 = std::time::Instant::now();
    *agent_stream = Some(driver.run(&prompt).await);
    super::super::core::lag::note_detail(
        "host_run_await",
        t0,
        &format!("prompt_len={}", prompt.len()),
    );
}

async fn drain_footer_token_if_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    // c1860: prefer settlement snapshot (no re-estimate / no OTel).
    if let Some(est) = session.take_pending_settlement_estimate() {
        let _ = session.take_pending_footer_token_refresh();
        let window = driver
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(0);
        let label = footer_token_label(est.provenance, est.tokens, window);
        session.set_footer_token_label(Some(label));
        let _ = session.render_now();
        return;
    }
    if !session.take_pending_footer_token_refresh() {
        return;
    }
    // Tests: await so ScriptedDriver estimate_override still applies.
    // Production: kick background job — never block input on HF encode.
    #[cfg(test)]
    {
        refresh_footer_tokens(session, driver).await;
        let _ = session.render_now();
    }
    #[cfg(not(test))]
    {
        kick_footer_token_refresh(session, driver).await;
    }
}
