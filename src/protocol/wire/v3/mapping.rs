//! Mapping layer — domain lifecycle events and serde wire commands ↔ v3 fory
//! shapes (c2834 task 3.2).
//!
//! Pure conversion, no transport. Semantics MUST stay aligned with
//! [`crate::protocol::wire::event`]'s `to_wire_event` / `TryFrom<&Event>` (same
//! degraded variant set, dynamic JSON blocks as `_json` strings per spec
//! r1906, counter widths widened for the binary track).

use serde_json::Value;

use super::generated as v3;
use crate::protocol::lifecycle::{XyEvent, XyEventError};
use crate::protocol::message::AgentMessage;
use crate::protocol::model::{ContextTokenEstimate, TokenProvenance};
use crate::protocol::session::{SessionTreeKind, TodoList, TodoStatus};
use crate::protocol::wire::Command as WireCommand;

// ── XyEvent → v3 Event ─────────────────────────────────────

/// Convert a domain lifecycle event to its v3 fory representation.
///
/// Returns `None` for the same internal-only events as
/// [`XyEvent::to_wire_event`] (auto-retry bookkeeping, session-info /
/// thinking-level projections covered elsewhere).
pub fn xy_event_to_v3(ev: &XyEvent) -> Option<v3::Event> {
    match ev {
        XyEvent::TextDelta(text) => {
            Some(v3::Event::TextDelta(v3::TextDelta { text: text.clone() }))
        }
        XyEvent::ThinkingDelta(text) => Some(v3::Event::ThinkingDelta(v3::ThinkingDelta {
            text: text.clone(),
        })),
        XyEvent::TurnStart { turn_index } => Some(v3::Event::TurnStart(v3::TurnStart {
            turn_index: *turn_index,
        })),
        XyEvent::TurnEnd { turn_index } => Some(v3::Event::TurnEnd(v3::TurnEnd {
            turn_index: *turn_index,
        })),
        XyEvent::MessageStart { role, message } => {
            Some(v3::Event::MessageStart(v3::MessageStart {
                role: role.clone(),
                message_json: agent_message_to_json_string(message.as_ref()),
            }))
        }
        XyEvent::MessageEnd { role, message } => Some(v3::Event::MessageEnd(v3::MessageEnd {
            role: role.clone(),
            message_json: agent_message_to_json_string(message.as_ref()),
        })),
        XyEvent::MessageUpdate {
            text,
            thinking,
            message,
        } => Some(v3::Event::MessageUpdate(v3::MessageUpdate {
            text: text.clone(),
            thinking: thinking.clone(),
            message_json: agent_message_to_json_string(message.as_ref()),
        })),
        XyEvent::ToolExecutionStart { id, name, args, .. } => {
            Some(v3::Event::ToolStart(v3::ToolStart {
                id: id.clone(),
                name: name.clone(),
                args_json: serde_json::to_string(args).unwrap_or_default(),
            }))
        }
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => Some(v3::Event::ToolEnd(v3::ToolEnd {
            id: id.clone(),
            name: name.clone(),
            result: result.clone(),
            is_error: *is_error,
        })),
        XyEvent::ToolExecutionUpdate { id, output } => {
            Some(v3::Event::ToolExecutionUpdate(v3::ToolExecutionUpdate {
                id: id.clone(),
                output: output.clone(),
            }))
        }
        XyEvent::ModelSelect { provider, model_id } => {
            Some(v3::Event::ModelSelect(v3::ModelSelect {
                provider: provider.clone(),
                model_id: model_id.clone(),
            }))
        }
        XyEvent::CompactionStart { reason } => {
            Some(v3::Event::CompactionStart(v3::CompactionStart {
                reason: reason.clone(),
            }))
        }
        XyEvent::CompactionEnd {
            result,
            aborted,
            reason,
            will_retry,
            error_message,
            summary,
            tokens_before,
            tokens_after,
            notice,
        } => Some(v3::Event::CompactionEnd(v3::CompactionEnd {
            result: result.clone(),
            aborted: *aborted,
            reason: reason.clone(),
            will_retry: *will_retry,
            error_message: error_message.clone(),
            summary: summary.clone(),
            tokens_before: *tokens_before,
            tokens_after: *tokens_after,
            notice: notice.clone(),
        })),
        XyEvent::ContextTokenSettlement {
            estimate,
            reason,
            generation,
        } => Some(v3::Event::ContextTokenSettlement(
            v3::ContextTokenSettlement {
                tokens: estimate.tokens,
                provenance: estimate.provenance.as_str().to_string(),
                usage_tokens: estimate.usage_tokens,
                trailing_tokens: estimate.trailing_tokens,
                // usize → u64 is lossless on supported targets.
                last_usage_index: estimate.last_usage_index.map(|i| i as u64),
                reason: reason.clone(),
                generation: *generation,
            },
        )),
        // Messages are REST-covered (`get_messages`), not part of the event.
        XyEvent::AgentEnd { .. } => Some(v3::Event::AgentEnd(v3::AgentEnd {})),
        XyEvent::Error(err) => Some(v3::Event::ErrorEvent(v3::ErrorEvent {
            kind: err.kind.clone(),
            message: err.message.clone(),
        })),
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => Some(v3::Event::QueueUpdate(v3::QueueUpdate {
            steer_count: *steer_count as u64,
            follow_up_count: *follow_up_count as u64,
        })),
        XyEvent::TodoUpdated { list } => Some(v3::Event::TodoListSnapshot(v3::TodoListSnapshot {
            list: Some(todo_list_to_v3(list)),
        })),
        // Degraded (not on the wire): process-local or REST-covered.
        XyEvent::AgentStart { .. }
        | XyEvent::AutoRetryStart { .. }
        | XyEvent::AutoRetryEnd { .. }
        | XyEvent::SessionInfoChanged { .. }
        | XyEvent::ThinkingLevelChanged { .. } => {
            log::debug!("v3: dropping non-mapped XyEvent event={}", ev.description());
            None
        }
    }
}

