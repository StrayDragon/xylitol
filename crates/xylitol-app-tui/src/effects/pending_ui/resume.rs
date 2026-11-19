use xylitol_tui::Terminal;

use crate::commands::BUSY_SESSION_SWITCH_NOTICE;
use crate::driver::XyDriver;
use crate::host::HostSession;
use crate::layout::UiIntent;

pub(super) async fn apply<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    intent: UiIntent,
) {
    match intent {
        UiIntent::SessionResumeSelect(session_id) => select(session, driver, session_id).await,
        UiIntent::SessionResumeRename(id, name) => rename(session, driver, id, name).await,
        UiIntent::SessionResumeDelete(id) => delete(session, driver, id).await,
        _ => {}
    }
}

use super::super::helpers::{SwitchRebuildKind, note_driver_err, switch_and_rebuild_transcript};

pub(super) async fn select<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    session_id: String,
) {
    if session.is_busy() {
        session.push_toast_notice(BUSY_SESSION_SWITCH_NOTICE);
    } else {
        log::info!(target: "xylitol::tui", "SwitchSession from resume picker session_id={}", session_id);
        switch_and_rebuild_transcript(session, driver, &session_id, SwitchRebuildKind::Resume)
            .await;
    }
    let _ = session.render_now();
}

pub(super) async fn rename<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    id: String,
    name: String,
) {
    if session.is_busy() {
        session.push_toast_notice(BUSY_SESSION_SWITCH_NOTICE);
    } else {
        match crate::dispatch::dispatch(
            driver,
            xylitol_protocol::Command::SetSessionNameFor {
                session_id: id.clone(),
                name: name.clone(),
            },
        )
        .await
        {
            Ok(crate::dispatch::DispatchOutcome::SessionName(Some(stored))) => {
                session.session_resume_apply_rename(&id, &stored);
                session.push_scroll_notice(format!("Session renamed: {stored}"));
            }
            Ok(other) => {
                let e = super::super::helpers::outcome_error("tui.set_session_name_for", &other);
                note_driver_err(
                    session,
                    "tui.set_session_name_for",
                    &e,
                    format!("rename failed: {e}"),
                );
                session.session_resume_set_status(format!("rename failed: {e}"));
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
    }
    let _ = session.render_now();
}

pub(super) async fn delete<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    id: String,
) {
    if session.is_busy() {
        session.push_toast_notice(BUSY_SESSION_SWITCH_NOTICE);
    } else if driver.session_id().as_deref() == Some(id.as_str()) {
        session.session_resume_set_status("Cannot delete the active session");
        session.push_scroll_notice("Cannot delete the active session");
    } else {
        match crate::dispatch::dispatch(
            driver,
            xylitol_protocol::Command::DeleteSession {
                session_id: id.clone(),
            },
        )
        .await
        {
            Ok(_) => {
                session.session_resume_remove_entry(&id);
                session.push_scroll_notice(format!("Deleted session {id}"));
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
