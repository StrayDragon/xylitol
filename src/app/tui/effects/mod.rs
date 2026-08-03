//! Shared pending side-effect pump for production host loop and harness (ath6 / c494).

mod bang;
mod helpers;
mod pending_ui;
mod slash;

use xylitol_tui::Terminal;

use crate::app::core::driver::{
    EventStream, XyDriver, XyDriverError, estimate_from_session_entries,
    tokenizer_override_from_app_config,
};

use super::host::HostSession;
use super::widgets::footer_token_label;

pub use bang::run_interactive_bang;

/// Refresh footer token usage from [`XyDriver::estimate_context_tokens`] (c1035).
///
/// Empty session → omit field (MUST NOT forge `used 0`). Estimate errors → omit.
///
/// **Harness / tests**: awaits estimate (override is instant). Production host
/// MUST prefer [`kick_footer_token_refresh`] so HF encode does not block input.
pub async fn refresh_footer_tokens<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn XyDriver,
) {
    let _ = session.take_pending_footer_token_refresh();
    let entries = match driver.get_messages().await {
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

/// Start a background footer estimate; return immediately (production host).
///
/// Loads messages on the async path (cheap), then `spawn_blocking` for encode.
/// Result arrives via [`HostSession::recv_footer_token`] → `HostEvent::FooterTokens`.
#[cfg_attr(test, allow(dead_code))] // production `drain_pending` only (`not(test)`)
pub async fn kick_footer_token_refresh<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn XyDriver,
) {
    let _ = session.take_pending_footer_token_refresh();
    let entries = match driver.get_messages().await {
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

    let job_id = session.begin_footer_token_job();
    let tx = session.footer_token_tx();
    let model_id = driver.current_model().map(|m| m.id);
    let context_window = driver
        .current_model()
        .map(|m| m.context_window)
        .unwrap_or(0);
    let tokenizer_override = model_id
        .as_deref()
        .and_then(tokenizer_override_from_app_config);

    tokio::spawn(async move {
        let label = tokio::task::spawn_blocking(move || {
            let est = estimate_from_session_entries(&entries, model_id, tokenizer_override);
            footer_token_label(est.provenance, est.tokens, context_window)
        })
        .await
        .ok();
        let _ = tx.send((job_id, label));
    });
}

/// Consume HostSession pending ops and call XyDriver / dispatch.
///
/// Ordering matches the historical `run_host_loop` body (abort → dequeue → steer →
/// follow-up → slash → optional submit→`XyDriver::run`). Bang is **not** awaited here
/// (c665 — host `select!` / [`run_pending_bash`]). Production and harness MUST share
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
    if session.take_abort() {
        log::info!(target: "xylitol::tui", "XyDriver::abort (Esc)");
        driver.abort();
        session.note_user_abort();
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if session.take_dequeue() {
        log::info!(target: "xylitol::tui", "XyDriver::clear_queue (Alt+Up dequeue)");
        let _ = driver.clear_queue(true, true);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_steer() {
        log::info!(target: "xylitol::tui", "XyDriver::steer prompt_len={}", msg.len());
        if let Err(e) = driver.steer(&msg) {
            e.log_failure("tui.steer");
            session.push_scroll_notice(format!("steer failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_follow_up() {
        log::info!(target: "xylitol::tui", "XyDriver::follow_up prompt_len={}", msg.len());
        if let Err(e) = driver.follow_up(&msg) {
            e.log_failure("tui.follow_up");
            session.push_scroll_notice(format!("follow-up failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }

    if let Some(slash) = session.take_slash() {
        slash::handle_slash(session, driver, slash).await;
    }

    // Bang (`!`/`!!`) is NOT awaited here — host loop / pump runs it so Esc can
    // abort concurrently (c665). Callers MUST `take_bash` after drain_pending.

    pending_ui::drain_pending_ui(session, driver).await;
    // Session switch/resume/import apply mid-drain and arm footer refresh; pick it
    // up in the same cycle (flag is only checked once at the top otherwise).
    drain_footer_token_if_pending(session, driver).await;

    if agent_stream.is_none()
        && let Some(prompt) = session.take_submit()
    {
        log::info!(target: "xylitol::tui", "XyDriver::run starting prompt_len={}", prompt.len());
        session.on_run_started(&prompt);
        let _ = session.render_now();
        // Spinner freezes while this await holds the host select loop.
        let t0 = std::time::Instant::now();
        *agent_stream = Some(driver.run(&prompt).await);
        super::super::core::lag::note_detail(
            "host_run_await",
            t0,
            &format!("prompt_len={}", prompt.len()),
        );
    }

    session.sync_runtime_chrome(driver);
    let _ = session.render_now();

    Ok(())
}

async fn drain_footer_token_if_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn XyDriver,
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
