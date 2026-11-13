use xylitol_tui::Terminal;

use crate::activity_fold::VisibleItemKind;
use crate::commands::{FoldAction, FoldItem, StageNameAction};
use crate::dispatch::{DispatchOutcome, dispatch};
use crate::driver::XyDriver;
use crate::host::HostSession;
use xylitol_protocol::Command;

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
    session.mount_session_resume_loading(0, 0);
    session.set_task_progress(true);
    let _ = session.render_now();
    let listed =
        crate::dispatch::dispatch(driver, xylitol_protocol::Command::ListSessions {}).await;
    session.set_task_progress(false);
    match listed {
        Ok(crate::dispatch::DispatchOutcome::Sessions(entries)) if entries.is_empty() => {
            session.close_session_resume_slot();
            session.push_scroll_notice("no sessions to resume");
        }
        Ok(crate::dispatch::DispatchOutcome::Sessions(entries)) => {
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
            crate::dispatch::dispatch(driver, xylitol_protocol::Command::NewSession {}).await;
        match created {
            Ok(crate::dispatch::DispatchOutcome::NewSession(sid)) => {
                match super::super::session_entries_page(
                    driver,
                    Some(crate::history_window::HISTORY_PAGE),
                    None,
                )
                .await
                {
                    Ok((entries, has_more)) => {
                        session.apply_new_session(&sid, entries, has_more);
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
                let forked = crate::dispatch::dispatch(
                    driver,
                    xylitol_protocol::Command::Fork {
                        entry_id: entry_id.clone(),
                        position: Some("at".to_string()),
                    },
                )
                .await;
                match forked {
                    Ok(crate::dispatch::DispatchOutcome::NewSession(child_id)) => {
                        let switched = crate::dispatch::dispatch(
                            driver,
                            xylitol_protocol::Command::SwitchSession {
                                session_path: child_id.clone(),
                            },
                        )
                        .await;
                        match switched {
                            Ok(_) => match super::super::session_entries_page(
                                driver,
                                Some(crate::history_window::HISTORY_PAGE),
                                None,
                            )
                            .await
                            {
                                Ok((entries, has_more)) => {
                                    session.apply_clone_session(&child_id, entries, has_more)
                                }
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

pub(super) fn fold<T: Terminal>(session: &mut HostSession<T>, action: FoldAction) {
    let mut notice = String::new();
    if let Some(root) = session.ui_root() {
        let mut root = root.borrow_mut();
        match action {
            FoldAction::View => {
                let settings = &root.activity().settings;
                let items = settings.visible_items();
                notice = format!(
                    "display level: {:?}; thinking={} tools={} output={} diff={}",
                    settings.display_level,
                    items.thinking,
                    items.tool_header,
                    items.tool_output,
                    items.diff,
                );
            }
            FoldAction::Set(level) => {
                let mut settings = root.activity().settings.clone();
                settings.apply_display_level(level);
                root.set_activity_settings(settings);
                notice = format!("display level: {level:?}");
            }
            FoldAction::Reset => {
                root.reset_fold_overrides();
                notice = "fold reset: display level Summary, overrides cleared".into();
            }
            FoldAction::Toggle(FoldItem::Args) => {
                root.toggle_args_default();
                let mode = root.fold_view().defaults.args_expanded;
                notice = format!("fold args: {mode:?}");
            }
            FoldAction::Toggle(item) => {
                let mut settings = root.activity().settings.clone();
                let (kind, label) = match item {
                    FoldItem::Thinking => (VisibleItemKind::Thinking, "thinking"),
                    FoldItem::Tools => (VisibleItemKind::ToolHeader, "tools"),
                    FoldItem::Output => (VisibleItemKind::ToolOutput, "output"),
                    FoldItem::Diff => (VisibleItemKind::Diff, "diff"),
                    FoldItem::Args => unreachable!("args handled above"),
                };
                let current = settings.visible_items();
                let value = match kind {
                    VisibleItemKind::Thinking => !current.thinking,
                    VisibleItemKind::ToolHeader => !current.tool_header,
                    VisibleItemKind::ToolOutput => !current.tool_output,
                    VisibleItemKind::Diff => !current.diff,
                };
                settings.set_visible_item(kind, value);
                notice = format!("fold {label}: {}", if value { "on" } else { "off" });
                root.set_activity_settings(settings);
            }
        }
    }
    if !notice.is_empty() {
        session.push_scroll_notice(notice);
    }
    let _ = session.render_now();
}

pub(super) async fn stage_name<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    action: StageNameAction,
) {
    match action {
        StageNameAction::View => match dispatch(driver, Command::GetSessionName {}).await {
            Ok(DispatchOutcome::SessionName(Some(name))) => {
                session.set_stage_name(Some(name.clone()));
                session.push_scroll_notice(format!("阶段名: {name}"));
            }
            Ok(_) => session.push_scroll_notice("阶段名未生成"),
            Err(error) => note_driver_err(
                session,
                "tui.get_session_name",
                &error,
                format!("/stage-name failed: {error}"),
            ),
        },
        StageNameAction::Set(raw) => {
            match dispatch(driver, Command::SetSessionName { name: raw }).await {
                Ok(DispatchOutcome::SessionName(Some(stored))) => {
                    session.set_stage_name(Some(stored.clone()));
                    session.push_scroll_notice(format!("阶段名已更新: {stored}"));
                }
                Ok(other) => note_driver_err(
                    session,
                    "tui.set_session_name",
                    &super::super::helpers::outcome_error("tui.set_session_name", &other),
                    format!("/stage-name failed: {other:?}"),
                ),
                Err(error) => note_driver_err(
                    session,
                    "tui.set_session_name",
                    &error,
                    format!("/stage-name failed: {error}"),
                ),
            }
        }
        StageNameAction::Refresh => {
            match dispatch(driver, Command::GenerateSessionName { force: true }).await {
                Ok(DispatchOutcome::SessionName(Some(name))) => {
                    session.set_stage_name(Some(name.clone()));
                    session.push_scroll_notice(format!("阶段名已刷新: {name}"));
                }
                Ok(other) => note_driver_err(
                    session,
                    "tui.generate_session_name",
                    &super::super::helpers::outcome_error("tui.generate_session_name", &other),
                    format!("/stage-name refresh failed: {other:?}"),
                ),
                Err(error) => note_driver_err(
                    session,
                    "tui.generate_session_name",
                    &error,
                    format!("/stage-name refresh failed: {error}"),
                ),
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
            let got =
                crate::dispatch::dispatch(driver, xylitol_protocol::Command::GetSessionName {})
                    .await;
            match got {
                Ok(crate::dispatch::DispatchOutcome::SessionName(Some(n))) => {
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
            let set = crate::dispatch::dispatch(
                driver,
                xylitol_protocol::Command::SetSessionName { name: raw.clone() },
            )
            .await;
            match set {
                Ok(crate::dispatch::DispatchOutcome::SessionName(Some(stored))) => {
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