// ── v3 Event → XyEvent ─────────────────────────────────────

/// Convert a v3 fory event back to the domain lifecycle event.
///
/// Returns `None` for `Event::Unknown` (schema-ahead payload). Dynamic JSON
/// blocks parse leniently: invalid JSON degrades to `Value::Null` / `None`
/// instead of failing the whole event (spec r1719 unknown-is-degradable
/// spirit) — MUST NOT panic.
pub fn v3_event_to_xy(ev: &v3::Event) -> Option<XyEvent> {
    match ev {
        v3::Event::TextDelta(e) => Some(XyEvent::TextDelta(e.text.clone())),
        v3::Event::ThinkingDelta(e) => Some(XyEvent::ThinkingDelta(e.text.clone())),
        v3::Event::TurnStart(e) => Some(XyEvent::TurnStart {
            turn_index: e.turn_index,
        }),
        v3::Event::TurnEnd(e) => Some(XyEvent::TurnEnd {
            turn_index: e.turn_index,
        }),
        v3::Event::MessageStart(e) => Some(XyEvent::MessageStart {
            role: e.role.clone(),
            message: json_string_to_agent_message(e.message_json.as_deref()),
        }),
        v3::Event::MessageEnd(e) => Some(XyEvent::MessageEnd {
            role: e.role.clone(),
            message: json_string_to_agent_message(e.message_json.as_deref()),
        }),
        v3::Event::MessageUpdate(e) => Some(XyEvent::MessageUpdate {
            text: e.text.clone(),
            thinking: e.thinking.clone(),
            message: json_string_to_agent_message(e.message_json.as_deref()),
        }),
        v3::Event::ToolStart(e) => Some(XyEvent::ToolExecutionStart {
            id: e.id.clone(),
            name: e.name.clone(),
            args: json_string_to_value(&e.args_json),
        }),
        v3::Event::ToolEnd(e) => Some(XyEvent::ToolExecutionEnd {
            id: e.id.clone(),
            name: e.name.clone(),
            result: e.result.clone(),
            is_error: e.is_error,
        }),
        v3::Event::ToolExecutionUpdate(e) => Some(XyEvent::ToolExecutionUpdate {
            id: e.id.clone(),
            output: e.output.clone(),
        }),
        v3::Event::ModelSelect(e) => Some(XyEvent::ModelSelect {
            provider: e.provider.clone(),
            model_id: e.model_id.clone(),
        }),
        v3::Event::CompactionStart(e) => Some(XyEvent::CompactionStart {
            reason: e.reason.clone(),
        }),
        v3::Event::CompactionEnd(e) => Some(XyEvent::CompactionEnd {
            result: e.result.clone(),
            aborted: e.aborted,
            reason: e.reason.clone(),
            will_retry: e.will_retry,
            error_message: e.error_message.clone(),
            summary: e.summary.clone(),
            tokens_before: e.tokens_before,
            tokens_after: e.tokens_after,
            notice: e.notice.clone(),
        }),
        v3::Event::ContextTokenSettlement(e) => Some(XyEvent::ContextTokenSettlement {
            estimate: ContextTokenEstimate {
                tokens: e.tokens,
                provenance: token_provenance_from_str(&e.provenance),
                usage_tokens: e.usage_tokens,
                trailing_tokens: e.trailing_tokens,
                last_usage_index: e.last_usage_index.map(|i| i as usize),
            },
            reason: e.reason.clone(),
            generation: e.generation,
        }),
        // Mirrors the JSON track: AgentEnd carries no message payload.
        v3::Event::AgentEnd(_) => Some(XyEvent::AgentEnd {
            messages: Vec::new(),
        }),
        v3::Event::ErrorEvent(e) => Some(XyEvent::Error(XyEventError::new(
            e.kind.clone(),
            e.message.clone(),
        ))),
        v3::Event::QueueUpdate(e) => Some(XyEvent::QueueUpdate {
            steer_count: e.steer_count as usize,
            follow_up_count: e.follow_up_count as usize,
        }),
        v3::Event::TodoListSnapshot(e) => {
            // Absent list is not a "cleared" signal (cleared = Some(empty));
            // degrade rather than invent a snapshot.
            e.list.as_ref().map(|list| XyEvent::TodoUpdated {
                list: v3_todo_list_to_xy(list),
            })
        }
        // Degraded: schema-ahead union member from a newer peer.
        v3::Event::Unknown(_) => None,
    }
}

