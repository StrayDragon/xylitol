//! Session-tree open / travel / fork / label pending ops.

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::app::tui::layout::map_session_tree_nodes;
use crate::protocol::session::SessionTreeKind;

use super::super::helpers::{deepest_tree_id, note_driver_err};

pub(super) async fn open<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    if !session.take_pending_session_tree_open() {
        return;
    }
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

pub(super) async fn travel<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(entry_id) = session.take_pending_session_tree_travel() else {
        return;
    };
    log::info!(target: "xylitol::tui", "XyDriver::travel_session_tree(MessageHistory) entry_id={}", entry_id);
    match driver
        .travel_session_tree(SessionTreeKind::MessageHistory, &entry_id)
        .await
    {
        Ok(travel) => match driver.get_messages().await {
            Ok(entries) => {
                session.apply_session_tree_travel(travel, entries);
                #[cfg(test)]
                super::super::refresh_footer_tokens(session, driver).await;
                #[cfg(not(test))]
                super::super::kick_footer_token_refresh(session, driver).await;
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

/// Returns `false` if the pump must abort remaining UI ops (parent get_messages failed).
pub(super) async fn fork<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    entry_id: String,
) -> bool {
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
            return false;
        }
    };
    let selected = parent_entries
        .iter()
        .find(|e| e.entry_id() == Some(entry_id.as_str()));
    match selected {
        None => {
            session.push_scroll_notice(format!("fork failed: entry not found: {entry_id}"));
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
                Err(e) => {
                    note_driver_err(session, "tui.fork_session", &e, format!("fork failed: {e}"))
                }
            }
        }
    }
    let _ = session.render_now();
    true
}

pub(super) async fn label<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some((entry_id, label)) = session.take_pending_session_tree_label() else {
        return;
    };
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
