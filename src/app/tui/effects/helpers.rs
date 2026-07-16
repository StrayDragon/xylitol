//! Shared helpers for effects drain (c1170).

use xylitol_tui::Terminal;

use crate::app::core::driver::Driver;

use super::super::host::HostSession;

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
    driver: &mut dyn Driver,
    session_id: &str,
    kind: SwitchRebuildKind,
) {
    let label = match kind {
        SwitchRebuildKind::Import => "import",
        SwitchRebuildKind::Resume => "resume",
    };
    match driver.switch_session(session_id).await {
        Ok(_) => match driver.get_messages().await {
            Ok(entries) => match kind {
                SwitchRebuildKind::Import => session.apply_import_session(session_id, entries),
                SwitchRebuildKind::Resume => session.apply_resume_session(session_id, entries),
            },
            Err(e) => {
                session.push_system_note(format!("{label}: get_messages failed: {e}"));
                match kind {
                    SwitchRebuildKind::Import => session.close_import_confirm(),
                    SwitchRebuildKind::Resume => session.close_session_resume_slot(),
                }
            }
        },
        Err(e) => {
            session.push_system_note(format!("{label}: switch failed: {e}"));
            match kind {
                SwitchRebuildKind::Import => session.close_import_confirm(),
                SwitchRebuildKind::Resume => session.close_session_resume_slot(),
            }
        }
    }
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
        .unwrap_or_else(|| state.thinking_level.as_str());
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