// ── wire Command ↔ v3 Command ──────────────────────────────

/// Convert a serde wire command to its v3 fory representation.
///
/// `ClearQueue`'s serde `default = true` semantics live at the JSON edge;
/// this layer passes the bools through as-is. `SessionTree` maps to the v3
/// `SessionTreeCmd` member (fbs keyword collision).
pub fn command_to_v3(cmd: &WireCommand) -> v3::Command {
    match cmd {
        WireCommand::Prompt { message } => v3::Command::Prompt(v3::Prompt {
            message: message.clone(),
        }),
        WireCommand::Abort {} => v3::Command::Abort(v3::Abort {}),
        WireCommand::GetState {} => v3::Command::GetState(v3::GetState {}),
        WireCommand::SetModel { provider, model_id } => v3::Command::SetModel(v3::SetModel {
            provider: provider.clone(),
            model_id: model_id.clone(),
        }),
        WireCommand::CycleModel {} => v3::Command::CycleModel(v3::CycleModel {}),
        WireCommand::GetAvailableModels {} => {
            v3::Command::GetAvailableModels(v3::GetAvailableModels {})
        }
        WireCommand::SetThinkingLevel { level } => {
            v3::Command::SetThinkingLevel(v3::SetThinkingLevel {
                level: level.clone(),
            })
        }
        WireCommand::Bash {
            command,
            exclude_from_context,
        } => v3::Command::Bash(v3::Bash {
            command: command.clone(),
            exclude_from_context: *exclude_from_context,
        }),
        WireCommand::Compact { instructions } => v3::Command::Compact(v3::Compact {
            instructions: instructions.clone(),
        }),
        WireCommand::GetSessionStats {} => v3::Command::GetSessionStats(v3::GetSessionStats {}),
        WireCommand::ExportHtml { output_path } => v3::Command::ExportHtml(v3::ExportHtml {
            output_path: output_path.clone(),
        }),
        WireCommand::ExportJsonl { output_path } => v3::Command::ExportJsonl(v3::ExportJsonl {
            output_path: output_path.clone(),
        }),
        WireCommand::ImportJsonl { input_path } => v3::Command::ImportJsonl(v3::ImportJsonl {
            input_path: input_path.clone(),
        }),
        WireCommand::SwitchSession { session_path } => {
            v3::Command::SwitchSession(v3::SwitchSession {
                session_path: session_path.clone(),
            })
        }
        WireCommand::Fork { entry_id, position } => v3::Command::Fork(v3::Fork {
            entry_id: entry_id.clone(),
            position: position.clone(),
        }),
        WireCommand::GetMessages {} => v3::Command::GetMessages(v3::GetMessages {}),
        WireCommand::EstimateContext {} => v3::Command::EstimateContext(v3::EstimateContext {}),
        WireCommand::GetCommands {} => v3::Command::GetCommands(v3::GetCommands {}),
        WireCommand::SessionTree { kind } => v3::Command::SessionTreeCmd(v3::SessionTreeCmd {
            kind: session_tree_kind_to_v3(*kind),
        }),
        WireCommand::TravelSessionTree { kind, entry_id } => {
            v3::Command::TravelSessionTree(v3::TravelSessionTree {
                kind: session_tree_kind_to_v3(*kind),
                entry_id: entry_id.clone(),
            })
        }
        WireCommand::AppendEntryLabel { target_id, label } => {
            v3::Command::AppendEntryLabel(v3::AppendEntryLabel {
                target_id: target_id.clone(),
                label: label.clone(),
            })
        }
        WireCommand::ListSessions {} => v3::Command::ListSessions(v3::ListSessions {}),
        WireCommand::LoadSessionEntries { session_id } => {
            v3::Command::LoadSessionEntries(v3::LoadSessionEntries {
                session_id: session_id.clone(),
            })
        }
        WireCommand::NewSession {} => v3::Command::NewSession(v3::NewSession {}),
        WireCommand::GetSessionName {} => v3::Command::GetSessionName(v3::GetSessionName {}),
        WireCommand::SetSessionName { name } => {
            v3::Command::SetSessionName(v3::SetSessionName { name: name.clone() })
        }
        WireCommand::SetSessionNameFor { session_id, name } => {
            v3::Command::SetSessionNameFor(v3::SetSessionNameFor {
                session_id: session_id.clone(),
                name: name.clone(),
            })
        }
        WireCommand::DeleteSession { session_id } => {
            v3::Command::DeleteSession(v3::DeleteSession {
                session_id: session_id.clone(),
            })
        }
        WireCommand::Reload {} => v3::Command::Reload(v3::Reload {}),
        WireCommand::LoadedResources {} => v3::Command::LoadedResources(v3::LoadedResources {}),
        WireCommand::GetQueueStats {} => v3::Command::GetQueueStats(v3::GetQueueStats {}),
        WireCommand::Steer { message } => v3::Command::Steer(v3::Steer {
            message: message.clone(),
        }),
        WireCommand::FollowUp { message } => v3::Command::FollowUp(v3::FollowUp {
            message: message.clone(),
        }),
        WireCommand::ClearQueue {
            clear_steer,
            clear_follow_up,
        } => v3::Command::ClearQueue(v3::ClearQueue {
            clear_steer: *clear_steer,
            clear_follow_up: *clear_follow_up,
        }),
        WireCommand::Subscribe {
            session_id,
            last_seq,
        } => v3::Command::Subscribe(v3::Subscribe {
            session_id: session_id.clone(),
            last_seq: *last_seq,
        }),
        WireCommand::ApproveTool { call_id, approved } => {
            v3::Command::ApproveTool(v3::ApproveTool {
                call_id: call_id.clone(),
                approved: *approved,
            })
        }
        WireCommand::AnswerQuestion { call_id, answer } => {
            v3::Command::AnswerQuestion(v3::AnswerQuestion {
                call_id: call_id.clone(),
                answer: answer.clone(),
            })
        }
        WireCommand::Quit {} => v3::Command::Quit(v3::Quit {}),
    }
}

