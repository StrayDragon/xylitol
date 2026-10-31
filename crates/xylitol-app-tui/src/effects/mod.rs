mod bang;
mod debug_activity_fold;
mod debug_verify;
pub(crate) mod designing;
mod helpers;
mod pending_ui;
mod reload;
mod slash;

use xylitol_tui::Terminal;

use crate::driver::{EventStream, QueueStats, XyDriver, XyDriverError};

use crate::dispatch::dispatch;
use xylitol_protocol::Command;

use crate::commands::slash_allowances;
use xylitol_protocol::registry::Exec;

use super::host::HostSession;
use super::widgets::footer_token_label;

pub use bang::run_interactive_bang;
pub use reload::{
    ClientReloadPhase, ClientReloadStep, default_client_reload_steps, run_interactive_reload,
};

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

pub async fn session_entries(
    driver: &mut dyn XyDriver,
) -> Result<Vec<xylitol_plugin_sdk::session::SessionEntry>, XyDriverError> {
    Ok(session_entries_page(driver, None, None).await?.0)
}

pub async fn session_entries_page(
    driver: &mut dyn XyDriver,
    limit: Option<usize>,
    before_entry_id: Option<String>,
) -> Result<(Vec<xylitol_plugin_sdk::session::SessionEntry>, bool), XyDriverError> {
    match crate::dispatch::dispatch(
        driver,
        xylitol_protocol::Command::GetMessages {
            limit,
            before_entry_id,
        },
    )
    .await?
    {
        crate::dispatch::DispatchOutcome::Messages {
            entries, has_more, ..
        } => Ok((entries, has_more)),
        _ => Ok((Vec::new(), false)),
    }
}

pub async fn queue_stats(driver: &mut dyn XyDriver) -> QueueStats {
    match crate::dispatch::dispatch(driver, xylitol_protocol::Command::GetQueueStats {}).await {
        Ok(crate::dispatch::DispatchOutcome::QueueStats {
            steer_count,
            follow_up_count,
        }) => QueueStats {
            steer_count,
            follow_up_count,
        },
        _ => QueueStats::default(),
    }
}

pub async fn refresh_footer_tokens<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    let _ = session.take_pending_footer_token_refresh();
    if !footer_entries_available(session, driver).await {
        return;
    }
    match driver.estimate_context_tokens().await {
        Ok(est) => {
            let window = driver
                .current_model()
                .map(|m| m.context_window)
                .unwrap_or(0);
            let label =
                footer_token_label(est.provenance, est.tokens, window, est.prompt_cache_read);
            session.set_footer_token_label(Some(label));
        }
        Err(_) => session.set_footer_token_label(None),
    }
}

#[cfg_attr(test, allow(dead_code))]
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

    let (label, heuristic) = match driver.estimate_context_tokens().await {
        Ok(est) => (
            Some(footer_token_label(
                est.provenance,
                est.tokens,
                context_window,
                est.prompt_cache_read,
            )),
            est.provenance == xylitol_plugin_sdk::model::TokenProvenance::Heuristic,
        ),
        Err(_) => (None, false),
    };
    let _ = tx.send((job_id, label, heuristic));
}

async fn drain_abort_and_lanes<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &Option<EventStream>,
) -> Option<String> {
    let aborted_now = session.take_abort();
    if aborted_now {
        log::info!(target: "xylitol::tui", "XyDriver::abort (Esc)");
        driver.abort();
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

pub async fn drain_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), XyDriverError> {
    session.sync_session_sidebar_reserve();
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

    pending_ui::drain_pending_ui(session, driver).await;
    drain_footer_token_if_pending(session, driver).await;

    if let Some(notice) = driver.take_mcp_gate_notice() {
        session.push_scroll_notice(notice);
    }

    if agent_stream.is_none()
        && driver.is_tools_frozen()
        && let Some(prompt) = session.take_gated_submit()
    {
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
            session.set_gated_submit(prompt.clone());
            session
                .ui_model_mut()
                .enqueue_follow_up_strip(prompt.clone());
            session.ui_model_mut().set_busy_status("Assembling");
            session.sync_ui_root_from_model();
            if let Some(root) = session.ui_root() {
                root.borrow_mut().refresh_mcp_short_cue();
            }
            let _ = session.render_now();
            driver.arm_tool_freeze_gate().await;
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
            crate::lag::note_detail(
                "host_run_await",
                t0,
                &format!("prompt_len={}", prompt.len()),
            );
        }
    }

    session.sync_fixed_zone(driver);
    let _ = session.render_now();
    session.sync_fold_hit_viewport();

    Ok(())
}

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
    crate::lag::note_detail(
        "host_run_await",
        t0,
        &format!("prompt_len={}", prompt.len()),
    );
}

async fn drain_footer_token_if_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if let Some(est) = session.take_pending_settlement_estimate() {
        let _ = session.take_pending_footer_token_refresh();
        let window = driver
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(0);
        let label = footer_token_label(est.provenance, est.tokens, window, est.prompt_cache_read);
        session.set_footer_token_label(Some(label));
        let _ = session.render_now();
        return;
    }
    if !session.take_pending_footer_token_refresh() {
        return;
    }
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

async fn footer_entries_available<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) -> bool {
    match session_entries_page(driver, Some(1), None).await {
        Ok((entries, _)) if !entries.is_empty() => true,
        _ => {
            session.set_footer_token_label(None);
            false
        }
    }
}
