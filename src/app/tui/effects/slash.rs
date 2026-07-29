//! PendingSlash side-effect arms (c1170). Still drained only via drain_pending.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{XyDriver, XyDriverError};
use crate::protocol::Command;

use super::super::bridge::{CompactionBlockStatus, UiEntry};
use super::super::commands::PendingSlash;
use super::super::host::HostSession;
use super::super::keybindings::ReloadOutcome;
use super::helpers::{format_session_stats_dump, note_driver_err};

fn last_assistant_text(entries: &[UiEntry]) -> Option<&str> {
    entries.iter().rev().find_map(|e| match e {
        UiEntry::Assistant { text } if !text.trim().is_empty() => Some(text.as_str()),
        _ => None,
    })
}

fn format_keybindings_reload(outcome: ReloadOutcome) -> String {
    match outcome {
        ReloadOutcome::Applied { path } => format!("keybindings: ok — {}", path.display()),
        ReloadOutcome::NoFile { path } => {
            format!("keybindings: ok (no file) — {}", path.display())
        }
        ReloadOutcome::Failed { path, error } => {
            format!("keybindings: failed — {} ({error})", path.display())
        }
    }
}

async fn handle_reload<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let agent_dir = super::super::keybindings::default_agent_dir();
    let mut lines = vec!["Reload:".to_string()];

    lines.push(format_keybindings_reload(
        session.reload_keybindings(&agent_dir),
    ));

    match driver.reload_runtime().await {
        Ok(report) => lines.extend(report.format_lines()),
        Err(e) => {
            e.log_failure("tui.reload_runtime");
            lines.push(format!("runtime: failed — {e}"));
        }
    }

    if let Some(pref) = session.theme_preference().map(str::to_string) {
        match session.reload_themes(&pref) {
            Ok(()) => lines.push(format!("themes: ok — kept `{pref}`")),
            Err(e) => {
                e.log_failure("tui.reload_themes");
                lines.push(format!("themes: failed — {e}"));
            }
        }
    } else {
        lines.push("themes: unchanged (no preference; kept current)".into());
    }

    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
    session.refresh_loaded_resources(driver).await;
    session.push_system_note(lines.join("\n"));
}