/// Convert a v3 fory command back to the serde wire command.
///
/// Returns `None` for `Command::Unknown` (schema-ahead payload from a newer
/// peer).
pub fn v3_to_command(cmd: &v3::Command) -> Option<WireCommand> {
    match cmd {
        v3::Command::Prompt(e) => Some(WireCommand::Prompt {
            message: e.message.clone(),
        }),
        v3::Command::Abort(_) => Some(WireCommand::Abort {}),
        v3::Command::GetState(_) => Some(WireCommand::GetState {}),
        v3::Command::SetModel(e) => Some(WireCommand::SetModel {
            provider: e.provider.clone(),
            model_id: e.model_id.clone(),
        }),
        v3::Command::CycleModel(_) => Some(WireCommand::CycleModel {}),
        v3::Command::GetAvailableModels(_) => Some(WireCommand::GetAvailableModels {}),
        v3::Command::SetThinkingLevel(e) => Some(WireCommand::SetThinkingLevel {
            level: e.level.clone(),
        }),
        v3::Command::Bash(e) => Some(WireCommand::Bash {
            command: e.command.clone(),
            exclude_from_context: e.exclude_from_context,
        }),
        v3::Command::Compact(e) => Some(WireCommand::Compact {
            instructions: e.instructions.clone(),
        }),
        v3::Command::GetSessionStats(_) => Some(WireCommand::GetSessionStats {}),
        v3::Command::ExportHtml(e) => Some(WireCommand::ExportHtml {
            output_path: e.output_path.clone(),
        }),
        v3::Command::ExportJsonl(e) => Some(WireCommand::ExportJsonl {
            output_path: e.output_path.clone(),
        }),
        v3::Command::ImportJsonl(e) => Some(WireCommand::ImportJsonl {
            input_path: e.input_path.clone(),
        }),
        v3::Command::SwitchSession(e) => Some(WireCommand::SwitchSession {
            session_path: e.session_path.clone(),
        }),
        v3::Command::Fork(e) => Some(WireCommand::Fork {
            entry_id: e.entry_id.clone(),
            position: e.position.clone(),
        }),
        v3::Command::GetMessages(_) => Some(WireCommand::GetMessages {}),
        v3::Command::EstimateContext(_) => Some(WireCommand::EstimateContext {}),
        v3::Command::GetCommands(_) => Some(WireCommand::GetCommands {}),
        v3::Command::SessionTreeCmd(e) => Some(WireCommand::SessionTree {
            kind: v3_session_tree_kind_to_xy(&e.kind),
        }),
        v3::Command::TravelSessionTree(e) => Some(WireCommand::TravelSessionTree {
            kind: v3_session_tree_kind_to_xy(&e.kind),
            entry_id: e.entry_id.clone(),
        }),
        v3::Command::AppendEntryLabel(e) => Some(WireCommand::AppendEntryLabel {
            target_id: e.target_id.clone(),
            label: e.label.clone(),
        }),
        v3::Command::ListSessions(_) => Some(WireCommand::ListSessions {}),
        v3::Command::LoadSessionEntries(e) => Some(WireCommand::LoadSessionEntries {
            session_id: e.session_id.clone(),
        }),
        v3::Command::NewSession(_) => Some(WireCommand::NewSession {}),
        v3::Command::GetSessionName(_) => Some(WireCommand::GetSessionName {}),
        v3::Command::SetSessionName(e) => Some(WireCommand::SetSessionName {
            name: e.name.clone(),
        }),
        v3::Command::SetSessionNameFor(e) => Some(WireCommand::SetSessionNameFor {
            session_id: e.session_id.clone(),
            name: e.name.clone(),
        }),
        v3::Command::DeleteSession(e) => Some(WireCommand::DeleteSession {
            session_id: e.session_id.clone(),
        }),
        v3::Command::Reload(_) => Some(WireCommand::Reload {}),
        v3::Command::LoadedResources(_) => Some(WireCommand::LoadedResources {}),
        v3::Command::GetQueueStats(_) => Some(WireCommand::GetQueueStats {}),
        v3::Command::Steer(e) => Some(WireCommand::Steer {
            message: e.message.clone(),
        }),
        v3::Command::FollowUp(e) => Some(WireCommand::FollowUp {
            message: e.message.clone(),
        }),
        v3::Command::ClearQueue(e) => Some(WireCommand::ClearQueue {
            clear_steer: e.clear_steer,
            clear_follow_up: e.clear_follow_up,
        }),
        v3::Command::Subscribe(e) => Some(WireCommand::Subscribe {
            session_id: e.session_id.clone(),
            last_seq: e.last_seq,
        }),
        v3::Command::ApproveTool(e) => Some(WireCommand::ApproveTool {
            call_id: e.call_id.clone(),
            approved: e.approved,
        }),
        v3::Command::AnswerQuestion(e) => Some(WireCommand::AnswerQuestion {
            call_id: e.call_id.clone(),
            answer: e.answer.clone(),
        }),
        v3::Command::Quit(_) => Some(WireCommand::Quit {}),
        // Degraded: schema-ahead union member from a newer peer.
        v3::Command::Unknown(_) => None,
    }
}

