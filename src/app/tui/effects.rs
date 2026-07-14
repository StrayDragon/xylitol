//! Shared pending side-effect pump for production host loop and harness (ath6 / c494).

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{Driver, EventStream};
use crate::domain::session_types::SessionTreeKind;
use crate::protocol::Command;

#[cfg(test)]
use super::commands::PendingBash;
use super::commands::PendingSlash;
use super::host::HostSession;
use super::layout::map_session_tree_nodes;

/// Consume HostSession pending ops and call Driver / dispatch.
///
/// Ordering matches the historical `run_host_loop` body (abort → dequeue → steer →
/// follow-up → slash → optional submit→`Driver::run`). Bang is **not** awaited here
/// (c665 — host `select!` / [`run_pending_bash`]). Production and harness MUST share
/// this entry so slash/steer branches cannot diverge.
///
/// When `agent_stream` is already `Some`, submit is not taken. A newly started
/// run is stored in `agent_stream`; callers decide whether to drain it (harness)
/// or poll it in a select loop (production).
pub async fn drain_pending<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), String> {
    if session.take_abort() {
        tracing::info!(target: "xylitol::tui", "Driver::abort (Esc)");
        driver.abort();
        session.note_user_abort();
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if session.take_dequeue() {
        tracing::info!(target: "xylitol::tui", "Driver::clear_queue (Alt+Up dequeue)");
        let _ = driver.clear_queue(true, true);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_steer() {
        tracing::info!(target: "xylitol::tui", prompt_len = msg.len(), "Driver::steer");
        if let Err(e) = driver.steer(&msg) {
            session.push_system_note(format!("steer failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_follow_up() {
        tracing::info!(target: "xylitol::tui", prompt_len = msg.len(), "Driver::follow_up");
        if let Err(e) = driver.follow_up(&msg) {
            session.push_system_note(format!("follow-up failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(slash) = session.take_slash() {
        match slash {
            PendingSlash::Exit => {
                session.request_quit();
            }
            PendingSlash::OpenModels => {
                if session.is_busy() {
                    session.push_system_note("models picker unavailable while busy");
                } else {
                    match dispatch(driver, Command::GetAvailableModels { id: None }).await {
                        Ok(DispatchOutcome::Models(models)) => {
                            let current = driver.current_model().map(|m| m.id);
                            session.mount_models_picker(models, current);
                        }
                        Ok(_) => session.push_system_note("models list unavailable"),
                        Err(e) => session.push_system_note(format!("/model failed: {e}")),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::SetModel(model_id) => {
                match dispatch(
                    driver,
                    Command::SetModel {
                        id: None,
                        provider: String::new(),
                        model_id,
                    },
                )
                .await
                {
                    Ok(DispatchOutcome::Model(m)) => {
                        let label = if m.display_name.is_empty() {
                            m.id
                        } else {
                            m.display_name
                        };
                        session.set_footer_model(label.clone());
                        session.push_system_note(format!("model → {label}"));
                    }
                    Ok(_) => session.push_system_note("model set"),
                    Err(e) => session.push_system_note(format!("/model failed: {e}")),
                }
                let _ = session.render_now();
            }
        }
    }

    // Bang (`!`/`!!`) is NOT awaited here — host loop / pump runs it so Esc can
    // abort concurrently (c665). Callers MUST `take_bash` after drain_pending.

    if session.take_pending_session_tree_open() {
        tracing::info!(target: "xylitol::tui", "Driver::session_tree(MessageHistory)");
        match driver.session_tree(SessionTreeKind::MessageHistory).await {
            Ok(nodes) => {
                let mapped = map_session_tree_nodes(&nodes);
                let active = deepest_tree_id(&mapped);
                session.mount_session_tree(mapped, active);
            }
            Err(e) => session.push_system_note(format!("session tree failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(entry_id) = session.take_pending_session_tree_travel() {
        tracing::info!(
            target: "xylitol::tui",
            entry_id = %entry_id,
            "Driver::travel_session_tree(MessageHistory)"
        );
        match driver
            .travel_session_tree(SessionTreeKind::MessageHistory, &entry_id)
            .await
        {
            Ok(travel) => {
                let entries = driver.get_messages().await.unwrap_or_default();
                session.apply_session_tree_travel(travel, entries);
            }
            Err(e) => session.push_system_note(format!("travel failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(entry_id) = session.take_pending_session_tree_fork() {
        use crate::domain::session_types::{ForkPosition, is_user_message, message_text};

        tracing::info!(
            target: "xylitol::tui",
            entry_id = %entry_id,
            "Driver::fork_session + switch_session"
        );
        let parent_entries = driver.get_messages().await.unwrap_or_default();
        let selected = parent_entries
            .iter()
            .find(|e| e.entry_id() == Some(entry_id.as_str()));
        match selected {
            None => {
                session.push_system_note(format!("fork failed: entry not found: {entry_id}"));
            }
            Some(e) => {
                let (position, prefill) = if is_user_message(e) {
                    let text = match e {
                        crate::domain::session_types::SessionEntry::Message(m) => {
                            let raw = message_text(&m.message);
                            raw.strip_prefix("[steer] ")
                                .unwrap_or(raw.as_str())
                                .to_string()
                        }
                        _ => String::new(),
                    };
                    (ForkPosition::Before, Some(text))
                } else {
                    (ForkPosition::At, None)
                };
                match driver.fork_session(&entry_id, position).await {
                    Ok(child_id) => match driver.switch_session(&child_id).await {
                        Ok(_) => {
                            let entries = driver.get_messages().await.unwrap_or_default();
                            session.apply_session_tree_fork(&child_id, entries, prefill);
                        }
                        Err(e) => {
                            session.push_system_note(format!("switch after fork failed: {e}"))
                        }
                    },
                    Err(e) => session.push_system_note(format!("fork failed: {e}")),
                }
            }
        }
        let _ = session.render_now();
    }

    if let Some((entry_id, label)) = session.take_pending_session_tree_label() {
        tracing::info!(
            target: "xylitol::tui",
            entry_id = %entry_id,
            "Driver::append_entry_label"
        );
        match driver.append_entry_label(&entry_id, label.as_deref()).await {
            Ok(()) => session.apply_session_tree_label(&entry_id, label),
            Err(e) => session.push_system_note(format!("label failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(model_id) = session.take_pending_model_select() {
        tracing::info!(target: "xylitol::tui", model_id = %model_id, "SetModel from picker");
        match dispatch(
            driver,
            Command::SetModel {
                id: None,
                provider: String::new(),
                model_id: model_id.clone(),
            },
        )
        .await
        {
            Ok(DispatchOutcome::Model(m)) => {
                let label = if m.display_name.is_empty() {
                    m.id
                } else {
                    m.display_name
                };
                session.set_footer_model(label.clone());
                session.push_system_note(format!("model → {label}"));
                session.close_models_slot();
            }
            Ok(_) => {
                session.push_system_note("model set");
                session.close_models_slot();
            }
            Err(e) => session.push_system_note(format!("/model failed: {e}")),
        }
        let _ = session.render_now();
    }

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
        *agent_stream = Some(driver.run(&prompt).await);
    }

    Ok(())
}

/// Run one pending bang bash to completion (harness / non-select callers).
#[cfg(test)]
pub async fn run_pending_bash<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn Driver,
    bash: PendingBash,
) -> Result<(), String> {
    tracing::info!(
        target: "xylitol::tui",
        command_len = bash.command.len(),
        exclude = bash.exclude_from_context,
        "Driver::execute_bash"
    );
    session.begin_bash_exec(&bash.command, bash.exclude_from_context);
    let _ = session.render_now();
    match driver
        .execute_bash(&bash.command, bash.exclude_from_context, None)
        .await
    {
        Ok(result) => {
            session.push_bash_result(&bash.command, &result);
        }
        Err(e) => session.push_system_note(format!("bash failed: {e}")),
    }
    session.end_bash_exec();
    let _ = session.render_now();
    Ok(())
}

fn deepest_tree_id(nodes: &[xylitol_tui::TreeNode]) -> Option<String> {
    fn walk(node: &xylitol_tui::TreeNode, last: &mut Option<String>) {
        *last = Some(node.id.clone());
        for child in &node.children {
            walk(child, last);
        }
    }
    let mut last = None;
    for node in nodes {
        walk(node, &mut last);
    }
    last
}
