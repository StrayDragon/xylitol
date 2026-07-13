//! Shared pending side-effect pump for production host loop and harness (ath6 / c494).

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{Driver, EventStream};
use crate::protocol::Command;

use super::commands::PendingSlash;
use super::host::HostSession;

/// Consume HostSession pending ops and call Driver / dispatch.
///
/// Ordering matches the historical `run_host_loop` body (abort → dequeue → steer →
/// follow-up → slash → bash → optional submit→`Driver::run`). Production and
/// harness MUST share this entry so slash/bash/steer branches cannot diverge.
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
            PendingSlash::CycleModel => {
                match dispatch(driver, Command::CycleModel { id: None }).await {
                    Ok(DispatchOutcome::Model(m)) => {
                        let label = if m.display_name.is_empty() {
                            m.id
                        } else {
                            m.display_name
                        };
                        session.set_footer_model(label.clone());
                        session.push_system_note(format!("model → {label}"));
                    }
                    Ok(_) => session.push_system_note("model cycled"),
                    Err(e) => session.push_system_note(format!("/model failed: {e}")),
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

    if let Some(bash) = session.take_bash() {
        tracing::info!(
            target: "xylitol::tui",
            command_len = bash.command.len(),
            exclude = bash.exclude_from_context,
            "Driver::execute_bash"
        );
        match dispatch(
            driver,
            Command::Bash {
                id: None,
                command: bash.command.clone(),
                exclude_from_context: bash.exclude_from_context,
            },
        )
        .await
        {
            Ok(DispatchOutcome::Bash(result)) => {
                session.push_bash_result(&bash.command, &result);
            }
            Ok(_) => session.push_system_note("bash: unexpected dispatch outcome"),
            Err(e) => session.push_system_note(format!("bash failed: {e}")),
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