// ── dynamic JSON blocks ────────────────────────────────────

/// Dynamic JSON block: `Option<AgentMessage>` → v3 `message_json` string
/// (`None` stays `None`; serialization failure degrades to `None`, mirroring
/// `to_wire_event`).
fn agent_message_to_json_string(message: Option<&AgentMessage>) -> Option<String> {
    message.and_then(|m| serde_json::to_string(m).ok())
}

/// Lenient dynamic JSON block: v3 `_json` string → `Value`. Parse failure
/// falls back to `Value::Null` — MUST NOT panic (spec r1719).
fn json_string_to_value(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or(Value::Null)
}

/// Lenient `message_json` parse: unparsable / absent → `None` (the Option
/// level equivalent of the Null fallback; mirrors `TryFrom<&Event>`).
fn json_string_to_agent_message(json: Option<&str>) -> Option<AgentMessage> {
    json.and_then(|s| serde_json::from_str(s).ok())
}

// ── small enum converters ──────────────────────────────────

fn token_provenance_from_str(provenance: &str) -> TokenProvenance {
    match provenance {
        "Api" => TokenProvenance::Api,
        "RemoteCount" => TokenProvenance::RemoteCount,
        "LocalTokenizer" => TokenProvenance::LocalTokenizer,
        "Heuristic" => TokenProvenance::Heuristic,
        _ => TokenProvenance::Unknown,
    }
}

fn session_tree_kind_to_v3(kind: SessionTreeKind) -> v3::SessionTreeKind {
    match kind {
        SessionTreeKind::MessageHistory => v3::SessionTreeKind::MessageHistory,
        SessionTreeKind::FileBrowser => v3::SessionTreeKind::FileBrowser,
    }
}

fn v3_session_tree_kind_to_xy(kind: &v3::SessionTreeKind) -> SessionTreeKind {
    match kind {
        v3::SessionTreeKind::MessageHistory => SessionTreeKind::MessageHistory,
        v3::SessionTreeKind::FileBrowser => SessionTreeKind::FileBrowser,
    }
}

// ── Todo snapshot converters (field-by-field) ──────────────

/// xy → v3 goes through the public `TodoList.items` field (`TodoItem` itself
/// is not exported outside `cfg(test)`).
fn todo_list_to_v3(list: &TodoList) -> v3::TodoList {
    v3::TodoList {
        items: list
            .items
            .iter()
            .map(|item| v3::TodoItem {
                id: item.id.clone(),
                content: item.content.clone(),
                status: todo_status_to_v3(item.status),
            })
            .collect(),
    }
}

fn todo_status_to_v3(status: TodoStatus) -> v3::TodoStatus {
    match status {
        TodoStatus::Pending => v3::TodoStatus::Pending,
        TodoStatus::InProgress => v3::TodoStatus::InProgress,
        TodoStatus::Completed => v3::TodoStatus::Completed,
    }
}

/// v3 → xy goes through the public loader (`from_data_value`); the JSON shape
/// is built from closed enums on both sides, so the load cannot fail.
fn v3_todo_list_to_xy(list: &v3::TodoList) -> TodoList {
    let items: Vec<serde_json::Value> = list
        .items
        .iter()
        .map(|item| {
            serde_json::json!({
                "id": item.id,
                "content": item.content,
                "status": v3_todo_status_to_xy(&item.status),
            })
        })
        .collect();
    TodoList::from_data_value(&serde_json::json!({ "items": items }))
        .expect("v3 todo item shape maps onto the closed TodoItem vocabulary")
}

