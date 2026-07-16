//! PendingSlash side-effect arms (c1170). Still drained only via drain_pending.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::Driver;
use crate::protocol::Command;

use super::super::commands::PendingSlash;
use super::super::host::HostSession;
use super::super::keybindings::ReloadOutcome;
use super::helpers::format_session_stats_dump;

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

async fn handle_reload<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn Driver) {
    let agent_dir = super::super::keybindings::default_agent_dir();
    let mut lines = vec!["Reload:".to_string()];

    lines.push(format_keybindings_reload(
        session.reload_keybindings(&agent_dir),
    ));

    match driver.reload_runtime().await {
        Ok(report) => lines.extend(report.format_lines()),
        Err(e) => lines.push(format!("runtime: failed — {e}")),
    }

    if let Some(pref) = session.theme_preference().map(str::to_string) {
        match session.reload_themes(&pref) {
            Ok(()) => lines.push(format!("themes: ok — kept `{pref}`")),
            Err(e) => lines.push(format!("themes: failed — {e}")),
        }
    } else {
        lines.push("themes: unchanged (no preference; kept current)".into());
    }

    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
    session.push_system_note(lines.join("\n"));
}

pub(super) async fn handle_slash<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
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
                        super::refresh_footer_tokens(session, driver).await;
                    }
                    Ok(_) => {
                        session.push_system_note("session compact complete");
                        super::refresh_footer_tokens(session, driver).await;
                    }
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
                        session.push_system_note(format!("/session-resume failed: {e}"));
                    }
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::SessionNew => {
            if session.is_busy() {
                session.push_system_note("session new unavailable while busy");
            } else {
                log::info!(target: "xylitol::tui", "Driver::new_session");
                match driver.new_session().await {
                    Ok(sid) => match driver.get_messages().await {
                        Ok(entries) => session.apply_new_session(&sid, entries),
                        Err(e) => session
                            .push_system_note(format!("new session: get_messages failed: {e}")),
                    },
                    Err(e) => session.push_system_note(format!("/session-new failed: {e}")),
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::SessionClone => {
            if session.is_busy() {
                session.push_system_note("session clone unavailable while busy");
            } else {
                use crate::domain::session_types::ForkPosition;
                match driver.leaf_entry_id() {
                    None => session.push_system_note("Nothing to clone yet"),
                    Some(entry_id) => {
                        log::info!(
                            target: "xylitol::tui",
                            "Driver::fork_session(At) + switch for /session-clone entry_id={}",
                            entry_id
                        );
                        match driver.fork_session(&entry_id, ForkPosition::At).await {
                            Ok(child_id) => match driver.switch_session(&child_id).await {
                                Ok(_) => match driver.get_messages().await {
                                    Ok(entries) => session.apply_clone_session(&child_id, entries),
                                    Err(e) => session.push_system_note(format!(
                                        "clone: get_messages failed: {e}"
                                    )),
                                },
                                Err(e) => session
                                    .push_system_note(format!("switch after clone failed: {e}")),
                            },
                            Err(e) => {
                                session.push_system_note(format!("/session-clone failed: {e}"))
                            }
                        }
                    }
                }
            }
            let _ = session.render_now();
        }
        PendingSlash::SessionName { name } => {
            if session.is_busy() {
                session.push_system_note("session name unavailable while busy");
            } else {
                match name {
                    None => match driver.get_session_name().await {
                        Ok(Some(n)) => session.push_system_note(format!("Session name: {n}")),
                        Ok(None) => session.push_system_note("usage: /session-name <name>"),
                        Err(e) => session.push_system_note(format!("/session-name failed: {e}")),
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
                        Err(e) => session.push_system_note(format!("/session-name failed: {e}")),
                    },
                }
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
        PendingSlash::Usage(msg) => {
            session.push_system_note(msg);
            let _ = session.render_now();
        }
    }
}
