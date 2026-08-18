//! Session-resume picker select / rename / delete.

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::commands::BUSY_SESSION_SWITCH_NOTICE;
use crate::app::tui::host::HostSession;

use super::super::helpers::{SwitchRebuildKind, note_driver_err, switch_and_rebuild_transcript};

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(session_id) = session.take_pending_session_resume_select() else {
        return;
    };
    if session.is_busy() {
        // c1780 / c1800 / atm10: browse Allow; switch Reject + chrome toast A.
        session.push_chrome_toast(BUSY_SESSION_SWITCH_NOTICE);
    } else {
        log::info!(target: "xylitol::tui", "SwitchSession from resume picker session_id={}", session_id);
        switch_and_rebuild_transcript(session, driver, &session_id, SwitchRebuildKind::Resume)
            .await;
    }
    let _ = session.render_now();
}

pub(super) async fn rename<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some((id, name)) = session.take_pending_session_resume_rename() else {
        return;
    };
    if session.is_busy() {
        session.push_chrome_toast(BUSY_SESSION_SWITCH_NOTICE);
    } else {
        match driver.set_session_name_for(&id, &name).await {
            Ok(stored) => {
                session.session_resume_apply_rename(&id, &stored);
                session.push_scroll_notice(format!("Session renamed: {stored}"));
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

pub(super) async fn delete<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(id) = session.take_pending_session_resume_delete() else {
        return;
    };
    if session.is_busy() {
        session.push_chrome_toast(BUSY_SESSION_SWITCH_NOTICE);
    } else if driver.session_id().as_deref() == Some(id.as_str()) {
        session.session_resume_set_status("Cannot delete the active session");
        session.push_scroll_notice("Cannot delete the active session");
    } else {
        match driver.delete_session(&id).await {
            Ok(()) => {
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