fn v3_todo_status_to_xy(status: &v3::TodoStatus) -> TodoStatus {
    match status {
        v3::TodoStatus::Pending => TodoStatus::Pending,
        v3::TodoStatus::InProgress => TodoStatus::InProgress,
        v3::TodoStatus::Completed => TodoStatus::Completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test-only re-export (production reaches items via `TodoList.items`).
    use crate::protocol::session::TodoItem;

    #[test]
    fn text_delta_roundtrips_through_v3() {
        let domain = XyEvent::TextDelta("chunk".into());
        let v3ev = xy_event_to_v3(&domain).expect("TextDelta is v3-visible");
        let v3::Event::TextDelta(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        assert_eq!(e.text, "chunk");
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        assert!(matches!(back, XyEvent::TextDelta(ref text) if text == "chunk"));
    }

    #[test]
    fn tool_start_roundtrips_args_json_through_v3() {
        // spec r1906: dynamic tool args cross as a JSON string, byte-faithful.
        let domain = XyEvent::ToolExecutionStart {
            id: "read-1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        };
        let v3ev = xy_event_to_v3(&domain).expect("ToolStart is v3-visible");
        let v3::Event::ToolStart(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        assert_eq!(e.args_json, r#"{"path":"README.md"}"#);
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        assert!(matches!(back, XyEvent::ToolExecutionStart { ref args, .. }
                if *args == serde_json::json!({"path": "README.md"})));
    }

    #[test]
    fn compaction_end_all_fields_roundtrip_through_v3() {
        let domain = XyEvent::CompactionEnd {
            result: Some("ok".into()),
            aborted: false,
            reason: "threshold".into(),
            will_retry: true,
            error_message: Some("boom".into()),
            summary: Some("prior work".into()),
            tokens_before: Some(101_080),
            tokens_after: Some(18_240),
            notice: Some("still large".into()),
        };
        let v3ev = xy_event_to_v3(&domain).expect("CompactionEnd is v3-visible");
        let v3::Event::CompactionEnd(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        assert_eq!(e.result.as_deref(), Some("ok"));
        assert!(!e.aborted);
        assert_eq!(e.reason, "threshold");
        assert!(e.will_retry);
        assert_eq!(e.error_message.as_deref(), Some("boom"));
        assert_eq!(e.summary.as_deref(), Some("prior work"));
        assert_eq!(e.tokens_before, Some(101_080));
        assert_eq!(e.tokens_after, Some(18_240));
        assert_eq!(e.notice.as_deref(), Some("still large"));

        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        let XyEvent::CompactionEnd {
            result,
            aborted,
            reason,
            will_retry,
            error_message,
            summary,
            tokens_before,
            tokens_after,
            notice,
        } = back
        else {
            panic!("unexpected event");
        };
        assert_eq!(result.as_deref(), Some("ok"));
        assert!(!aborted);
        assert_eq!(reason, "threshold");
        assert!(will_retry);
        assert_eq!(error_message.as_deref(), Some("boom"));
        assert_eq!(summary.as_deref(), Some("prior work"));
        assert_eq!(tokens_before, Some(101_080));
        assert_eq!(tokens_after, Some(18_240));
        assert_eq!(notice.as_deref(), Some("still large"));
    }

    #[test]
    fn todo_updated_roundtrips_through_v3() {
        // atd13 / pa-todo1: full TodoList snapshot crosses v3 field-by-field;
        // empty list (cleared) stays Some(empty).
        let domain = XyEvent::TodoUpdated {
            list: TodoList::new(vec![TodoItem {
                id: "todo-1".into(),
                content: "check env".into(),
                status: TodoStatus::InProgress,
            }]),
        };
        let v3ev = xy_event_to_v3(&domain).expect("TodoUpdated is v3-visible");
        let v3::Event::TodoListSnapshot(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        let list = e.list.as_ref().expect("list is present");
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, "todo-1");
        assert_eq!(list.items[0].status, v3::TodoStatus::InProgress);
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        assert!(matches!(back, XyEvent::TodoUpdated { ref list }
                if list.items.len() == 1 && list.items[0].status == TodoStatus::InProgress));

        let cleared = XyEvent::TodoUpdated {
            list: TodoList::default(),
        };
        let v3ev = xy_event_to_v3(&cleared).expect("empty list still v3-visible");
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        assert!(matches!(back, XyEvent::TodoUpdated { ref list } if list.is_empty()));
    }

    #[test]
    fn message_start_message_json_roundtrips_through_v3() {
        // ati12: user MessageStart carries the full message as message_json.
        let domain = XyEvent::MessageStart {
            role: "user".into(),
            message: Some(AgentMessage::user("steer text")),
        };
        let v3ev = xy_event_to_v3(&domain).expect("MessageStart is v3-visible");
        let v3::Event::MessageStart(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        assert_eq!(e.role, "user");
        let message_json = e.message_json.as_deref().expect("message_json present");
        assert!(message_json.contains("steer text"));
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        match back {
            XyEvent::MessageStart { role, message } => {
                assert_eq!(role, "user");
                assert_eq!(message.expect("message survives").text(), "steer text");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let bare = XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        };
        let v3ev = xy_event_to_v3(&bare).expect("bare MessageStart is v3-visible");
        let v3::Event::MessageStart(e) = &v3ev else {
            panic!("unexpected v3 event: {v3ev:?}")
        };
        assert_eq!(e.message_json, None);
        let back = v3_event_to_xy(&v3ev).expect("roundtrip");
        assert!(matches!(back, XyEvent::MessageStart { ref message, .. } if message.is_none()));
    }

    #[test]
    fn degraded_xy_events_do_not_cross_v3() {
        // Same degraded set as `to_wire_event`: process-local or REST-covered.
        let degraded = vec![
            XyEvent::AgentStart {
                session_id: "s1".into(),
                model: "m1".into(),
            },
            XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 500,
            },
            XyEvent::AutoRetryEnd {
                success: true,
                attempt: 1,
            },
            XyEvent::SessionInfoChanged {
                key: "name".into(),
                value: serde_json::json!("renamed"),
            },
            XyEvent::ThinkingLevelChanged {
                level: "high".into(),
            },
        ];
        assert_eq!(degraded.len(), 5, "degraded set drift");
        for ev in &degraded {
            assert!(
                xy_event_to_v3(ev).is_none(),
                "{} must not cross v3",
                ev.description()
            );
        }
    }

    #[test]
    fn bad_dynamic_json_degrades_without_panic() {
        // spec r1719 spirit: unparsable dynamic blocks degrade per-field
        // (message → None, args → Null); the event itself still converts.
        let v3ev = v3::Event::MessageStart(v3::MessageStart {
            role: "user".into(),
            message_json: Some("{not json".into()),
        });
        let back = v3_event_to_xy(&v3ev).expect("event itself still converts");
        assert!(
            matches!(back, XyEvent::MessageStart { ref message, .. } if message.is_none()),
            "unparsable message_json falls back to None"
        );

        let v3ev = v3::Event::ToolStart(v3::ToolStart {
            id: "t1".into(),
            name: "bash".into(),
            args_json: "{broken".into(),
        });
        let back = v3_event_to_xy(&v3ev).expect("event itself still converts");
        assert!(
            matches!(back, XyEvent::ToolExecutionStart { ref args, .. } if args.is_null()),
            "unparsable args_json falls back to Value::Null"
        );
    }

    #[test]
    fn v3_unknown_degrades_to_none() {
        assert!(v3_event_to_xy(&v3::Event::Unknown(fory::UnknownCase::new(99, ()))).is_none());
        assert!(v3_to_command(&v3::Command::Unknown(fory::UnknownCase::new(99, ()))).is_none());
    }

    #[test]
    fn v3_todo_snapshot_without_list_degrades_to_none() {
        // Absent list must not fabricate a "cleared" snapshot.
        let v3ev = v3::Event::TodoListSnapshot(v3::TodoListSnapshot { list: None });
        assert!(v3_event_to_xy(&v3ev).is_none());
    }

    /// All 38 wire command variants (declaration order; second tuple item is
    /// the expected v3 member name — `SessionTree` pairs with the fbs-renamed
    /// `SessionTreeCmd`).
    fn all_wire_commands() -> Vec<(WireCommand, &'static str)> {
        vec![
            (
                WireCommand::Prompt {
                    message: "hello".into(),
                },
                "Prompt",
            ),
            (WireCommand::Abort {}, "Abort"),
            (WireCommand::GetState {}, "GetState"),
            (
                WireCommand::SetModel {
                    provider: "zen".into(),
                    model_id: "glm-5".into(),
                },
                "SetModel",
            ),
            (WireCommand::CycleModel {}, "CycleModel"),
            (WireCommand::GetAvailableModels {}, "GetAvailableModels"),
            (
                WireCommand::SetThinkingLevel {
                    level: "high".into(),
                },
                "SetThinkingLevel",
            ),
            (
                WireCommand::Bash {
                    command: "ls".into(),
                    exclude_from_context: true,
                },
                "Bash",
            ),
            (
                WireCommand::Compact {
                    instructions: Some("focus".into()),
                },
                "Compact",
            ),
            (WireCommand::GetSessionStats {}, "GetSessionStats"),
            (
                WireCommand::ExportHtml {
                    output_path: Some("a.html".into()),
                },
                "ExportHtml",
            ),
            (
                WireCommand::ExportJsonl { output_path: None },
                "ExportJsonl",
            ),
            (
                WireCommand::ImportJsonl {
                    input_path: "a.jsonl".into(),
                },
                "ImportJsonl",
            ),
            (
                WireCommand::SwitchSession {
                    session_path: "s.jsonl".into(),
                },
                "SwitchSession",
            ),
            (
                WireCommand::Fork {
                    entry_id: "e1".into(),
                    position: Some("before".into()),
                },
                "Fork",
            ),
            (WireCommand::GetMessages {}, "GetMessages"),
            (WireCommand::EstimateContext {}, "EstimateContext"),
            (WireCommand::GetCommands {}, "GetCommands"),
            (
                WireCommand::SessionTree {
                    kind: SessionTreeKind::MessageHistory,
                },
                "SessionTreeCmd",
            ),
            (
                WireCommand::TravelSessionTree {
                    kind: SessionTreeKind::FileBrowser,
                    entry_id: "e1".into(),
                },
                "TravelSessionTree",
            ),
            (
                WireCommand::AppendEntryLabel {
                    target_id: "e1".into(),
                    label: Some("L".into()),
                },
                "AppendEntryLabel",
            ),
            (WireCommand::ListSessions {}, "ListSessions"),
            (
                WireCommand::LoadSessionEntries {
                    session_id: "s1".into(),
                },
                "LoadSessionEntries",
            ),
            (WireCommand::NewSession {}, "NewSession"),
            (WireCommand::GetSessionName {}, "GetSessionName"),
            (
                WireCommand::SetSessionName { name: "n".into() },
                "SetSessionName",
            ),
            (
                WireCommand::SetSessionNameFor {
                    session_id: "s1".into(),
                    name: "n".into(),
                },
                "SetSessionNameFor",
            ),
            (
                WireCommand::DeleteSession {
                    session_id: "s1".into(),
                },
                "DeleteSession",
            ),
            (WireCommand::Reload {}, "Reload"),
            (WireCommand::LoadedResources {}, "LoadedResources"),
            (WireCommand::GetQueueStats {}, "GetQueueStats"),
            (
                WireCommand::Steer {
                    message: "steer".into(),
                },
                "Steer",
            ),
            (
                WireCommand::FollowUp {
                    message: "follow".into(),
                },
                "FollowUp",
            ),
            (
                WireCommand::ClearQueue {
                    clear_steer: true,
                    clear_follow_up: false,
                },
                "ClearQueue",
            ),
            (
                WireCommand::Subscribe {
                    session_id: "s1".into(),
                    last_seq: 42,
                },
                "Subscribe",
            ),
            (
                WireCommand::ApproveTool {
                    call_id: "c1".into(),
                    approved: true,
                },
                "ApproveTool",
            ),
            (
                WireCommand::AnswerQuestion {
                    call_id: "c1".into(),
                    answer: "yes".into(),
                },
                "AnswerQuestion",
            ),
            (WireCommand::Quit {}, "Quit"),
        ]
    }

    /// Test-only v3 member name (catches symmetric member mix-ups such as
    /// Steer ↔ FollowUp that a pure roundtrip cannot see).
    fn v3_kind(cmd: &v3::Command) -> &'static str {
        match cmd {
            v3::Command::Unknown(_) => "Unknown",
            v3::Command::Prompt(_) => "Prompt",
            v3::Command::Abort(_) => "Abort",
            v3::Command::GetState(_) => "GetState",
            v3::Command::SetModel(_) => "SetModel",
            v3::Command::CycleModel(_) => "CycleModel",
            v3::Command::GetAvailableModels(_) => "GetAvailableModels",
            v3::Command::SetThinkingLevel(_) => "SetThinkingLevel",
            v3::Command::Bash(_) => "Bash",
            v3::Command::Compact(_) => "Compact",
            v3::Command::GetSessionStats(_) => "GetSessionStats",
            v3::Command::ExportHtml(_) => "ExportHtml",
            v3::Command::ExportJsonl(_) => "ExportJsonl",
            v3::Command::ImportJsonl(_) => "ImportJsonl",
            v3::Command::SwitchSession(_) => "SwitchSession",
            v3::Command::Fork(_) => "Fork",
            v3::Command::GetMessages(_) => "GetMessages",
            v3::Command::EstimateContext(_) => "EstimateContext",
            v3::Command::GetCommands(_) => "GetCommands",
            v3::Command::SessionTreeCmd(_) => "SessionTreeCmd",
            v3::Command::TravelSessionTree(_) => "TravelSessionTree",
            v3::Command::AppendEntryLabel(_) => "AppendEntryLabel",
            v3::Command::ListSessions(_) => "ListSessions",
            v3::Command::LoadSessionEntries(_) => "LoadSessionEntries",
            v3::Command::NewSession(_) => "NewSession",
            v3::Command::GetSessionName(_) => "GetSessionName",
            v3::Command::SetSessionName(_) => "SetSessionName",
            v3::Command::SetSessionNameFor(_) => "SetSessionNameFor",
            v3::Command::DeleteSession(_) => "DeleteSession",
            v3::Command::Reload(_) => "Reload",
            v3::Command::LoadedResources(_) => "LoadedResources",
            v3::Command::GetQueueStats(_) => "GetQueueStats",
            v3::Command::Steer(_) => "Steer",
            v3::Command::FollowUp(_) => "FollowUp",
            v3::Command::ClearQueue(_) => "ClearQueue",
            v3::Command::Subscribe(_) => "Subscribe",
            v3::Command::ApproveTool(_) => "ApproveTool",
            v3::Command::AnswerQuestion(_) => "AnswerQuestion",
            v3::Command::Quit(_) => "Quit",
        }
    }

    #[test]
    fn all_command_variants_roundtrip_through_v3() {
        use strum::VariantNames;
        assert_eq!(
            WireCommand::VARIANTS.len(),
            38,
            "wire Command variant count drift"
        );
        let cases = all_wire_commands();
        assert_eq!(cases.len(), 38, "case table drift");
        for (cmd, expected_kind) in &cases {
            let v3cmd = command_to_v3(cmd);
            assert_eq!(v3_kind(&v3cmd), *expected_kind, "member pairing drift");
            let back = v3_to_command(&v3cmd).expect("known member must map back");
            assert_eq!(
                serde_json::to_value(cmd).expect("wire serializes"),
                serde_json::to_value(&back).expect("wire serializes"),
                "{expected_kind} roundtrip payload drift"
            );
        }
    }
}
