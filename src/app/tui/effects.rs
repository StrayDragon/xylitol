//! Shared pending side-effect pump for production host loop and harness (ath6 / c494).

use std::time::Duration;

use futures::Stream;
use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{Driver, EventStream};
use crate::domain::session_types::SessionTreeKind;
use crate::protocol::Command;

use super::commands::{PendingBash, PendingSlash};
use super::host::{HostEvent, HostSession};
use super::layout::ImportConfirmDecision;
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
        log::info!(target: "xylitol::tui", "Driver::abort (Esc)");
        driver.abort();
        session.note_user_abort();
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if session.take_dequeue() {
        log::info!(target: "xylitol::tui", "Driver::clear_queue (Alt+Up dequeue)");
        let _ = driver.clear_queue(true, true);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_steer() {
        log::info!(target: "xylitol::tui", "Driver::steer prompt_len={}", msg.len());
        if let Err(e) = driver.steer(&msg) {
            session.push_system_note(format!("steer failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_follow_up() {
        log::info!(target: "xylitol::tui", "Driver::follow_up prompt_len={}", msg.len());
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
            PendingSlash::DebugScene(scene) => {
                let scene = scene.trim().to_ascii_lowercase();
                if scene.is_empty() || scene == "list" {
                    session.push_system_note(crate::app::debug_fixtures::list_note());
                } else {
                    log::info!(target: "xylitol::tui", "Driver::load_debug_scene scene={}", scene);
                    match driver.load_debug_scene(&scene).await {
                        Ok(load) => session.apply_debug_scene(load),
                        Err(e) => session.push_system_note(e),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::OpenTree => {
                if session.is_busy() {
                    session.push_system_note("session tree unavailable while busy");
                } else {
                    session.request_session_tree_open();
                }
                let _ = session.render_now();
            }
            PendingSlash::ForkAtLeaf => {
                if session.is_busy() {
                    session.push_system_note("fork unavailable while busy");
                } else {
                    match driver.leaf_entry_id() {
                        Some(id) => session.request_session_tree_fork(id),
                        None => session.push_system_note(
                            "fork failed: no leaf (send a message first, or /session-tree then Shift+F)",
                        ),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::Compact => {
                if session.is_busy() {
                    session.push_system_note("session compact unavailable while busy");
                } else {
                    match dispatch(driver, Command::Compact { id: None }).await {
                        Ok(DispatchOutcome::Compacted(did)) => {
                            let msg = if did {
                                "session compacted"
                            } else {
                                "session unchanged (nothing to compact)"
                            };
                            session.push_system_note(msg);
                        }
                        Ok(_) => session.push_system_note("session compact complete"),
                        Err(e) => session.push_system_note(format!("/session-compact failed: {e}")),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::Export { path } => {
                if session.is_busy() {
                    session.push_system_note("session export unavailable while busy");
                } else {
                    let cmd = if path
                        .as_ref()
                        .is_some_and(|p| p.to_ascii_lowercase().ends_with(".jsonl"))
                    {
                        Command::ExportJsonl {
                            id: None,
                            output_path: path,
                        }
                    } else {
                        Command::ExportHtml {
                            id: None,
                            output_path: path,
                        }
                    };
                    match dispatch(driver, cmd).await {
                        Ok(DispatchOutcome::ExportedPath(written)) => {
                            session.push_system_note(format!("exported → {written}"));
                        }
                        Ok(_) => session.push_system_note("session exported"),
                        Err(e) => session.push_system_note(format!("/session-export failed: {e}")),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::Import { path } => {
                if session.is_busy() {
                    session.push_system_note("session import unavailable while busy");
                } else {
                    session.mount_import_confirm(&path);
                }
                let _ = session.render_now();
            }
            PendingSlash::SessionDump => {
                if session.is_busy() {
                    session.push_system_note("session info unavailable while busy");
                } else {
                    match dispatch(driver, Command::GetSessionStats { id: None }).await {
                        Ok(DispatchOutcome::SessionStats(stats)) => {
                            let state = driver.get_state();
                            session.push_system_note(format_session_stats_dump(&stats, &state));
                        }
                        Ok(_) => session.push_system_note("session stats unavailable"),
                        Err(e) => session.push_system_note(format!("/session failed: {e}")),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::OpenSessionResume => {
                if session.is_busy() {
                    session.push_system_note("session resume unavailable while busy");
                } else {
                    match driver.list_sessions().await {
                        Ok(entries) if entries.is_empty() => {
                            session.push_system_note("no sessions to resume");
                        }
                        Ok(entries) => {
                            let current = driver.session_id();
                            session.mount_session_resume_picker(entries, current);
                        }
                        Err(e) => session.push_system_note(format!("/session-resume failed: {e}")),
                    }
                }
                let _ = session.render_now();
            }
            PendingSlash::Usage(msg) => {
                session.push_system_note(msg);
                let _ = session.render_now();
            }
        }
    }

    // Bang (`!`/`!!`) is NOT awaited here — host loop / pump runs it so Esc can
    // abort concurrently (c665). Callers MUST `take_bash` after drain_pending.

    if session.take_pending_session_tree_open() {
        log::info!(target: "xylitol::tui", "Driver::session_tree(MessageHistory)");
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
        log::info!(target: "xylitol::tui", "Driver::travel_session_tree(MessageHistory) entry_id={}", entry_id);
        match driver
            .travel_session_tree(SessionTreeKind::MessageHistory, &entry_id)
            .await
        {
            Ok(travel) => match driver.get_messages().await {
                Ok(entries) => session.apply_session_tree_travel(travel, entries),
                Err(e) => session.push_system_note(format!("travel: get_messages failed: {e}")),
            },
            Err(e) => session.push_system_note(format!("travel failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(entry_id) = session.take_pending_session_tree_fork() {
        use crate::domain::session_types::{ForkPosition, is_user_message, message_text};

        log::info!(target: "xylitol::tui", "Driver::fork_session + switch_session entry_id={}", entry_id);
        let parent_entries = match driver.get_messages().await {
            Ok(e) => e,
            Err(e) => {
                session.push_system_note(format!("fork: get_messages failed: {e}"));
                let _ = session.render_now();
                return Ok(());
            }
        };
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
                        Ok(_) => match driver.get_messages().await {
                            Ok(entries) => {
                                session.apply_session_tree_fork(&child_id, entries, prefill);
                            }
                            Err(e) => {
                                session.push_system_note(format!("fork: get_messages failed: {e}"))
                            }
                        },
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
        log::info!(target: "xylitol::tui", "Driver::append_entry_label entry_id={}", entry_id);
        match driver.append_entry_label(&entry_id, label.as_deref()).await {
            Ok(()) => session.apply_session_tree_label(&entry_id, label),
            Err(e) => session.push_system_note(format!("label failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(model_id) = session.take_pending_model_select() {
        log::info!(target: "xylitol::tui", "SetModel from picker model_id={}", model_id);
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

    if let Some(decision) = session.take_pending_import_decision() {
        match decision {
            ImportConfirmDecision::Rejected => {
                session.push_system_note("Import cancelled");
                session.close_import_confirm();
            }
            ImportConfirmDecision::Accepted { path } => {
                log::info!(target: "xylitol::tui", "ImportJsonl path={}", path);
                match dispatch(
                    driver,
                    Command::ImportJsonl {
                        id: None,
                        input_path: path.clone(),
                    },
                )
                .await
                {
                    Ok(DispatchOutcome::NewSession(id)) => {
                        switch_and_rebuild_transcript(
                            session,
                            driver,
                            &id,
                            SwitchRebuildKind::Import,
                        )
                        .await;
                    }
                    Ok(_) => {
                        session.push_system_note("import complete");
                        session.close_import_confirm();
                    }
                    Err(e) => {
                        session.push_system_note(format!("/session-import failed: {e}"));
                        session.close_import_confirm();
                    }
                }
            }
        }
        let _ = session.render_now();
    }

    if let Some(session_id) = session.take_pending_session_resume_select() {
        log::info!(target: "xylitol::tui", "SwitchSession from resume picker session_id={}", session_id);
        switch_and_rebuild_transcript(session, driver, &session_id, SwitchRebuildKind::Resume)
            .await;
        let _ = session.render_now();
    }

    if agent_stream.is_none()
        && let Some(prompt) = session.take_submit()
    {
        log::info!(target: "xylitol::tui", "Driver::run starting prompt_len={}", prompt.len());
        session.on_run_started(&prompt);
        let _ = session.render_now();
        *agent_stream = Some(driver.run(&prompt).await);
    }

    Ok(())
}

/// Interactive bang loop shared by production `run_host_loop` and harness (c715 / ath9).
///
/// `input` yields terminal-side [`HostEvent`]s (Input / Paste / Resize). Tick is owned
/// here (16ms). Esc → `Driver::abort` + [`HostSession::note_bash_cancelled`] (not agent
/// Aborted). Empty `input` is valid for fire-and-forget bang that completes without Esc.
pub async fn run_interactive_bang<T, S>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
    bash: PendingBash,
    agent_stream: &mut Option<EventStream>,
    input: S,
) -> Result<(), String>
where
    T: Terminal,
    S: Stream<Item = Result<HostEvent, String>>,
{
    tokio::pin!(input);
    log::info!(target: "xylitol::tui", "Driver::execute_bash (interactive bang) command_len={} exclude={}", bash.command.len(), bash.exclude_from_context);
    session.begin_bash_exec(&bash.command, bash.exclude_from_context);
    let _ = session.render_now();
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let (bash_result, aborted_during_bash) = {
        let bash_fut =
            driver.execute_bash(&bash.command, bash.exclude_from_context, Some(chunk_tx));
        tokio::pin!(bash_fut);
        let mut ticker = tokio::time::interval(Duration::from_millis(16));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut aborted_during_bash = false;
        let bash_result = loop {
            tokio::select! {
                biased;
                result = &mut bash_fut => break result,
                chunk = chunk_rx.recv() => {
                    if let Some(bytes) = chunk {
                        session.append_bash_chunk(&bytes);
                    }
                }
                _ = ticker.tick() => {
                    session.step(HostEvent::Tick)?;
                }
                maybe = input.next() => {
                    match maybe {
                        Some(Ok(ev)) => {
                            session.step(ev)?;
                            if session.take_abort() {
                                if aborted_during_bash {
                                    continue;
                                }
                                log::info!(
                                    target: "xylitol::tui",
                                    "Driver::abort during bang"
                                );
                                driver.abort();
                                session.note_bash_cancelled();
                                aborted_during_bash = true;
                                let _ = session.render_now();
                            }
                        }
                        Some(Err(e)) => {
                            session.tui.finish_inline();
                            return Err(e);
                        }
                        None => {
                            session.request_quit();
                            break Err("input closed during bang".into());
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
                            log::debug!(
                                target: "xylitol::tui",
                                "agent EventStream ended during bang"
                            );
                            *agent_stream = None;
                            session.on_run_stream_closed();
                            let _ = session.tui.try_render();
                        }
                    }
                }
            }
        };
        (bash_result, aborted_during_bash)
    };
    match bash_result {
        Ok(r) => {
            if !aborted_during_bash {
                session.push_bash_result(&bash.command, &r);
            }
        }
        Err(e) => session.push_system_note(format!("bash failed: {e}")),
    }
    session.end_bash_exec();
    if aborted_during_bash {
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
    }
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

enum SwitchRebuildKind {
    Import,
    Resume,
}

/// Shared import/resume path: `switch_session` → rebuild transcript (c1010 / c1015).
async fn switch_and_rebuild_transcript<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
    session_id: &str,
    kind: SwitchRebuildKind,
) {
    let label = match kind {
        SwitchRebuildKind::Import => "import",
        SwitchRebuildKind::Resume => "resume",
    };
    match driver.switch_session(session_id).await {
        Ok(_) => match driver.get_messages().await {
            Ok(entries) => match kind {
                SwitchRebuildKind::Import => session.apply_import_session(session_id, entries),
                SwitchRebuildKind::Resume => session.apply_resume_session(session_id, entries),
            },
            Err(e) => {
                session.push_system_note(format!("{label}: get_messages failed: {e}"));
                match kind {
                    SwitchRebuildKind::Import => session.close_import_confirm(),
                    SwitchRebuildKind::Resume => session.close_session_resume_slot(),
                }
            }
        },
        Err(e) => {
            session.push_system_note(format!("{label}: switch failed: {e}"));
            match kind {
                SwitchRebuildKind::Import => session.close_import_confirm(),
                SwitchRebuildKind::Resume => session.close_session_resume_slot(),
            }
        }
    }
}

/// Pi-aligned session info/stats text block for `/session` (c1015).
fn format_session_stats_dump(
    stats: &serde_json::Value,
    state: &crate::app::core::driver::SessionState,
) -> String {
    let session_id = stats
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or(&state.session_id);
    let user = stats
        .get("user_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let assistant = stats
        .get("assistant_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total = stats
        .get("total_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let thinking = stats
        .get("thinking_level")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| state.thinking_level.as_str());
    let mut lines = vec![
        "Session Info".to_string(),
        format!("  Session: {session_id}"),
    ];
    if let Some(model) = stats.get("model").and_then(|m| {
        let provider = m.get("provider")?.as_str()?;
        let model_id = m.get("model_id")?.as_str()?;
        Some(format!("{provider}/{model_id}"))
    }) {
        lines.push(format!("  Model: {model}"));
    } else if let Some(m) = &state.model {
        let label = if m.display_name.is_empty() {
            m.id.clone()
        } else {
            m.display_name.clone()
        };
        lines.push(format!("  Model: {label}"));
    }
    lines.push(format!("  Thinking: {thinking}"));
    lines.push(String::new());
    lines.push("Messages".to_string());
    lines.push(format!("  User: {user}"));
    lines.push(format!("  Assistant: {assistant}"));
    lines.push(format!("  Total: {total}"));
    lines.join("\n")
}
