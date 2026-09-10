//! Shared helpers for effects drain (c1170).

use xylitol_tui::Terminal;

use crate::app::core::driver::{XyDriver, XyDriverError};

use super::super::host::HostSession;

/// Log `error.kind` then push a scroll notice (direct Driver calls, not via dispatch).
pub(super) fn note_driver_err<T: Terminal>(
    session: &mut HostSession<T>,
    where_: &str,
    e: &XyDriverError,
    note: impl Into<String>,
) {
    e.log_failure(where_);
    session.push_scroll_notice(note);
}

/// Log `error.kind` then push an error-styled note.
pub(super) fn note_driver_err_styled<T: Terminal>(
    session: &mut HostSession<T>,
    where_: &str,
    e: &XyDriverError,
    note: impl Into<String>,
) {
    e.log_failure(where_);
    session.push_error_note(note);
}

pub(super) fn deepest_tree_id(nodes: &[xylitol_tui::TreeNode]) -> Option<String> {
    fn walk(node: &xylitol_tui::TreeNode, last: &mut Option<String>) {
        *last = Some(node.id.clone());
        for child in &node.children {
            walk(child, last);
        }
    }
    let mut last = None;
    for node in nodes {
        walk(node, &mut last);
    }
    last
}

pub(super) enum SwitchRebuildKind {
    Import,
    Resume,
}

/// Shared import/resume path: `switch_session` → rebuild transcript (c1010 / c1015).
pub(super) async fn switch_and_rebuild_transcript<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    session_id: &str,
    kind: SwitchRebuildKind,
) {
    let label = match kind {
        SwitchRebuildKind::Import => "import",
        SwitchRebuildKind::Resume => "resume",
    };
    match crate::app::core::dispatch::dispatch(
        driver,
        crate::protocol::Command::SwitchSession {
            session_path: session_id.to_string(),
        },
    )
    .await
    {
        Ok(crate::app::core::dispatch::DispatchOutcome::SwitchedSession(_))
        | Ok(crate::app::core::dispatch::DispatchOutcome::NewSession(_)) => {
            match super::session_entries(driver).await {
                Ok(entries) => match kind {
                    SwitchRebuildKind::Import => session.apply_import_session(session_id, entries),
                    SwitchRebuildKind::Resume => session.apply_resume_session(session_id, entries),
                },
                Err(e) => {
                    note_driver_err(
                        session,
                        &format!("tui.{label}.get_messages"),
                        &e,
                        format!("{label}: get_messages failed: {e}"),
                    );
                    match kind {
                        SwitchRebuildKind::Import => session.close_import_confirm(),
                        SwitchRebuildKind::Resume => session.close_session_resume_slot(),
                    }
                }
            }
        }
        Ok(other) => {
            let e = outcome_error(&format!("tui.{label}.switch_session"), &other);
            note_driver_err(
                session,
                &format!("tui.{label}.switch_session"),
                &e,
                format!("{label}: switch failed: {e}"),
            );
            match kind {
                SwitchRebuildKind::Import => session.close_import_confirm(),
                SwitchRebuildKind::Resume => session.close_session_resume_slot(),
            }
        }
        Err(e) => {
            note_driver_err(
                session,
                &format!("tui.{label}.switch_session"),
                &e,
                format!("{label}: switch failed: {e}"),
            );
            match kind {
                SwitchRebuildKind::Import => session.close_import_confirm(),
                SwitchRebuildKind::Resume => session.close_session_resume_slot(),
            }
        }
    }
    // Resume/import clears freeze + may restart MCP — refresh cue / /mcp cache now.
    session.refresh_loaded_resources(driver).await;
    session.set_mcp_blocks_agent(driver.mcp_blocks_agent());
}

/// Turn an unexpected `DispatchOutcome` into a driver error for UI notices (c2710).
pub(super) fn outcome_error(
    op: &str,
    outcome: &crate::app::core::dispatch::DispatchOutcome,
) -> crate::app::core::driver::XyDriverError {
    crate::app::core::driver::XyDriverError::invalid_input(format!(
        "{op}: unexpected outcome {outcome:?}"
    ))
}

/// Pi-aligned session info/stats text block for `/session` (c1015).
pub(super) fn format_session_stats_dump(
    stats: &serde_json::Value,
    state: &crate::app::core::driver::SessionState,
) -> String {
    let session_id = stats
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or(&state.session_id);
    let user = stats
        .get("user_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let assistant = stats
        .get("assistant_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total = stats
        .get("total_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let thinking = stats
        .get("thinking_level")
        .and_then(|v| v.as_str())
        .unwrap_or(state.thinking_level.as_str());
    let mut lines = vec![
        "Session Info".to_string(),
        format!("  Session: {session_id}"),
    ];
    if let Some(model) = stats.get("model").and_then(|m| {
        let provider = m.get("provider")?.as_str()?;
        let model_id = m.get("model_id")?.as_str()?;
        Some(format!("{provider}/{model_id}"))
    }) {
        lines.push(format!("  Model: {model}"));
    } else if let Some(m) = &state.model {
        let label = if m.display_name.is_empty() {
            m.id.clone()
        } else {
            m.display_name.clone()
        };
        lines.push(format!("  Model: {label}"));
    }
    lines.push(format!("  Thinking: {thinking}"));
    lines.push(String::new());
    lines.push("Messages".to_string());
    lines.push(format!("  User: {user}"));
    lines.push(format!("  Assistant: {assistant}"));
    lines.push(format!("  Total: {total}"));
    lines.join("\n")
}
