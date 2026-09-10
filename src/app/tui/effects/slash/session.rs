//! Session lifecycle slash arms (`/session-*`, tree, fork, export, import).

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::protocol::Command;

use super::super::helpers::{format_session_stats_dump, note_driver_err};

pub(super) async fn open_tree<T: Terminal>(session: &mut HostSession<T>) {
    if session.is_busy() {
        session.push_scroll_notice("session tree unavailable while busy");
    } else {
        session.request_session_tree_open();
    }
    let _ = session.render_now();
}

pub(super) async fn fork_at_leaf<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.is_busy() {
        session.push_scroll_notice("fork unavailable while busy");
    } else {
        match driver.leaf_entry_id() {
            Some(id) => session.request_session_tree_fork(id),
            None => session.push_scroll_notice(
                "fork failed: no leaf (send a message first, or /session-tree then Shift+F)",
            ),
        }
    }
    let _ = session.render_now();
}

pub(super) async fn export<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    path: Option<String>,
) {
    let cmd = if path
        .as_ref()
        .is_some_and(|p| p.to_ascii_lowercase().ends_with(".jsonl"))
    {
        Command::ExportJsonl { output_path: path }
    } else {
        Command::ExportHtml { output_path: path }
    };
    match dispatch(driver, cmd).await {
        Ok(DispatchOutcome::ExportedPath(written)) => {
            session.push_scroll_notice(format!("exported → {written}"));
        }
        Ok(_) => session.push_scroll_notice("session exported"),
        Err(e) => session.push_scroll_notice(format!("/session-export failed: {e}")),
    }
    let _ = session.render_now();
}

pub(super) async fn import<T: Terminal>(session: &mut HostSession<T>, path: String) {
    if session.is_busy() {
        session.push_scroll_notice("session import unavailable while busy");
    } else {
        session.mount_import_confirm(&path);
    }
    let _ = session.render_now();
}

pub(super) async fn dump<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    match dispatch(driver, Command::GetSessionStats {}).await {
        Ok(DispatchOutcome::SessionStats(stats)) => {
            let state = driver.get_state();
            session.push_scroll_notice(format_session_stats_dump(&stats, &state));
        }
        Ok(_) => session.push_scroll_notice("session stats unavailable"),
        Err(e) => session.push_scroll_notice(format!("/session failed: {e}")),
    }
    let _ = session.render_now();
}

pub(super) async fn open_resume<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    // c1780: busy Allow browse; switch/rename/delete gated in pending_ui.
    // pi-style load UX: slot shows loaded/total; OSC 9;4 while scanning.
    session.mount_session_resume_loading(0, 0);
    session.set_task_progress(true);
    let _ = session.render_now();
    let listed =
        crate::app::core::dispatch::dispatch(driver, crate::protocol::Command::ListSessions {})
            .await;
    session.set_task_progress(false);
    match listed {
        Ok(crate::app::core::dispatch::DispatchOutcome::Sessions(entries))
            if entries.is_empty() =>
        {
            session.close_session_resume_slot();
            session.push_scroll_notice("no sessions to resume");
        }
        Ok(crate::app::core::dispatch::DispatchOutcome::Sessions(entries)) => {
            let n = entries.len();
            session.mount_session_resume_loading(n, n);
            let _ = session.render_now();
            let current = driver.session_id();
            session.mount_session_resume_picker(entries, current);
        }
        Ok(other) => {
            session.close_session_resume_slot();
            let e = super::super::helpers::outcome_error("tui.list_sessions", &other);
            note_driver_err(
                session,
                "tui.list_sessions",
                &e,
                format!("/session-resume failed: {e}"),
            );
        }
        Err(e) => {
            session.close_session_resume_slot();
            note_driver_err(
                session,
                "tui.list_sessions",
                &e,
                format!("/session-resume failed: {e}"),
            );
        }
    }
    let _ = session.render_now();
}

