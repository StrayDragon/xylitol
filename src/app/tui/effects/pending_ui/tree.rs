//! Session-tree open / travel / fork / label pending ops.

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::app::tui::layout::map_session_tree_nodes;
use crate::protocol::session::SessionTreeKind;

use super::super::helpers::{deepest_tree_id, note_driver_err};

/// Turn an unexpected `DispatchOutcome` into a driver error for UI notices (c2710).
pub(super) fn outcome_error(
    op: &str,
    outcome: &crate::app::core::dispatch::DispatchOutcome,
) -> crate::app::core::driver::XyDriverError {
    crate::app::core::driver::XyDriverError::invalid_input(format!(
        "{op}: unexpected outcome {outcome:?}"
    ))
}

pub(super) async fn open<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    if !session.take_pending_session_tree_open() {
        return;
    }
    log::info!(target: "xylitol::tui", "Command::SessionTree(MessageHistory)");
    match crate::app::core::dispatch::dispatch(
        driver,
        crate::protocol::Command::SessionTree {
            kind: SessionTreeKind::MessageHistory,
        },
    )
    .await
    {
        Ok(crate::app::core::dispatch::DispatchOutcome::SessionTree(nodes)) => {
            let mapped = map_session_tree_nodes(&nodes);
            let active = deepest_tree_id(&mapped);
            session.mount_session_tree(mapped, active);
        }
        Ok(other) => note_driver_err(
            session,
            "tui.session_tree",
            &outcome_error("tui.session_tree", &other),
            format!("session tree failed: {other:?}"),
        ),
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
    log::info!(target: "xylitol::tui", "Command::TravelSessionTree(MessageHistory) entry_id={}", entry_id);
    match crate::app::core::dispatch::dispatch(
        driver,
        crate::protocol::Command::TravelSessionTree {
            kind: SessionTreeKind::MessageHistory,
            entry_id: entry_id.clone(),
        },
    )
    .await
    {
        Ok(crate::app::core::dispatch::DispatchOutcome::SessionTreeTravel(travel)) => {
            match super::super::session_entries(driver).await {
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
            }
        }
        Ok(other) => note_driver_err(
            session,
            "tui.travel_session_tree",
            &outcome_error("tui.travel_session_tree", &other),
            format!("travel failed: {other:?}"),
        ),
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

    log::info!(target: "xylitol::tui", "Command::Fork + SwitchSession entry_id={}", entry_id);
    let parent_entries = match super::super::session_entries(driver).await {
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
            let pos_str = match position {
                ForkPosition::Before => "before",
                ForkPosition::At => "at",
            };
            let forked = crate::app::core::dispatch::dispatch(
                driver,
                crate::protocol::Command::Fork {
                    entry_id: entry_id.clone(),
                    position: Some(pos_str.to_string()),
                },
            )
            .await;
            match forked {
                Ok(crate::app::core::dispatch::DispatchOutcome::NewSession(child_id)) => {
                    match crate::app::core::dispatch::dispatch(
                        driver,
                        crate::protocol::Command::SwitchSession {
                            session_path: child_id.clone(),
                        },
                    )
                    .await
                    {
                        Ok(_) => match super::super::session_entries(driver).await {
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
                    }
                }
                Ok(other) => note_driver_err(
                    session,
                    "tui.fork_session",
                    &outcome_error("tui.fork_session", &other),
                    format!("fork failed: {other:?}"),
                ),
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
    log::info!(target: "xylitol::tui", "Command::AppendEntryLabel entry_id={}", entry_id);
    match crate::app::core::dispatch::dispatch(
        driver,
        crate::protocol::Command::AppendEntryLabel {
            target_id: entry_id.clone(),
            label: label.clone(),
        },
    )
    .await
    {
        Ok(_) => session.apply_session_tree_label(&entry_id, label),
        Err(e) => note_driver_err(
            session,
            "tui.append_entry_label",
            &e,
            format!("label failed: {e}"),
        ),
    }
    let _ = session.render_now();
}
