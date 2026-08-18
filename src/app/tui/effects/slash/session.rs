//! Session lifecycle slash arms (`/session-*`, tree, fork, export, import).

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::protocol::Command;
use crate::protocol::session::ForkPosition;

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
    match dispatch(driver, Command::GetSessionStats { id: None }).await {
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
    let listed = driver.list_sessions().await;
    session.set_task_progress(false);
    match listed {
        Ok(entries) if entries.is_empty() => {
            session.close_session_resume_slot();
            session.push_scroll_notice("no sessions to resume");
        }
        Ok(entries) => {
            let n = entries.len();
            session.mount_session_resume_loading(n, n);
            let _ = session.render_now();
            let current = driver.session_id();
            session.mount_session_resume_picker(entries, current);
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
        log::info!(target: "xylitol::tui", "XyDriver::new_session");
        match driver.new_session().await {
            Ok(sid) => match driver.get_messages().await {
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
            },
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
                    "XyDriver::fork_session(At) + switch for /session-clone entry_id={}",
                    entry_id
                );
                match driver.fork_session(&entry_id, ForkPosition::At).await {
                    Ok(child_id) => match driver.switch_session(&child_id).await {
                        Ok(_) => match driver.get_messages().await {
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
                    },
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
        None => match driver.get_session_name().await {
            Ok(Some(n)) => session.push_scroll_notice(format!("Session name: {n}")),
            Ok(None) => session.push_scroll_notice("usage: /session-name <name>"),
            Err(e) => note_driver_err(
                session,
                "tui.get_session_name",
                &e,
                format!("/session-name failed: {e}"),
            ),
        },
        Some(raw) => match driver.set_session_name(&raw).await {
            Ok(stored) => {
                if stored != raw {
                    session.push_scroll_notice(format!(
                        "Session name was normalized from {raw:?} to {stored:?}"
                    ));
                }
                session.push_scroll_notice(format!("Session name set: {stored}"));
            }
            Err(e) => note_driver_err(
                session,
                "tui.set_session_name",
                &e,
                format!("/session-name failed: {e}"),
            ),
        },
    }
    let _ = session.render_now();
}
