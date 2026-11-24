mod bang;
mod debug_activity_fold;
mod debug_verify;
pub(crate) mod designing;
mod helpers;
mod mcp;
mod pending_ui;
mod reload;
mod slash;

use xylitol_tui::Terminal;

use crate::driver::{EventStream, QueueStats, XyDriver, XyDriverError};

use crate::dispatch::{DispatchOutcome, dispatch};
use xylitol_protocol::Command;

use crate::commands::slash_allowances;
use xylitol_protocol::registry::Exec;

use super::host::HostSession;
use super::widgets::footer_token_label;

pub use bang::run_interactive_bang;
pub use reload::{default_client_reload_steps, run_interactive_reload};

pub async fn drain_inline_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.take_pending_queue_refresh() {
        let stats = queue_stats(driver).await;
        session.set_queue_depth(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
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
        Ok(crate::dispatch::DispatchOutcome::QueueItems(items)) => QueueStats {
            steer_count: items
                .iter()
                .filter(|i| i.kind == xylitol_plugin_sdk::session::QueueItemKind::Steer)
                .count(),
            follow_up_count: items
                .iter()
                .filter(|i| i.kind == xylitol_plugin_sdk::session::QueueItemKind::FollowUp)
                .count(),
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

async fn drain_abort_and_outbox<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.take_abort() {
        log::info!(target: "xylitol::tui", "XyDriver::abort (Esc)");
        driver.abort();
        let gate_was_latched = session.assembly_gate_latched();
        if gate_was_latched {
            let _ = session.take_all_follow_up();
        }
        if let Some(prompt) = session.take_gated_submit() {
            session.ui_model_mut().pop_follow_up_strip_matching(&prompt);
        }
        let cleared_steer = session.ui_model_mut().clear_steer_strip();
        let cleared_follow = if gate_was_latched {
            session.ui_model_mut().queue_item_ids.clear();
            session.ui_model_mut().clear_follow_up_strip()
        } else {
            let rows = session.ui_model().pending_follow_up.clone();
            session
                .ui_model_mut()
                .queue_item_ids
                .retain(|(text, _)| rows.contains(text));
            0
        };
        log::info!(
            target: "xylitol::tui",
            "abort clears rows gate_latched={gate_was_latched} steer={cleared_steer} follow={cleared_follow}"
        );
        session.note_user_abort();
        let _ = dispatch(
            driver,
            Command::ClearQueue {
                clear_steer: true,
                clear_follow_up: gate_was_latched,
            },
        )
        .await;
        let stats = queue_stats(driver).await;
        session.set_queue_depth(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if session.take_bash_cancelled() {
        let dropped = session.take_all_steer();
        let cleared = session.ui_model_mut().clear_steer_strip();
        log::info!(
            target: "xylitol::tui",
            "bang cancel drops local steer queued={} rows={cleared}",
            dropped.len()
        );
    }
    if let Some(id) = session.take_revoke_queue_item()
        && let Err(e) = dispatch(driver, Command::RevokeQueueItem { id }).await
    {
        e.log_failure("tui.revoke_queue_item");
        session.push_scroll_notice(format!("queue recall failed: {e}"));
    }
    if session.assembly_gate_latched() || driver.assembly_gate_open() {
        session.latch_assembly_gate();
    }
    if session.has_pending_submit() {
        return;
    }
    let steers = session.take_all_steer();
    flush_outbox(session, driver, steers, true).await;
    let follow_ups = session.take_all_follow_up();
    flush_outbox(session, driver, follow_ups, false).await;
}

async fn flush_outbox<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    messages: Vec<String>,
    steer: bool,
) {
    let mut latest_items = None;
    for msg in messages {
        let command = if steer {
            Command::Steer {
                message: msg.clone(),
            }
        } else {
            Command::FollowUp {
                message: msg.clone(),
            }
        };
        log::info!(
            target: "xylitol::tui",
            "{} prompt_len={}",
            if steer { "Command::Steer" } else { "Command::FollowUp" },
            msg.len()
        );
        match dispatch(driver, command).await {
            Ok(DispatchOutcome::QueueItemEnqueued { id, items }) => {
                session.ui_model_mut().note_queue_item_id(&msg, id);
                if !items.is_empty() {
                    latest_items = Some(items);
                }
            }
            Ok(_) => {}
            Err(e) => {
                e.log_failure("tui.queue_outbox");
                session.push_scroll_notice(format!("queue send failed: {e}"));
                session.ui_model_mut().pop_follow_up_strip_matching(&msg);
                session.ui_model_mut().pop_steer_strip_matching(&msg);
            }
        }
    }
    if let Some(items) = latest_items {
        session.set_queue_badge(&items);
    }
    let _ = session.render_now();
}

pub async fn drain_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), XyDriverError> {
    session.sync_session_sidebar_reserve();
    drain_footer_token_if_pending(session, driver).await;
    drain_abort_and_outbox(session, driver).await;
    if session.take_dequeue_one() {
        log::info!(target: "xylitol::tui", "queue row recall (single)");
        let stats = queue_stats(driver).await;
        session.set_queue_depth(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
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
        session.set_queue_depth(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }

    if let Some(slash) = session.take_slash() {
        slash::handle_slash(session, driver, slash).await;
    }

    if session.take_stage_name_request() {
        let session_id = driver.session_id().unwrap_or_default();
        if !session_id.is_empty() {
            if let Some(rx) = driver.kick_generate_session_name() {
                session.arm_stage_name_job(rx);
            } else {
                match dispatch(driver, Command::GenerateSessionName { force: false }).await {
                    Ok(DispatchOutcome::SessionName(Some(name))) => {
                        session.set_stage_name(Some(name));
                    }
                    Ok(_) => {}
                    Err(error) => {
                        log::warn!(
                            target: "xylitol::tui",
                            "stage name generation failed session_id={session_id} error={error}"
                        );
                    }
                }
            }
        }
    }

    pending_ui::drain_pending_ui(session, driver).await;
    drain_footer_token_if_pending(session, driver).await;

    if let Some(notice) = driver.take_mcp_gate_notice().await {
        session.push_scroll_notice(notice);
        session.refresh_loaded_resources(driver).await;
    }

    if driver.assembly_gate_open() {
        session.latch_assembly_gate();
    }

    release_gate_if_settled(session, driver, agent_stream).await;

    let next_submit = session.take_submit_with_attachments();
    if agent_stream.is_none()
        && let Some((prompt, attachments)) = next_submit
    {
        if gate_holds_submit(session, driver) && attachments.is_empty() {
            session.latch_assembly_gate();
            session.ui_model_mut().enqueue_steer_strip(prompt.clone());
            session.ui_model_mut().set_busy_status("Assembling");
            session.sync_ui_root_from_model();
            flush_outbox(session, driver, vec![prompt], true).await;
        } else if gate_holds_submit(session, driver) {
            hold_submit_for_gate(session, driver, prompt, attachments).await;
        } else {
            start_agent_run(session, driver, agent_stream, prompt, attachments).await;
        }
    }

    if session.assembly_gate_latched() && !driver.is_tools_frozen() {
        driver.arm_tool_freeze_gate().await;
    }

    if !session.assembly_gate_latched()
        && !session.run_active()
        && session.ui_model().status.as_deref() == Some("Assembling")
        && session.ui_model().pending_steer.is_empty()
        && session.ui_model().pending_follow_up.is_empty()
        && !session.has_gated_submit()
    {
        session.end_gated_assemble_idle();
    }

    session.sync_fixed_zone(driver);
    let _ = session.render_now();
    session.sync_fold_hit_viewport();

    Ok(())
}

fn gate_holds_submit<T: Terminal>(session: &HostSession<T>, driver: &dyn XyDriver) -> bool {
    if session.assembly_gate_expired() {
        return false;
    }
    !driver.is_tools_frozen() || driver.assembly_gate_open() || session.assembly_gate_latched()
}

async fn hold_submit_for_gate<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    prompt: String,
    attachments: crate::mcp_attach::PendingMcpAttachments,
) {
    if driver.assembly_gate_open() {
        session.latch_assembly_gate_for_mcp();
    } else {
        session.latch_assembly_gate();
    }
    session.set_gated_submit(prompt.clone(), attachments);
    session
        .ui_model_mut()
        .enqueue_follow_up_strip_first(prompt.clone());
    session.ui_model_mut().set_busy_status("Assembling");
    session.sync_ui_root_from_model();
    if let Some(root) = session.ui_root() {
        root.borrow_mut().refresh_mcp_short_cue();
    }
    let _ = session.render_now();
    driver.arm_tool_freeze_gate().await;
    session.refresh_loaded_resources(driver).await;
}

async fn release_gate_if_settled<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) {
    if agent_stream.is_some() || session.run_active() {
        return;
    }
    if session.assembly_gate_waiting_mcp() && driver.assembly_gate_open() {
        session.refresh_loaded_resources(driver).await;
    }
    let settled = session.assembly_gate_expired()
        || (session.assembly_gate_waiting_mcp() && !driver.assembly_gate_open())
        || (!session.assembly_gate_waiting_mcp() && driver.is_tools_frozen());

    if !settled || !session.assembly_gate_latched() {
        return;
    }
    session.release_assembly_gate();
    log::info!(
        target: "xylitol::tui",
        "assembly gate released gated_submit={}",
        session.has_gated_submit()
    );
    if let Some((prompt, attachments)) = session.take_gated_submit_with_attachments() {
        session.ui_model_mut().pop_follow_up_strip_matching(&prompt);
        session.end_gated_assemble_idle();
        start_run_after_tool_gate(session, driver, agent_stream, prompt, attachments).await;
        return;
    }
    let queued = !session.ui_model().pending_steer.is_empty()
        || !session.ui_model().pending_follow_up.is_empty();
    if !queued {
        session.end_gated_assemble_idle();
    }
}

async fn start_run_after_tool_gate<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
    prompt: String,
    attachments: crate::mcp_attach::PendingMcpAttachments,
) {
    session.ui_model_mut().pop_follow_up_strip_matching(&prompt);
    session.sync_ui_root_from_model();
    log::info!(
        target: "xylitol::tui",
        "XyDriver::run after tool freeze prompt_len={} contexts_pending={}",
        prompt.len(),
        !attachments.is_empty()
    );
    start_agent_run(session, driver, agent_stream, prompt, attachments).await;
}

async fn start_agent_run<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
    prompt: String,
    attachments: crate::mcp_attach::PendingMcpAttachments,
) {
    let contexts = match mcp::resolve_contexts(session, driver, &attachments).await {
        Some(contexts) => contexts,
        None => {
            session.restore_blocked_prompt(&prompt);
            session.end_gated_assemble_idle();
            let _ = session.render_now();
            return;
        }
    };
    log::info!(
        target: "xylitol::tui",
        "XyDriver::run starting prompt_len={} contexts={}",
        prompt.len(),
        contexts.len()
    );
    let context_summary = contexts
        .iter()
        .map(|c| format!("{}/{}", c.server, c.uri))
        .collect::<Vec<_>>()
        .join(", ");
    session.on_turn_started(&prompt);
    let _ = session.render_now();
    let t0 = std::time::Instant::now();
    *agent_stream = Some(if contexts.is_empty() {
        driver.run(&prompt).await
    } else {
        driver.run_with_context(&prompt, contexts).await
    });
    crate::lag::note_detail(
        "host_run_await",
        t0,
        &format!("prompt_len={}", prompt.len()),
    );
    if !context_summary.is_empty() {
        session.push_scroll_notice(format!("MCP context attached: {context_summary}"));
    }
    if !attachments.is_empty() {
        session.mcp_tray_clear();
    }
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