pub(super) async fn handle_slash<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    slash: PendingSlash,
) {
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
                        session.mount_models_picker(models, current, driver.thinking_level());
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
                Ok(DispatchOutcome::Model(_)) => {
                    session.sync_runtime_chrome(driver);
                }
                Ok(_) => {
                    session.sync_runtime_chrome(driver);
                }
                Err(e) => session.push_system_note(format!("/model failed: {e}")),
            }
            let _ = session.render_now();
        }
        PendingSlash::DebugScene(scene) => {
            let scene = scene.trim().to_ascii_lowercase();
            if scene.is_empty() || scene == "list" {
                session.push_system_note(crate::app::debug_fixtures::list_note());
            } else {
                log::info!(target: "xylitol::tui", "XyDriver::load_debug_scene scene={}", scene);
                match driver.load_debug_scene(&scene).await {
                    Ok(load) => session.apply_debug_scene(load),
                    Err(e) => note_driver_err(session, "tui.load_debug_scene", &e, e.to_string()),
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
        PendingSlash::Compact { instructions } => {
            match dispatch(
                driver,
                Command::Compact {
                    id: None,
                    instructions,
                },
            )
            .await
            {
                Ok(DispatchOutcome::Compacted(did)) => {
                    if did {
                        append_compaction_from_session(session, driver).await;
                    } else {
                        session.push_system_note("session unchanged (nothing to compact)");
                    }
                    super::refresh_footer_tokens(session, driver).await;
                }
                Ok(_) => {
                    append_compaction_from_session(session, driver).await;
                    super::refresh_footer_tokens(session, driver).await;
                }
                Err(e) => session.push_system_note(format!("/session-compact failed: {e}")),
            }
            let _ = session.render_now();
        }
        PendingSlash::Export { path } => {
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
            match dispatch(driver, Command::GetSessionStats { id: None }).await {
                Ok(DispatchOutcome::SessionStats(stats)) => {
                    let state = driver.get_state();
                    session.push_system_note(format_session_stats_dump(&stats, &state));
                }
                Ok(_) => session.push_system_note("session stats unavailable"),
                Err(e) => session.push_system_note(format!("/session failed: {e}")),
            }
            let _ = session.render_now();
        }
        PendingSlash::OpenSessionResume => {
            if session.is_busy() {
                session.push_system_note("session resume unavailable while busy");
            } else {
                // pi-style load UX: slot shows loaded/total; OSC 9;4 while scanning.
                session.mount_session_resume_loading(0, 0);
                session.set_task_progress(true);
                let _ = session.render_now();
                let listed = driver.list_sessions().await;
                session.set_task_progress(false);
                match listed {
                    Ok(entries) if entries.is_empty() => {
                        session.close_session_resume_slot();
                        session.push_system_note("no sessions to resume");
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
            }
            let _ = session.render_now();
        }
        PendingSlash::SessionNew => {
            if session.is_busy() {
                session.push_system_note("session new unavailable while busy");
            } else {
                log::info!(target: "xylitol::tui", "XyDriver::new_session");
                match driver.new_session().await {
                    Ok(sid) => match driver.get_messages().await {
                        Ok(entries) => {
                            session.apply_new_session(&sid, entries);
                            session.seed_editor_history_for_new_session(driver).await;
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
        PendingSlash::SessionClone => {
            if session.is_busy() {
                session.push_system_note("session clone unavailable while busy");
            } else {
                use crate::protocol::session::ForkPosition;
                match driver.leaf_entry_id() {
                    None => session.push_system_note("Nothing to clone yet"),
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
        PendingSlash::SessionName { name } => {
            match name {
                None => match driver.get_session_name().await {
                    Ok(Some(n)) => session.push_system_note(format!("Session name: {n}")),
                    Ok(None) => session.push_system_note("usage: /session-name <name>"),
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
                            session.push_system_note(format!(
                                "Session name was normalized from {raw:?} to {stored:?}"
                            ));
                        }
                        session.push_system_note(format!("Session name set: {stored}"));
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
        PendingSlash::Reload => {
            if session.is_busy() {
                session.push_system_note("agent busy — /reload refused");
            } else {
                handle_reload(session, driver).await;
            }
            let _ = session.render_now();
        }
        PendingSlash::Trust { mode } => {
            if session.is_busy() {
                session.push_system_note("agent busy — /trust refused");
            } else {
                match driver.persist_project_trust(mode) {
                    Ok(report) => session.push_system_note(report.message),
                    Err(e) => {
                        e.log_failure("tui.persist_project_trust");
                        session.push_system_note(format!("/trust failed: {e}"));
                    }
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::HistoryCopyLast => {
            // c1110: busy allowed (readonly). Native copy async; OSC52 on host thread.
            match last_assistant_text(&session.ui_model().entries) {
                None => session.push_system_note("no assistant message to copy"),
                Some(text) => {
                    let text = text.to_string();
                    let n = text.chars().count();
                    match driver.copy_text_to_clipboard(&text).await {
                        Ok(outcome) => {
                            if let Some(seq) = outcome.pending_osc52.as_deref() {
                                session.emit_clipboard_osc52(seq);
                            }
                            session.push_system_note(format!(
                                "Copied last assistant message ({n} chars)"
                            ));
                        }
                        Err(e) => note_driver_err(
                            session,
                            "tui.copy_text_to_clipboard",
                            &e,
                            format!("/history-copy-last failed: {e}"),
                        ),
                    }
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::Theme { arg } => {
            if session.is_busy() {
                session.push_system_note("agent busy — /theme refused");
            } else {
                match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    None => session.mount_themes_picker(),
                    Some(name) => {
                        let resolved = resolve_theme_arg(session, name);
                        match resolved {
                            Ok(theme_name) => match session.reload_themes(&theme_name) {
                                Ok(()) => {}
                                Err(e) => note_driver_err(
                                    session,
                                    "tui.reload_themes.slash",
                                    &e,
                                    format!("/theme failed: {e}"),
                                ),
                            },
                            Err(e) => {
                                note_driver_err(session, "tui.resolve_theme_arg", &e, e.to_string())
                            }
                        }
                    }
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::Usage(msg) => {
            session.push_system_note(msg.to_string());
            let _ = session.render_now();
        }
    }
}

/// Resolve `/theme` argument to a built-in name (c1115).
fn resolve_theme_arg<T: Terminal>(
    session: &HostSession<T>,
    arg: &str,
) -> Result<String, XyDriverError> {
    match arg.to_ascii_lowercase().as_str() {
        "dark" | "light" => Ok(arg.to_ascii_lowercase()),
        "toggle" | "cycle" => {
            let current = session.theme_preference().unwrap_or("dark");
            let next = if current.eq_ignore_ascii_case("light") {
                "dark"
            } else {
                "light"
            };
            Ok(next.to_string())
        }
        _ => Err(XyDriverError::invalid_input(format!(
            "unknown theme `{arg}` (usage: /theme [dark|light|toggle])"
        ))),
    }
}

/// After slash/force compact, surface the latest CompactionEntry as a collapsed block.
async fn append_compaction_from_session<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn XyDriver,
) {
    let Ok(entries) = driver.get_messages().await else {
        session.push_system_note("session compacted");
        return;
    };
    let Some(comp) = entries.iter().rev().find_map(|e| match e {
        crate::protocol::session::SessionEntry::Compaction(c) => Some(c),
        _ => None,
    }) else {
        session.push_system_note("session compacted");
        return;
    };
    // Avoid duplicate if CompactionEnd already arrived via turn stream / tee.
    let already = session.ui_model().entries.iter().any(|e| {
        matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Complete,
                tokens_before,
                ..
            } if *tokens_before == comp.tokens_before
        )
    });
    if already {
        return;
    }
    session.ui_model_mut().entries.push(UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: comp.summary.clone(),
        tokens_before: comp.tokens_before,
        detail: None,
    });
    session.sync_ui_root_from_model();
}
