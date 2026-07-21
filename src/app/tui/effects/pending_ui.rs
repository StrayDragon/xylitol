//! Non-slash pending UI ops drained after slash (tree / pickers) (c1170).

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::protocol::Command;
use crate::protocol::session::SessionTreeKind;

use super::super::host::HostSession;
use super::super::layout::ImportConfirmDecision;
use super::super::layout::map_session_tree_nodes;
use super::helpers::{
    SwitchRebuildKind, deepest_tree_id, note_driver_err, note_driver_err_styled,
    switch_and_rebuild_transcript,
};

pub(super) async fn drain_pending_ui<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if session.take_paste_image() {
        log::info!(target: "xylitol::tui", "XyDriver::stage_clipboard_image");
        let image_outcome = driver.stage_clipboard_image().await;
        match image_outcome {
            Ok(Some(path)) => {
                let insert = path.display().to_string();
                if let Some(root) = session.ui_root() {
                    root.borrow_mut().insert_editor_text_at_cursor(&insert);
                }
            }
            Ok(None) | Err(_) => {
                // c1156 / pi: no image (or read failed) → try system clipboard text.
                if let Err(e) = &image_outcome {
                    e.log_failure("tui.stage_clipboard_image");
                    log::info!(
                        target: "xylitol::tui",
                        "clipboard image unavailable, trying text: {e}"
                    );
                }
                match driver.read_clipboard_text().await {
                    Ok(Some(text)) if !text.is_empty() => {
                        if let Some(root) = session.ui_root() {
                            root.borrow_mut().insert_editor_text_at_cursor(&text);
                        }
                    }
                    Ok(_) => {
                        session.push_error_note("clipboard: no image or text");
                    }
                    Err(e) => {
                        note_driver_err_styled(
                            session,
                            "tui.read_clipboard_text",
                            &e,
                            format!("clipboard: {e}"),
                        );
                    }
                }
            }
        }
        let _ = session.render_now();
    }

    if session.take_pending_thinking_cycle() {
        match driver.cycle_thinking_level() {
            Ok(level) => session.apply_thinking_level_ui(level),
            Err(e) => {
                e.log_failure("tui.cycle_thinking_level");
                log::warn!(
                    target: "xylitol::tui",
                    "cycle_thinking_level failed: {e}"
                );
            }
        }
        let _ = session.render_now();
    }

    if session.take_pending_session_tree_open() {
        log::info!(target: "xylitol::tui", "XyDriver::session_tree(MessageHistory)");
        match driver.session_tree(SessionTreeKind::MessageHistory).await {
            Ok(nodes) => {
                let mapped = map_session_tree_nodes(&nodes);
                let active = deepest_tree_id(&mapped);
                session.mount_session_tree(mapped, active);
            }
            Err(e) => note_driver_err(
                session,
                "tui.session_tree",
                &e,
                format!("session tree failed: {e}"),
            ),
        }
        let _ = session.render_now();
    }

    if let Some(entry_id) = session.take_pending_session_tree_travel() {
        log::info!(target: "xylitol::tui", "XyDriver::travel_session_tree(MessageHistory) entry_id={}", entry_id);
        match driver
            .travel_session_tree(SessionTreeKind::MessageHistory, &entry_id)
            .await
        {
            Ok(travel) => match driver.get_messages().await {
                Ok(entries) => {
                    session.apply_session_tree_travel(travel, entries);
                    #[cfg(test)]
                    super::refresh_footer_tokens(session, driver).await;
                    #[cfg(not(test))]
                    super::kick_footer_token_refresh(session, driver).await;
                }
                Err(e) => note_driver_err(
                    session,
                    "tui.travel.get_messages",
                    &e,
                    format!("travel: get_messages failed: {e}"),
                ),
            },
            Err(e) => note_driver_err(
                session,
                "tui.travel_session_tree",
                &e,
                format!("travel failed: {e}"),
            ),
        }
        let _ = session.render_now();
    }

    if let Some(entry_id) = session.take_pending_session_tree_fork() {
        use crate::protocol::session::{ForkPosition, is_user_message, message_text};

        log::info!(target: "xylitol::tui", "XyDriver::fork_session + switch_session entry_id={}", entry_id);
        let parent_entries = match driver.get_messages().await {
            Ok(e) => e,
            Err(e) => {
                note_driver_err(
                    session,
                    "tui.fork.get_messages",
                    &e,
                    format!("fork: get_messages failed: {e}"),
                );
                let _ = session.render_now();
                return;
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
                        crate::protocol::session::SessionEntry::Message(m) => {
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
                            Err(e) => note_driver_err(
                                session,
                                "tui.fork.get_messages_after",
                                &e,
                                format!("fork: get_messages failed: {e}"),
                            ),
                        },
                        Err(e) => note_driver_err(
                            session,
                            "tui.fork.switch_session",
                            &e,
                            format!("switch after fork failed: {e}"),
                        ),
                    },
                    Err(e) => note_driver_err(
                        session,
                        "tui.fork_session",
                        &e,
                        format!("fork failed: {e}"),
                    ),
                }
            }
        }
        let _ = session.render_now();
    }

    if let Some((entry_id, label)) = session.take_pending_session_tree_label() {
        log::info!(target: "xylitol::tui", "XyDriver::append_entry_label entry_id={}", entry_id);
        match driver.append_entry_label(&entry_id, label.as_deref()).await {
            Ok(()) => session.apply_session_tree_label(&entry_id, label),
            Err(e) => note_driver_err(
                session,
                "tui.append_entry_label",
                &e,
                format!("label failed: {e}"),
            ),
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
                session.apply_thinking_level_ui(driver.thinking_level());
                session.push_system_note(format!("model → {label}"));
                session.close_models_slot();
            }
            Ok(_) => {
                session.apply_thinking_level_ui(driver.thinking_level());
                session.push_system_note("model set");
                session.close_models_slot();
            }
            // dispatch already logs error.kind
            Err(e) => session.push_system_note(format!("/model failed: {e}")),
        }
        let _ = session.render_now();
    }

    if let Some(theme_name) = session.take_pending_theme_select() {
        log::info!(target: "xylitol::tui", "reload_themes from picker theme={}", theme_name);
        match session.reload_themes(&theme_name) {
            Ok(()) => {
                session.push_system_note(format!("theme → {theme_name}"));
                session.close_themes_slot();
            }
            Err(e) => note_driver_err(
                session,
                "tui.reload_themes.picker",
                &e,
                format!("/theme failed: {e}"),
            ),
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
                    // dispatch already logs error.kind
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

    if let Some((id, name)) = session.take_pending_session_resume_rename() {
        match driver.set_session_name_for(&id, &name).await {
            Ok(stored) => {
                session.session_resume_apply_rename(&id, &stored);
                session.push_system_note(format!("Session renamed: {stored}"));
            }
            Err(e) => {
                note_driver_err(
                    session,
                    "tui.set_session_name_for",
                    &e,
                    format!("rename failed: {e}"),
                );
                session.session_resume_set_status(format!("rename failed: {e}"));
            }
        }
        let _ = session.render_now();
    }

    if let Some(id) = session.take_pending_session_resume_delete() {
        if driver.session_id().as_deref() == Some(id.as_str()) {
            session.session_resume_set_status("Cannot delete the active session");
            session.push_system_note("Cannot delete the active session");
        } else {
            match driver.delete_session(&id).await {
                Ok(()) => {
                    session.session_resume_remove_entry(&id);
                    session.push_system_note(format!("Deleted session {id}"));
                }
                Err(e) => {
                    note_driver_err(
                        session,
                        "tui.delete_session",
                        &e,
                        format!("delete failed: {e}"),
                    );
                    session.session_resume_set_status(format!("delete failed: {e}"));
                }
            }
        }
        let _ = session.render_now();
    }
}
