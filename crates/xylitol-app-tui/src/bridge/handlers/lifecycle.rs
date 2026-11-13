use crate::bridge::{CompactionBlockStatus, UiEntry, UiModel, UiPhase, trailing_aborted_note};
use crate::driver::XyEvent;
use xylitol_plugin_sdk::CompactionOutcome;

pub fn apply_lifecycle_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => {
            model.sync_queue(*steer_count, *follow_up_count);
            true
        }
        XyEvent::CompactionStart { .. } => {
            push_compaction_pending(model);
            model.set_busy_status("Compacting");
            true
        }
        XyEvent::CompactionEnd {
            outcome,
            summary,
            tokens_before,
            tokens_after,
            notice,
            ..
        } => {
            match outcome {
                CompactionOutcome::Completed => {
                    finish_compaction(
                        model,
                        CompactionBlockStatus::Complete,
                        summary.clone().unwrap_or_default(),
                        tokens_before.unwrap_or(0),
                        *tokens_after,
                        None,
                    );
                    if let Some(text) = notice {
                        model
                            .entries
                            .push(UiEntry::ScrollNotice { text: text.clone() });
                    }
                }
                CompactionOutcome::Failed { error, .. } => {
                    finish_compaction(
                        model,
                        CompactionBlockStatus::Failed,
                        String::new(),
                        0,
                        None,
                        Some(error.clone().unwrap_or_else(|| "compaction aborted".into())),
                    );
                }
                CompactionOutcome::Aborted => {
                    finish_compaction(
                        model,
                        CompactionBlockStatus::Failed,
                        String::new(),
                        0,
                        None,
                        Some("compaction aborted".into()),
                    );
                }
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::AutoRetryStart {
            attempt,
            max_retries,
            ..
        } => {
            model.set_busy_status(format!("Retry {attempt}/{max_retries}"));
            true
        }
        XyEvent::AutoRetryEnd { success, attempt } => {
            if !*success {
                model.entries.push(UiEntry::ScrollNotice {
                    text: format!("retry failed (attempt {attempt})"),
                });
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::Error(err) => {
            if err.is_aborted() {
                model.flush_streaming();
                if !trailing_aborted_note(&model.entries) {
                    model.entries.push(UiEntry::ScrollNotice {
                        text: "Operation aborted".into(),
                    });
                }
                model.clear_streaming_buffers();
                if model.queue.follow_up_count == 0 {
                    model.phase = UiPhase::Idle;
                    model.status = None;
                }
            } else {
                log::warn!(
                    target: "xylitol::tui",
                    "XyEvent::Error error.kind={} message={}",
                    err.kind,
                    err.message
                );
                model.entries.push(UiEntry::Error {
                    text: err.message.clone(),
                });
                if model.phase == UiPhase::Busy && model.queue.follow_up_count == 0 {
                    model.phase = UiPhase::Idle;
                    model.status = None;
                }
            }
            true
        }
        XyEvent::SessionInfoChanged { key, value } => {
            if key == "mcp_oauth"
                && let Ok(oauth) =
                    serde_json::from_value::<xylitol_plugin_sdk::mcp::McpOAuthEvent>(value.clone())
            {
                let text = match oauth {
                    xylitol_plugin_sdk::mcp::McpOAuthEvent::Started { flow } => format!(
                        "MCP {} authorization URL: {} (open in browser; /mcp cancel {})",
                        flow.server, flow.auth_url, flow.flow_id
                    ),
                    xylitol_plugin_sdk::mcp::McpOAuthEvent::Completed { server, .. } => {
                        format!("MCP {server} authorization completed")
                    }
                    xylitol_plugin_sdk::mcp::McpOAuthEvent::Failed { server, error, .. } => {
                        format!("MCP {server} authorization failed: {error}")
                    }
                };
                model.entries.push(UiEntry::ScrollNotice { text });
            }
            true
        }
        XyEvent::ModelSelect { .. }
        | XyEvent::ThinkingLevelChanged { .. }
        | XyEvent::ContextTokenSettlement { .. } => {
            log::debug!(target: "xylitol::tui", "XyEvent ignored by bridge (metadata) event={}", event.description());
            true
        }
        _ => false,
    }
}

fn push_compaction_pending(model: &mut UiModel) {
    if model.entries.iter().any(|e| {
        matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Pending,
                ..
            }
        )
    }) {
        return;
    }
    let ordinal = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Compaction { .. }))
        .count();
    model.upsert_compaction_block(UiEntry::Compaction {
        id: format!("compaction-{ordinal}"),
        status: CompactionBlockStatus::Pending,
        summary: String::new(),
        tokens_before: 0,
        tokens_after: None,
        detail: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn finish_compaction(
    model: &mut UiModel,
    status: CompactionBlockStatus,
    summary: String,
    tokens_before: u64,
    tokens_after: Option<u64>,
    detail: Option<String>,
) {
    let id = model.entries.iter().rev().find_map(|e| match e {
        UiEntry::Compaction {
            id,
            status: CompactionBlockStatus::Pending,
            ..
        } => Some(id.clone()),
        _ => None,
    });
    let ordinal = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Compaction { .. }))
        .count();
    model.upsert_compaction_block(UiEntry::Compaction {
        id: id.unwrap_or_else(|| format!("compaction-{ordinal}")),
        status,
        summary,
        tokens_before,
        tokens_after,
        detail,
    });
}