pub(super) async fn new_session<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.is_busy() {
        session.push_scroll_notice("session new unavailable while busy");
    } else {
        log::info!(target: "xylitol::tui", "Command::NewSession");
        let created =
            crate::app::core::dispatch::dispatch(driver, crate::protocol::Command::NewSession {})
                .await;
        match created {
            Ok(crate::app::core::dispatch::DispatchOutcome::NewSession(sid)) => {
                match super::super::session_entries(driver).await {
                    Ok(entries) => {
                        session.apply_new_session(&sid, entries);
                        if !session.kick_editor_history_seed_async(driver) {
                            session.seed_editor_history_for_new_session(driver).await;
                        }
                    }
                    Err(e) => note_driver_err(
                        session,
                        "tui.session_new.get_messages",
                        &e,
                        format!("new session: get_messages failed: {e}"),
                    ),
                }
            }
            Ok(other) => note_driver_err(
                session,
                "tui.new_session",
                &super::super::helpers::outcome_error("tui.new_session", &other),
                format!("/session-new failed: {other:?}"),
            ),
            Err(e) => note_driver_err(
                session,
                "tui.new_session",
                &e,
                format!("/session-new failed: {e}"),
            ),
        }
    }
    let _ = session.render_now();
}

pub(super) async fn clone_session<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.is_busy() {
        session.push_scroll_notice("session clone unavailable while busy");
    } else {
        match driver.leaf_entry_id() {
            None => session.push_scroll_notice("Nothing to clone yet"),
            Some(entry_id) => {
                log::info!(
                    target: "xylitol::tui",
                    "Command::Fork(At) + SwitchSession for /session-clone entry_id={}",
                    entry_id
                );
                let forked = crate::app::core::dispatch::dispatch(
                    driver,
                    crate::protocol::Command::Fork {
                        entry_id: entry_id.clone(),
                        position: Some("at".to_string()),
                    },
                )
                .await;
                match forked {
                    Ok(crate::app::core::dispatch::DispatchOutcome::NewSession(child_id)) => {
                        let switched = crate::app::core::dispatch::dispatch(
                            driver,
                            crate::protocol::Command::SwitchSession {
                                session_path: child_id.clone(),
                            },
                        )
                        .await;
                        match switched {
                            Ok(_) => match super::super::session_entries(driver).await {
                                Ok(entries) => session.apply_clone_session(&child_id, entries),
                                Err(e) => note_driver_err(
                                    session,
                                    "tui.session_clone.get_messages",
                                    &e,
                                    format!("clone: get_messages failed: {e}"),
                                ),
                            },
                            Err(e) => note_driver_err(
                                session,
                                "tui.session_clone.switch",
                                &e,
                                format!("switch after clone failed: {e}"),
                            ),
                        }
                    }
                    Ok(other) => note_driver_err(
                        session,
                        "tui.session_clone.fork",
                        &super::super::helpers::outcome_error("tui.session_clone.fork", &other),
                        format!("/session-clone failed: {other:?}"),
                    ),
                    Err(e) => note_driver_err(
                        session,
                        "tui.session_clone.fork",
                        &e,
                        format!("/session-clone failed: {e}"),
                    ),
                }
            }
        }
    }
    let _ = session.render_now();
}

pub(super) async fn name<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    name: Option<String>,
) {
    match name {
        None => {
            let got = crate::app::core::dispatch::dispatch(
                driver,
                crate::protocol::Command::GetSessionName {},
            )
            .await;
            match got {
                Ok(crate::app::core::dispatch::DispatchOutcome::SessionName(Some(n))) => {
                    session.push_scroll_notice(format!("Session name: {n}"))
                }
                Ok(_) => session.push_scroll_notice("usage: /session-name <name>"),
                Err(e) => note_driver_err(
                    session,
                    "tui.get_session_name",
                    &e,
                    format!("/session-name failed: {e}"),
                ),
            }
        }
        Some(raw) => {
            let set = crate::app::core::dispatch::dispatch(
                driver,
                crate::protocol::Command::SetSessionName { name: raw.clone() },
            )
            .await;
            match set {
                Ok(crate::app::core::dispatch::DispatchOutcome::SessionName(Some(stored))) => {
                    if stored != raw {
                        session.push_scroll_notice(format!(
                            "Session name was normalized from {raw:?} to {stored:?}"
                        ));
                    }
                    session.push_scroll_notice(format!("Session name set: {stored}"));
                }
                Ok(other) => note_driver_err(
                    session,
                    "tui.set_session_name",
                    &super::super::helpers::outcome_error("tui.set_session_name", &other),
                    format!("/session-name failed: {other:?}"),
                ),
                Err(e) => note_driver_err(
                    session,
                    "tui.set_session_name",
                    &e,
                    format!("/session-name failed: {e}"),
                ),
            }
        }
    }
    let _ = session.render_now();
}
